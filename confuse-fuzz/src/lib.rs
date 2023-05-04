//! Confuse-Fuzz
//!
//! This library implements a Fuzzer wrapping a [`SimicsProject`] using a [`Client`] to
//! communicate with the `Confuse` SIMICS module.

use anyhow::{bail, ensure, Error, Result};
use confuse_module::{
    client::Client, config::InputConfig, stops::StopReason, traits::ConfuseClient,
};
use confuse_simics_project::SimicsProject;
use crossterm::{
    cursor::Show,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, Clear, ClearType, LeaveAlternateScreen},
};
use ipc_shm::{IpcShm, IpcShmWriter};
use libafl::{
    prelude::{
        tui::{ui::TuiUI, TuiMonitor},
        *,
    },
    schedulers::powersched::PowerSchedule,
    Fuzzer as _,
};
use log::{debug, error, info, Level};
use std::{
    fs::create_dir_all,
    io::stdout,
    path::{Path, PathBuf},
};

fn libafl_err<T>(r: Result<T>) -> Result<T, libafl::Error> {
    r.map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))
}

pub fn fuzz<S: AsRef<str>, P: AsRef<Path>>(
    // 1,2-4,6: clients run in cores 1,2,3,4,6
    // all: one client runs on each available core
    cores: S,
    input_corpus: P,
    output_corpus: P,
    input_config: InputConfig,
    simics_project: SimicsProject,
    simics_log_level: Level,
    cycles: Option<u64>,
) -> Result<()> {
    if !output_corpus.as_ref().is_dir() {
        create_dir_all(&output_corpus)?;
    }

    let ui = TuiUI::new("Confuse Fuzzer".to_string(), true);
    let monitor = TuiMonitor::new(ui);

    let output_corpus = output_corpus.as_ref().to_path_buf();

    let mut fuzz_one = |state: Option<_>, mut mgr, core_id| -> Result<(), libafl::Error> {
        info!("Initializing fuzzer on core {:?}", core_id);

        let config = input_config.clone();

        let mut project = libafl_err(simics_project.try_clone())?.with_loglevel(simics_log_level);
        project.persist();

        info!("Initializing fuzzer client");

        let mut client = libafl_err(Client::try_new(project))?;

        let mut output_config = libafl_err(client.initialize(config))?;

        let mut shm = libafl_err(output_config.coverage())?;

        let mut shm_writer = libafl_err(shm.writer())?;

        libafl_err(client.reset())?;

        info!("SIMICS ready to fuzz");

        let mut coverage_observer = unsafe {
            HitcountsMapObserver::new(StdMapObserver::from_mut_ptr(
                "map",
                shm_writer.as_mut_ptr(),
                shm_writer.len(),
            ))
        };

        let mut coverage_feedback = MaxMapFeedback::new(&coverage_observer);

        let mut objective = feedback_or_fast!(CrashFeedback::new(), TimeoutFeedback::new());

        let mut state = state.unwrap_or_else(|| {
            StdState::new(
                StdRand::with_seed(current_nanos()),
                CachedOnDiskCorpus::new(output_corpus.clone().join("corpus"), 4096)
                    .expect("Couldn't create corpus directory"),
                OnDiskCorpus::new(output_corpus.clone().join("solutions"))
                    .expect("Could not create solutions directory"),
                &mut coverage_feedback,
                &mut objective,
            )
            .expect("Could not create state")
        });

        let scheduler =
            PowerQueueScheduler::new(&mut state, &mut coverage_observer, PowerSchedule::EXPLORE);
        let mut fuzzer = StdFuzzer::new(scheduler, coverage_feedback, objective);

        let mut harness = |input: &BytesInput| {
            let target = input.target_bytes();
            let buf = target.as_slice();
            let run_input = buf.to_vec();
            let mut exit_kind = ExitKind::Ok;
            // We expect we'll get a simics ready message:

            info!("Running with input '{:?}'", run_input);

            info!("Sending run signal");

            match client.run(run_input) {
                Ok(reason) => match reason {
                    StopReason::Crash(fault) => {
                        error!("Target crashed with fault {:?}, yeehaw!", fault);
                        exit_kind = ExitKind::Crash;
                    }
                    StopReason::SimulationExit(_) => {
                        info!("Target stopped normally ;_;");

                        exit_kind = ExitKind::Ok;
                    }
                    StopReason::TimeOut => {
                        error!("Target timed out, yeehaw(???)");
                        exit_kind = ExitKind::Timeout;
                    }
                    StopReason::Magic(_) => {
                        exit_kind = ExitKind::Ok;
                    }
                    StopReason::Error((e, _p)) => {
                        error!("An error occurred during execution: {:?}", e);
                        exit_kind = ExitKind::Ok;
                    }
                },
                Err(e) => {
                    error!("Error running SIMICS: {}", e);
                }
            }

            // We'd read the state of the vm here, including caught exceptions and branch trace
            // Now we send the reset signal
            debug!("Sending reset signal");

            if let Err(e) = client.reset() {
                error!("Error resetting SIMICS: {}", e);
            }

            debug!("Harness done");

            exit_kind
        };

        let mut executor = InProcessExecutor::new(
            &mut harness,
            tuple_list!(coverage_observer),
            &mut fuzzer,
            &mut state,
            &mut mgr,
        )?;

        if state.must_load_initial_inputs() {
            info!("Loading initial inputs");
            state.load_initial_inputs(
                &mut fuzzer,
                &mut executor,
                &mut mgr,
                &[input_corpus.as_ref().to_path_buf()],
            )?;
            info!("Loaded {} inputs", state.corpus().count());
        }

        let mutator = StdScheduledMutator::new(havoc_mutations());
        let mut stages = tuple_list!(StdMutationalStage::new(mutator));

        info!("Starting fuzz loop");

        match cycles {
            Some(cycles) => {
                _ = fuzzer.fuzz_loop_for(
                    &mut stages,
                    &mut executor,
                    &mut state,
                    &mut mgr,
                    cycles,
                )?;
                libafl_err(client.exit())?;
            }
            None => {
                _ = fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
                // This is probably unreachable?
                libafl_err(client.exit())?;
            }
        }

        Ok(())
    };

    let cores = Cores::from_cmdline(cores.as_ref())?;

    match Launcher::builder()
        .shmem_provider(StdShMemProvider::new()?)
        .monitor(monitor)
        .configuration(EventConfig::from_build_id())
        .run_client(&mut fuzz_one)
        .cores(&cores)
        .build()
        .launch()
    {
        Ok(()) => {}
        Err(libafl::Error::ShuttingDown) => {}
        Err(e) => bail!("Failed to run launcher: {}", e),
    }

    disable_raw_mode()?;
    execute!(
        stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show,
        Clear(ClearType::Purge)
    )?;

    Ok(())
}
