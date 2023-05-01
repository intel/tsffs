//! Confuse-Fuzz
//!
//! This library contains abstractions over a fuzzing campaign using the SIMICS platform

use anyhow::Result;
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
    prelude::{tui::TuiMonitor, *},
    Fuzzer as _,
};
use log::{debug, error, info, Level};
use std::{io::stdout, path::PathBuf};

/// Customizable fuzzer for SIMICS
pub struct Fuzzer {
    /// The client for the SIMICS module which also owns the SIMICS project the fuzzer is started
    /// with
    client: Client,
    /// The shared memory handle for the coverage map
    _shm: IpcShm,
    /// A r/w handle to the shared memory
    shm_writer: IpcShmWriter,
    /// The path on disk to the input corpus
    input_corpus: PathBuf,
}

impl Fuzzer {
    /// Create a new fuzzer and set up the simulator it will fuzz against.
    ///
    /// # Arguments
    ///
    /// * `input_corpus` - The path to a directory on disk where an initial fuzzing corpus is
    ///                    located
    /// * `config` - The initial configuration for the fuzzer. This configuration can be changed
    ///              during SIMICS initialization and before the fuzzer starts running by using
    ///              the Python API of the CONFUSE Simics module
    /// * `simics_project` - A SIMICS project that is configured with required packages and files
    ///                      and is ready to start with the CONFUSE module added.
    /// * `simics_log_level` - The log level to use for SIMICS
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
                    StopReason::Error((e, p)) => {
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

    /// Run the fuzzer infinitely, until it is killed manually by the user
    pub fn run(&mut self) -> Result<()> {
        self.run_inner(None)
    }

    /// Run the fuzzer for a certain number of fuzzing cycles. Note that a fuzzing *cycle* is
    /// different from an *execution*
    ///
    /// # Arguments
    ///
    /// * `cycles` - The number of cycles to run for. Cycles do not map directly to iterations, but
    ///              a good rule of thumb is ~1k iterations per cycle.
    pub fn run_cycles(&mut self, cycles: u64) -> Result<()> {
        self.run_inner(Some(cycles))
    }

    /// Stop the fuzzer
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
