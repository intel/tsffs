use crate::{
    args::{command::Command, Args},
    modules::tsffs::{ModuleInterface, TSFFS_MODULE_CRATE_NAME, TSFFS_WORKSPACE_PATH},
};
use anyhow::{anyhow, bail, Error, Result};
use artifact_dependency::{ArtifactDependencyBuilder, CrateType};
use derive_builder::Builder;
use libafl::{
    bolts::core_affinity::Cores,
    feedback_and_fast, feedback_not, feedback_or_fast,
    prelude::{
        current_nanos, havoc_mutations,
        ondisk::OnDiskMetadataFormat,
        tui::{ui::TuiUI, TuiMonitor},
        tuple_list, AflMapFeedback, AsMutSlice, AsSlice, BytesInput, CachedOnDiskCorpus, Corpus,
        CrashFeedback, EventConfig, EventRestarter, ExitKind, HasTargetBytes, HitcountsMapObserver,
        InProcessExecutor, Launcher, LlmpRestartingEventManager, MultiMonitor, OnDiskCorpus,
        OwnedMutSlice, RandBytesGenerator, ShMemProvider, StdMapObserver, StdRand,
        StdScheduledMutator, StdShMemProvider, TimeFeedback, TimeObserver, TimeoutExecutor,
    },
    schedulers::{powersched::PowerSchedule, PowerQueueScheduler},
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
    ErrorBacktrace, Fuzzer, StdFuzzer,
};
use simics::{
    api::{
        alloc_attr_list, create_object, get_class, get_interface, load_module,
        make_attr_data_adopt, sys::SIMICS_VERSION, DeprecationLevel, GuiMode, InitArg, InitArgs,
        Interface,
    },
    module::ModuleBuilder,
    project::{Project, ProjectBuilder},
    simics::Simics,
};
use std::{
    fs::{read, set_permissions, OpenOptions, Permissions},
    io::stdout,
    net::TcpListener,
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};
use tracing::{debug, error, info, metadata::LevelFilter};
use tracing::{trace, Level};
use tracing_subscriber::{filter::filter_fn, fmt, prelude::*, registry, Layer};
use tsffs_module::{
    client::Client,
    config::{InputConfigBuilder, TraceMode},
    messages::{client::ClientMessage, module::ModuleMessage},
    stops::StopReason,
    traits::ThreadClient,
};

const INITIAL_INPUTS: usize = 16;

#[derive(Builder)]
#[builder(
    pattern = "owned",
    build_fn(validate = "Self::validate", error = "Error")
)]
pub struct SimicsFuzzer {
    project: Project,
    #[builder(setter(into), default)]
    input: Option<PathBuf>,
    #[builder(setter(custom), default)]
    corpus: PathBuf,
    #[builder(setter(custom), default)]
    solutions: PathBuf,
    #[builder(setter(into), default)]
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
    #[builder(default)]
    simics_gui: bool,
    #[builder(setter(into), default)]
    iterations: Option<u64>,
    #[builder(setter(into))]
    tui_stdout_file: PathBuf,
    #[builder(default)]
    repro: Option<PathBuf>,
}

impl SimicsFuzzerBuilder {
    pub fn corpus(mut self, value: Option<PathBuf>) -> Self {
        self.corpus = Some(value.unwrap_or(PathBuf::from(SimicsFuzzer::DEFAULT_CORPUS_DIRECTORY)));
        self
    }

    pub fn solutions(mut self, value: Option<PathBuf>) -> Self {
        self.solutions =
            Some(value.unwrap_or(PathBuf::from(SimicsFuzzer::DEFAULT_SOLUTIONS_DIRECTORY)));
        self
    }

    fn validate(&self) -> Result<()> {
        if self.simics_gui.as_ref().is_some_and(|g| *g)
            && self.cores.as_ref().is_some_and(|c| c.ids.len() > 1)
        {
            bail!("Cannot enable GUI with more than one fuzzer core!");
        }
        Ok(())
    }
}

impl SimicsFuzzer {
    pub const NAME: &str = "Fuzzer";
    pub const MAP_SIZE: usize = 128 * 1024;
    pub const CACHE_LEN: usize = 4096;
    pub const DEFAULT_CORPUS_DIRECTORY: &str = "corpus";
    pub const DEFAULT_SOLUTIONS_DIRECTORY: &str = "solutions";

    pub fn cli_main(args: Args) -> Result<()> {
        let reg = registry().with({
            fmt::layer()
                .pretty()
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_writer(stdout)
                .with_filter(args.log_level)
                .with_filter(filter_fn(|metadata| {
                    // LLMP absolutely spams the log when tracing
                    !(metadata.target() == "libafl::bolts::llmp"
                        && matches!(metadata.level(), &Level::TRACE))
                }))
        });
        if let Some(log_file) = &args.log_file {
            let file = Box::new({
                let f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .write(true)
                    .open(log_file)?;
                set_permissions(log_file, Permissions::from_mode(0o700))?;
                f
            });
            reg.with({
                fmt::layer()
                    .compact()
                    .with_writer(Mutex::new(file))
                    .with_filter(args.log_level)
                    .with_filter(filter_fn(|metadata| {
                        // LLMP absolutely spams the log when tracing
                        !(metadata.target() == "libafl::bolts::llmp"
                            && matches!(metadata.level(), &Level::TRACE))
                    }))
            })
            .try_init()
            .map_err(|e| {
                error!("Could not install tracing subscriber: {}", e);
                e
            })
            .ok();
        } else {
            reg.try_init()
                .map_err(|e| {
                    error!("Could not install tracing subscriber: {}", e);
                    e
                })
                .ok();
        }

        trace!("Setting up project with args: {:?}", args);

        let mut builder: ProjectBuilder = if let Some(project_path) = args.project {
            if let Ok(project) = Project::try_from(project_path.clone()) {
                info!("Setting up from existing project");
                project.into()
            } else {
                info!("Setting up new project");
                // TODO: Merge with else branch, they are practically the same code.
                let mut builder = ProjectBuilder::default();

                builder = builder.path(project_path);
                builder
            }
        } else if let Ok(project) = Project::try_from(PathBuf::from(".")) {
            info!("Setting up new project in current directory");
            project.into()
        } else {
            info!("Setting up new project in default location");
            ProjectBuilder::default()
        };

        for p in args.package {
            builder = builder.package(p.package);
        }
        for m in args.module {
            builder = builder.module(m.module);
        }
        for d in args.directory {
            builder = builder.directory((d.src, d.dst));
        }
        for f in args.file {
            builder = builder.file((f.src, f.dst));
        }
        for s in args.path_symlink {
            builder = builder.path_symlink((s.src, s.dst));
        }

        let project = builder
            .module(
                ModuleBuilder::default()
                    .artifact(
                        ArtifactDependencyBuilder::default()
                            .crate_name(TSFFS_MODULE_CRATE_NAME)
                            .workspace_root(PathBuf::from(TSFFS_WORKSPACE_PATH))
                            .build_missing(true)
                            .artifact_type(CrateType::CDynamicLibrary)
                            .feature(SIMICS_VERSION)
                            .target_name("simics-fuzz")
                            .build()?
                            .build()?,
                    )
                    .build()?,
            )
            .build()?
            .setup()?;

        SimicsFuzzerBuilder::default()
            .project(project)
            .input(args.input)
            .corpus(args.corpus)
            .solutions(args.solutions)
            .tui(args.tui)
            ._grimoire(args.grimoire)
            .cores(Cores::from((0..args.cores).collect::<Vec<_>>()))
            .command(args.command)
            .timeout(args.timeout)
            .executor_timeout(args.executor_timeout)
            .log_level(args.log_level)
            .trace_mode(args.trace_mode)
            .simics_gui(args.enable_simics_gui)
            .iterations(args.iterations)
            .tui_stdout_file(args.tui_stdout_file)
            .repro(args.repro)
            .build()?
            .launch()?;

        Ok(())
    }

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
        let simics_gui = self.simics_gui;

        spawn(move || -> Result<()> {
            // TODO: Take these args from CLI, we should let users override anything including GUI
            // mode if they really want to
            let mut simics_args =
                InitArgs::default().arg(InitArg::deprecation_level(DeprecationLevel::NoWarnings)?);

            if simics_gui {
                debug!("Enabling SIMICS GUI");
                simics_args = simics_args.arg(InitArg::gui_mode(GuiMode::Default)?)
            } else {
                // By default, don't enable the GUI
                simics_args = simics_args
                    .arg(InitArg::batch_mode()?)
                    .arg(InitArg::gui_mode(GuiMode::None)?)
                    // TODO: Maybe disable this if we can output logs through tracing?
                    .arg(InitArg::no_windows()?);
            }

            simics_args = simics_args.arg(InitArg::project(path.to_string_lossy().to_string())?);

            Simics::try_init(simics_args)?;

            // Doesn't matter if the script has a `@SIM_load_module` in it, as long as it isn't
            // asserting that it returns not-None
            load_module(TSFFS_MODULE_CRATE_NAME)?;

            info!("Loaded SIMICS module");

            let tsffs = create_object(
                get_class(TSFFS_MODULE_CRATE_NAME)?,
                TSFFS_MODULE_CRATE_NAME,
                alloc_attr_list(0),
            )?;

            let tsffs_interface = get_interface::<ModuleInterface>(
                tsffs,
                Interface::Other(TSFFS_MODULE_CRATE_NAME.to_string()),
            )?;

            let tx = Box::new(make_attr_data_adopt(tx)?);
            let rx = Box::new(make_attr_data_adopt(rx)?);
            let tx = Box::into_raw(tx);
            let rx = Box::into_raw(rx);

            info!("Setting up channels");

            (unsafe { *tsffs_interface }.add_channels)(tsffs, tx, rx);

            info!("Set channel for object");

            command.iter().try_for_each(|c| match c {
                Command::Command { command } => Simics::command(command),
                Command::Python { file } => Simics::python(file.canonicalize(&path)?),
                Command::Config { config } => Simics::config(config.canonicalize(&path)?),
            })?;

            info!("Finished running provided commands");

            // If the command we just ran includes `@SIM_main_loop`, the below code will be unreachable, but that is OK. The callbacks will eventually call `@SIM_quit` for us
            // and this will never be called. If the command doesn't include, `@SIM_main_loop`, then we need to enter it now.

            Simics::run();
        })
    }

    pub fn repro(&mut self) -> Result<()> {
        let mut coverage_map = OwnedMutSlice::from(vec![0; SimicsFuzzer::MAP_SIZE]);

        let config = InputConfigBuilder::default()
            .coverage_map((
                coverage_map.as_mut_slice().as_mut_ptr(),
                coverage_map.as_slice().len(),
            ))
            .trace_mode(self.trace_mode)
            .timeout(self.timeout)
            .log_level(self.log_level)
            .repro(true)
            .build()
            .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

        let (tx, orx) = channel::<ClientMessage>();
        let (otx, rx) = channel::<ModuleMessage>();

        let _simics = self.simics(otx, orx);
        let mut client = Client::new(tx, rx);

        let _output_config = client
            .initialize(config)
            .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

        client
            .reset()
            .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;
        let run_input = read(
            self.repro
                .as_ref()
                .ok_or_else(|| anyhow!("Repro file disappeared"))?,
        )?;

        client.run(run_input)?;

        Ok(())
    }

    pub fn launch(&mut self) -> Result<()> {
        if self.tui {
            self.log_level = LevelFilter::ERROR;
        }

        if self.repro.is_some() {
            return Ok(self.repro()?);
        }

        let shmem_provider = StdShMemProvider::new()?;

        let broker_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

        let mut run_client = |state: Option<_>,
                              mut mgr: LlmpRestartingEventManager<_, _>,
                              cpu_id|
         -> Result<(), libafl::Error> {
            debug!("Running on CPU {:?}", cpu_id);

            let mut coverage_map = OwnedMutSlice::from(vec![0; SimicsFuzzer::MAP_SIZE]);

            let config = InputConfigBuilder::default()
                .coverage_map((
                    coverage_map.as_mut_slice().as_mut_ptr(),
                    coverage_map.as_slice().len(),
                ))
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
                Some(OnDiskMetadataFormat::Json),
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

                debug!("Running with input '{:?}'", run_input);

                debug!("Sending run signal");

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
                        StopReason::Breakpoint(breakpoint_number) => {
                            info!("Target got a breakpoint #{}", breakpoint_number);
                            exit_kind = ExitKind::Crash;
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
                    info!(
                        "No corpus provided. Generating {} initial inputs",
                        INITIAL_INPUTS
                    );
                    let mut generator = RandBytesGenerator::new(64);
                    state.generate_initial_inputs(
                        &mut fuzzer,
                        &mut executor,
                        &mut generator,
                        &mut mgr,
                        INITIAL_INPUTS,
                    )?;
                }
                info!("Imported {} inputs from disk", state.corpus().count());

                if state.corpus().count() == 0 {
                    error!(
                        "No interesting cases found from inputs! This may mean \
                        your harness is incorrect (check your arguments), your inputs \
                        are not triggering new code paths, or all inputs are causing \
                        crashes."
                    );
                    mgr.send_exiting()?;
                    return Ok(());
                }
            }

            let mut stages = tuple_list!(calibration, std_power);

            if let Some(iterations) = self.iterations {
                info!("Fuzzing for {} iterations", iterations);
                fuzzer.fuzz_loop_for(
                    &mut stages,
                    &mut executor,
                    &mut state,
                    &mut mgr,
                    iterations,
                )?;
                mgr.send_exiting()?;
                info!("Done fuzzing");
            } else {
                info!("Fuzzing until stopped");
                fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
            }

            client
                .exit()
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            simics
                .join()
                .map_err(|e| libafl::Error::Unknown(format!("{:?}", e), ErrorBacktrace::new()))?
                .map_err(|e| libafl::Error::Unknown(format!("{:?}", e), ErrorBacktrace::new()))?;

            info!("Fuzzer done, bye!");

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
                // NOTE: We send stdout to /dev/null when using the TUI. If users are using the TUI
                // and want logs, they need to pass `-L /whatever.txt`. We don't want to send
                // stdout there.
                .stdout_file(Some(&self.tui_stdout_file.as_os_str().to_string_lossy()))
                .build()
                .launch()
            {
                Ok(()) => {}
                Err(libafl::Error::ShuttingDown) => {
                    info!("Fuzzer stopped by user. shutting down.");
                }
                res @ Err(_) => {
                    return res.map_err(|e| anyhow!("Failed to run launcher: {}", e));
                }
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
        }

        self.project.path.remove_on_drop(true);

        Ok(())
    }
}
