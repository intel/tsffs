use crate::{
    args::command::Command,
    modules::confuse::{ConfuseModuleInterface, CONFUSE_MODULE_CRATE_NAME},
};
use anyhow::{anyhow, Error, Result};
use confuse_module::{
    client::Client,
    config::{InputConfig, InputConfigBuilder, TraceMode},
    faults::{x86_64::X86_64Fault, Fault},
    messages::{client::ClientMessage, module::ModuleMessage},
    module::Confuse,
    stops::StopReason,
    traits::ConfuseClient,
};
use derive_builder::Builder;
use libafl::{
    bolts::core_affinity::Cores,
    feedback_and_fast, feedback_not, feedback_or_fast,
    prelude::{
        current_nanos, havoc_mutations,
        ondisk::OnDiskMetadataFormat,
        tui::{ui::TuiUI, TuiMonitor},
        tuple_list, AflMapFeedback, AsMutSlice, AsSlice, BytesInput, CachedOnDiskCorpus, Corpus,
        CrashFeedback, EventConfig, ExitKind, HasTargetBytes, HitcountsMapObserver,
        InProcessExecutor, Launcher, MultiMonitor, OnDiskCorpus, OwnedMutSlice, RandBytesGenerator,
        ShMemProvider, StdMapObserver, StdRand, StdScheduledMutator, StdShMemProvider,
        TimeFeedback, TimeObserver, TimeoutExecutor,
    },
    schedulers::{
        powersched::PowerSchedule, IndexesLenTimeMinimizerScheduler, PowerQueueScheduler,
    },
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
    ErrorBacktrace, Fuzzer, StdFuzzer,
};
use simics::{
    api::{
        alloc_attr_list, create_object, get_class, get_interface, get_object, load_module,
        make_attr_data_adopt, DeprecationLevel, GuiMode, InitArg, InitArgs, Interface,
    },
    project::Project,
    simics::Simics,
};
use std::{
    mem::size_of,
    net::TcpListener,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{spawn, JoinHandle},
    time::Duration,
};
use tracing::{debug, error, info, metadata::LevelFilter};

mod feedbacks;
mod monitors;
mod observers;

#[derive(Builder)]
#[builder(build_fn(validate = "Self::validate", error = "Error"))]
pub struct SimicsFuzzer {
    project: Project,
    #[builder(setter(into), default)]
    input: Option<PathBuf>,
    #[builder(setter(custom), default)]
    corpus: PathBuf,
    #[builder(setter(custom), default)]
    solutions: PathBuf,
    tui: bool,
    #[builder(default)]
    _shrink: bool,
    #[builder(default)]
    _dedup: bool,
    #[builder(default)]
    _grimoire: bool,
    timeout: f64,
    executor_timeout: u64,
    cores: Cores,
    command: Vec<Command>,
    log_level: LevelFilter,
    trace_mode: TraceMode,
}

impl SimicsFuzzerBuilder {
    pub fn corpus(&mut self, value: Option<PathBuf>) -> &mut Self {
        self.corpus = Some(value.unwrap_or(PathBuf::from(SimicsFuzzer::DEFAULT_CORPUS_DIRECTORY)));
        self
    }

    pub fn solutions(&mut self, value: Option<PathBuf>) -> &mut Self {
        self.solutions =
            Some(value.unwrap_or(PathBuf::from(SimicsFuzzer::DEFAULT_SOLUTIONS_DIRECTORY)));
        self
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl SimicsFuzzer {
    pub const NAME: &str = "Confuse Fuzzer";
    pub const MAP_SIZE: usize = 128 * 1024;
    pub const CACHE_LEN: usize = 4096;
    pub const DEFAULT_CORPUS_DIRECTORY: &str = "corpus";
    pub const DEFAULT_SOLUTIONS_DIRECTORY: &str = "solutions";

    pub fn simics(
        &self,
        tx: Sender<ModuleMessage>,
        rx: Receiver<ClientMessage>,
    ) -> JoinHandle<Result<()>> {
        let path = self.project.path.path.clone();

        info!(
            "Starting SIMICS project at {}",
            self.project.path.path.display()
        );

        let command = self.command.clone();

        spawn(move || -> Result<()> {
            // TODO: Take these args from CLI, we should let users override anything including GUI
            // mode if they really want to
            let simics_args = InitArgs::default()
                .arg(InitArg::batch_mode()?)
                .arg(InitArg::deprecation_level(DeprecationLevel::NoWarnings)?)
                .arg(InitArg::gui_mode(GuiMode::None)?)
                // TODO: Maybe disable this if we can output logs through tracing?
                .arg(InitArg::no_windows()?)
                .arg(InitArg::project(path.to_string_lossy().to_string())?)
                // TODO: maybe disable these for verbosity reasons
                .arg(InitArg::log_enable()?)
                .arg(InitArg::log_file("/tmp/simics.log")?);

            Simics::try_init(simics_args)?;

            // Doesn't matter if the script has a `@SIM_load_module` in it, as long as it isn't
            // asserting that it returns not-None
            load_module(CONFUSE_MODULE_CRATE_NAME)?;

            info!("Loaded SIMICS module");

            let confuse = create_object(
                get_class(CONFUSE_MODULE_CRATE_NAME)?,
                CONFUSE_MODULE_CRATE_NAME,
                alloc_attr_list(0),
            )?;

            info!("Got confuse at {:#x}", confuse as usize);

            let confuse_interface = get_interface::<ConfuseModuleInterface>(
                confuse,
                Interface::Other(CONFUSE_MODULE_CRATE_NAME.to_string()),
            )?;

            info!("Got SIMICS object: {:#x}", confuse as usize);
            info!("Got confuse interface at {:#x}", confuse_interface as usize);

            let tx = Box::new(make_attr_data_adopt(tx)?);
            let rx = Box::new(make_attr_data_adopt(rx)?);
            let tx = Box::into_raw(tx);
            let rx = Box::into_raw(rx);

            info!("Tx: {:#x} Rx: {:#x}", tx as usize, rx as usize);

            info!("Setting up channels");

            (unsafe { *confuse_interface }.add_channels)(confuse, tx, rx);

            info!("Set channel for object");

            command.iter().try_for_each(|c| match c {
                Command::Command { command } => Simics::command(command),
                Command::Python { file } => Simics::python(file.canonicalize(&path)?),
                Command::Config { config } => Simics::config(config.canonicalize(&path)?),
            })?;

            // If the command we just ran includes `@SIM_main_loop`, the below code will be unreachable, but that is OK. The callbacks will eventually call `@SIM_quit` for us
            // and this will never be called. If the command doesn't include, `@SIM_main_loop`, then we need to enter it now.

            Simics::run();
        })
    }

    pub fn launch(&mut self) -> Result<()> {
        if self.tui {
            self.log_level = LevelFilter::ERROR;
        }

        let shmem_provider = StdShMemProvider::new()?;

        let broker_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

        let mut run_client = |state: Option<_>, mut mgr, cpu_id| -> Result<(), libafl::Error> {
            debug!("Running on CPU {:?}", cpu_id);

            let mut coverage_map = OwnedMutSlice::from(vec![0; SimicsFuzzer::MAP_SIZE]);

            let config = InputConfigBuilder::default()
                .coverage_map((
                    coverage_map.as_mut_slice().as_mut_ptr(),
                    coverage_map.as_slice().len(),
                ))
                .fault(Fault::X86_64(X86_64Fault::Page))
                .fault(Fault::X86_64(X86_64Fault::InvalidOpcode))
                .trace_mode(self.trace_mode)
                .timeout(self.timeout)
                .log_level(self.log_level)
                .build()
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            let (tx, orx) = channel::<ClientMessage>();
            let (otx, rx) = channel::<ModuleMessage>();

            let simics = self.simics(otx, orx);
            let mut client = Client::new(tx, rx);

            let _output_config = client
                .initialize(config)
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            client
                .reset()
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            let counters_observer =
                HitcountsMapObserver::new(StdMapObserver::from_mut_slice("coverage", coverage_map));

            let counters_feedback = AflMapFeedback::new(&counters_observer);

            let time_observer = TimeObserver::new("time");

            let calibration = CalibrationStage::new(&counters_feedback);

            let mut feedback =
                feedback_and_fast!(feedback_not!(CrashFeedback::new()), counters_feedback);

            let mut objective = feedback_or_fast!(
                feedback_and_fast!(CrashFeedback::new()),
                TimeFeedback::new("time")
            );

            let solutions =
                OnDiskCorpus::with_meta_format(&self.solutions, OnDiskMetadataFormat::JsonPretty)?;

            let corpus = CachedOnDiskCorpus::with_meta_format(
                &self.corpus,
                SimicsFuzzer::CACHE_LEN,
                Some(OnDiskMetadataFormat::JsonPretty),
            )?;

            let mut state = state.unwrap_or_else(|| {
                StdState::new(
                    StdRand::with_seed(current_nanos()),
                    corpus,
                    solutions,
                    &mut feedback,
                    &mut objective,
                )
                .expect("Couldn't initialize state")
            });

            let std_mutator = StdScheduledMutator::new(havoc_mutations());
            let std_power = StdPowerMutationalStage::new(std_mutator);
            let scheduler =
                PowerQueueScheduler::new(&mut state, &counters_observer, PowerSchedule::FAST);
            let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

            let mut harness = |input: &BytesInput| {
                let target = input.target_bytes();
                let buf = target.as_slice();
                let run_input = buf.to_vec();
                let mut exit_kind = ExitKind::Ok;

                info!("Running with input '{:?}'", run_input);

                info!("Sending run signal");

                match client.run(run_input) {
                    Ok(reason) => match reason {
                        StopReason::Magic((_magic, _p)) => {
                            exit_kind = ExitKind::Ok;
                        }
                        StopReason::SimulationExit(_) => {
                            exit_kind = ExitKind::Ok;
                        }
                        StopReason::Crash((fault, _p)) => {
                            info!("Target crashed with fault {:?}", fault);
                            exit_kind = ExitKind::Crash;
                        }
                        StopReason::TimeOut => {
                            info!("Target timed out");
                            exit_kind = ExitKind::Timeout;
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

                debug!("Sending reset signal");

                if let Err(e) = client.reset() {
                    error!("Error resetting SIMICS: {}", e);
                }

                debug!("Harness done");

                exit_kind
            };

            let mut executor = TimeoutExecutor::new(
                InProcessExecutor::new(
                    &mut harness,
                    tuple_list!(counters_observer, time_observer),
                    &mut fuzzer,
                    &mut state,
                    &mut mgr,
                )?,
                // The executor's timeout can be quite long
                Duration::from_secs(self.executor_timeout),
            );

            if state.must_load_initial_inputs() {
                if let Some(input) = self.input.as_ref() {
                    state.load_initial_inputs(
                        &mut fuzzer,
                        &mut executor,
                        &mut mgr,
                        &[input.clone()],
                    )?;
                }
                if state.corpus().count() < 1 {
                    let mut generator = RandBytesGenerator::new(64);
                    state.generate_initial_inputs(
                        &mut fuzzer,
                        &mut executor,
                        &mut generator,
                        &mut mgr,
                        1 << 10,
                    )?;
                }
                info!("Imported {} inputs from disk", state.corpus().count());
            }

            let mut stages = tuple_list!(calibration, std_power);

            fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;

            client
                .exit()
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            simics
                .join()
                .map_err(|e| libafl::Error::Unknown(format!("{:?}", e), ErrorBacktrace::new()))?
                .map_err(|e| libafl::Error::Unknown(format!("{:?}", e), ErrorBacktrace::new()))?;

            Ok(())
        };

        // TODO: Deduplicate this nastiness
        if self.tui {
            // Set log level to error if in TUI mode
            let monitor = TuiMonitor::new(TuiUI::new(Self::NAME.to_owned(), true));
            match Launcher::builder()
                .shmem_provider(shmem_provider)
                .configuration(EventConfig::from_name(Self::NAME))
                .monitor(monitor)
                .run_client(&mut run_client)
                .cores(&self.cores)
                .broker_port(broker_port)
                .stdout_file(Some("/tmp/test.txt"))
                .build()
                .launch()
            {
                Ok(()) => (),
                Err(libafl::Error::ShuttingDown) => {
                    info!("Fuzzer stopped by user. shutting down.");
                }
                res @ Err(_) => return res.map_err(|e| anyhow!("Failed to run launcher: {}", e)),
            }
        } else {
            let monitor = MultiMonitor::new(|s| {
                info!("{}", s);
            });
            match Launcher::builder()
                .shmem_provider(shmem_provider)
                .configuration(EventConfig::from_name(Self::NAME))
                .monitor(monitor)
                .run_client(&mut run_client)
                .cores(&self.cores)
                .broker_port(broker_port)
                .build()
                .launch()
            {
                Ok(()) => (),
                Err(libafl::Error::ShuttingDown) => {
                    info!("Fuzzer stopped by user. shutting down.");
                }
                res @ Err(_) => return res.map_err(|e| anyhow!("Failed to run launcher: {}", e)),
            }
        };

        self.project.cleanup();

        Ok(())
    }
}
