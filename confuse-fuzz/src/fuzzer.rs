use std::{
    ffi::OsStr,
    io::{stdout, BufRead, BufReader},
    path::PathBuf,
    process::{Child, Stdio},
    thread::{spawn, JoinHandle},
};

use anyhow::{bail, Result};
use confuse_module::{
    client::Client,
    module::{
        config::InputConfig, controller::messages::module::ModuleMessage, stop_reason::StopReason,
    },
};
use confuse_simics_project::SimicsProject;
use crossterm::{
    cursor::Show,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, Clear, ClearType, LeaveAlternateScreen},
};
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use ipc_shm::{IpcShm, IpcShmWriter};
use libafl::{
    prelude::{tui::TuiMonitor, *},
    Fuzzer as _,
};
use log::{debug, error, info, warn, Level};

/// Customizable fuzzer for SIMICS
pub struct Fuzzer {
    client: Client,
    _shm: IpcShm,
    shm_writer: IpcShmWriter,
    input_corpus: PathBuf,
}

impl Fuzzer {
    pub fn try_new(
        input_corpus: PathBuf,
        config: InputConfig,
        mut simics_project: SimicsProject,
        simics_log_level: Level,
    ) -> Result<Self> {
        info!("Initializing fuzzer");

        simics_project = simics_project.with_loglevel(simics_log_level);

        simics_project.persist();

        let mut client = Client::try_new(simics_project)?;

        info!("Initializing fuzzer client");

        let mut output_config = client.initialize(config)?;

        let mut shm = output_config.coverage()?;

        let shm_writer = shm.writer()?;

        client.reset()?;

        Ok(Self {
            client,
            _shm: shm,
            shm_writer,
            input_corpus,
        })
    }

    fn run_inner(&mut self, cycles: Option<u64>) -> Result<()> {
        let coverage_observer = unsafe {
            HitcountsMapObserver::new(StdMapObserver::from_mut_ptr(
                "map",
                self.shm_writer.as_mut_ptr(),
                self.shm_writer.len(),
            ))
        };

        let mut coverage_feedback = MaxMapFeedback::new(&coverage_observer);

        let mut objective = CrashFeedback::new();

        let input_corpus = InMemoryCorpus::new();

        let mut state = StdState::new(
            StdRand::with_seed(current_nanos()),
            input_corpus,
            OnDiskCorpus::new(self.client.project.base_path.join("crashes"))?,
            &mut coverage_feedback,
            &mut objective,
        )?;

        let mon = TuiMonitor::new("Confuse Fuzzer".to_string(), true);
        let mut mgr = SimpleEventManager::new(mon);
        let scheduler = QueueScheduler::new();
        let mut fuzzer = StdFuzzer::new(scheduler, coverage_feedback, objective);
        let mut harness = |input: &BytesInput| {
            let target = input.target_bytes();
            let buf = target.as_slice();
            let run_input = buf.to_vec();
            let mut exit_kind = ExitKind::Ok;
            // We expect we'll get a simics ready message:

            info!("Running with input '{:?}'", run_input);

            info!("Sending run signal");

            match self.client.run(run_input) {
                Ok(reason) => match reason {
                    StopReason::Crash(fault) => {
                        error!("Target crashed with fault {:?}, yeehaw!", fault);
                        exit_kind = ExitKind::Crash;
                    }
                    StopReason::SimulationExit => {
                        info!("Target stopped normally ;_;");

                        exit_kind = ExitKind::Ok;
                    }
                    StopReason::TimeOut => {
                        warn!("Target timed out, yeehaw(???)");
                        exit_kind = ExitKind::Timeout;
                    }
                    StopReason::Magic(_) => {
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

            if let Err(e) = self.client.reset() {
                error!("Error resetting SIMICS: {}", e);
            }

            debug!("Harness done");

            exit_kind
        };

        info!("Creating executor");

        let mut executor = InProcessExecutor::new(
            &mut harness,
            tuple_list!(coverage_observer),
            &mut fuzzer,
            &mut state,
            &mut mgr,
        )?;

        if state.corpus().count() < 1 {
            state.load_initial_inputs(
                &mut fuzzer,
                &mut executor,
                &mut mgr,
                &[self.input_corpus.clone()],
            )?;
            info!("Loaded {} initial inputs", state.corpus().count());
        }

        info!("Creating mutator");

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
            }
            None => {
                _ = fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
            }
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.run_inner(None)
    }

    pub fn run_cycles(&mut self, cycles: u64) -> Result<()> {
        self.run_inner(Some(cycles))
    }

    pub fn stop(mut self) -> Result<()> {
        info!("Stopping the fuzzer.");
        // We expect we'll get a simics ready message:
        // TODO: Do we need to figure out how to exit cleanly on the simics side or can we just
        // kill?

        self.client.exit()?;

        // TODO: PR a fix for this to libafl to make this not necessary
        // The TUI Monitor doesn't clean itself up nicely so we do this for now
        disable_raw_mode()?;
        execute!(
            stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show,
            Clear(ClearType::Purge)
        )?;

        info!("Stopped fuzzer.");

        Ok(())
    }
}
