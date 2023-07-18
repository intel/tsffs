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
        tuple_list, AFLppCmpMap, AFLppRedQueen, AflMapFeedback, AsMutSlice, AsSlice, BytesInput,
        CachedOnDiskCorpus, CmpMap, CmpObserver, Corpus, CorpusId, CrashFeedback, EventConfig,
        EventRestarter, ExitKind, HasTargetBytes, HitcountsMapObserver, InProcessExecutor,
        Launcher, LlmpRestartingEventManager, MultiMonitor, Named, Observer, OnDiskCorpus,
        OwnedMutSlice, OwnedRefMut, RandBytesGenerator, ShMemProvider, StdMapObserver, StdRand,
        StdScheduledMutator, StdShMemProvider, TimeFeedback, TimeObserver, TimeoutExecutor,
        UsesInput,
    },
    schedulers::{powersched::PowerSchedule, PowerQueueScheduler},
    stages::{
        mutational::MultipleMutationalStage, tracing::AFLppCmplogTracingStage, CalibrationStage,
        ColorizationStage, IfStage, StdPowerMutationalStage,
    },
    state::{HasCorpus, HasMetadata, StdState},
    ErrorBacktrace, Fuzzer, StdFuzzer,
};
use serde::{Deserialize, Serialize};
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
    cell::RefCell,
    fs::{set_permissions, OpenOptions, Permissions},
    io::stdout,
    marker::PhantomData,
    mem::MaybeUninit,
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
    faults::{x86_64::X86_64Fault, Fault},
    messages::{client::ClientMessage, module::ModuleMessage},
    stops::StopReason,
    traits::ThreadClient,
};

const INITIAL_INPUTS: usize = 16;

#[derive(Serialize, Deserialize, Debug)]
pub struct AFLppInprocessesCmpObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    cmp_map: OwnedRefMut<'a, AFLppCmpMap>,
    size: Option<OwnedRefMut<'a, usize>>,
    name: String,
    add_meta: bool,
    original: bool,
    phantom: PhantomData<S>,
}

impl<'a, S> CmpObserver<AFLppCmpMap, S> for AFLppInprocessesCmpObserver<'a, S>
where
    S: UsesInput + core::fmt::Debug + HasMetadata,
{
    fn usable_count(&self) -> usize {
        todo!()
    }

    fn cmp_map(&self) -> &AFLppCmpMap {
        todo!()
    }

    fn cmp_map_mut(&mut self) -> &mut AFLppCmpMap {
        todo!()
    }

    fn add_cmpvalues_meta(&mut self, state: &mut S)
    where
        S: HasMetadata,
    {
        #[allow(clippy::option_if_let_else)] // we can't mutate state in a closure
        let meta = if let Some(meta) = state
            .metadata_map_mut()
            .get_mut::<libafl::prelude::CmpValuesMetadata>()
        {
            meta
        } else {
            state.add_metadata(libafl::prelude::CmpValuesMetadata::new());
            state
                .metadata_map_mut()
                .get_mut::<libafl::prelude::CmpValuesMetadata>()
                .expect("Couldn't get cmp values metadata")
        };
        meta.list.clear();
        let count = self.usable_count();
        for i in 0..count {
            let execs = self.cmp_map().usable_executions_for(i);
            if execs > 0 {
                // Recongize loops and discard if needed
                if execs > 4 {
                    let mut increasing_v0 = 0;
                    let mut increasing_v1 = 0;
                    let mut decreasing_v0 = 0;
                    let mut decreasing_v1 = 0;

                    let mut last: Option<libafl::prelude::CmpValues> = None;
                    for j in 0..execs {
                        if let Some(val) = self.cmp_map().values_of(i, j) {
                            if let Some(l) = last.and_then(|x| x.to_u64_tuple()) {
                                if let Some(v) = val.to_u64_tuple() {
                                    if l.0.wrapping_add(1) == v.0 {
                                        increasing_v0 += 1;
                                    }
                                    if l.1.wrapping_add(1) == v.1 {
                                        increasing_v1 += 1;
                                    }
                                    if l.0.wrapping_sub(1) == v.0 {
                                        decreasing_v0 += 1;
                                    }
                                    if l.1.wrapping_sub(1) == v.1 {
                                        decreasing_v1 += 1;
                                    }
                                }
                            }
                            last = Some(val);
                        }
                    }
                    // We check for execs-2 because the logged execs may wrap and have something like
                    // 8 9 10 3 4 5 6 7
                    if increasing_v0 >= execs - 2
                        || increasing_v1 >= execs - 2
                        || decreasing_v0 >= execs - 2
                        || decreasing_v1 >= execs - 2
                    {
                        continue;
                    }
                }
                for j in 0..execs {
                    if let Some(val) = self.cmp_map().values_of(i, j) {
                        meta.list.push(val);
                    }
                }
            }
        }
    }
}

impl<'a, S> Observer<S> for AFLppInprocessesCmpObserver<'a, S> where
    S: UsesInput + core::fmt::Debug + HasMetadata
{
}

impl<'a, S> Named for AFLppInprocessesCmpObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    fn name(&self) -> &str {
        &self.name
    }
}

impl<'a, S> AFLppInprocessesCmpObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    /// Creates a new [`ForkserverCmpObserver`] with the given name and map.
    #[must_use]
    pub fn new(name: &'static str, map: &'a mut AFLppCmpMap, add_meta: bool) -> Self {
        Self {
            name: name.to_string(),
            size: None,
            cmp_map: OwnedRefMut::Ref(map),
            add_meta,
            original: false,
            phantom: PhantomData,
        }
    }
    /// Setter for the flag if the executed input is a mutated one or the original one
    pub fn set_original(&mut self, v: bool) {
        self.original = v;
    }
}

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

            info!("Got tsffs at {:#x}", tsffs as usize);

            let tsffs_interface = get_interface::<ModuleInterface>(
                tsffs,
                Interface::Other(TSFFS_MODULE_CRATE_NAME.to_string()),
            )?;

            info!("Got SIMICS object: {:#x}", tsffs as usize);
            info!("Got tsffs interface at {:#x}", tsffs_interface as usize);

            let tx = Box::new(make_attr_data_adopt(tx)?);
            let rx = Box::new(make_attr_data_adopt(rx)?);
            let tx = Box::into_raw(tx);
            let rx = Box::into_raw(rx);

            info!("Tx: {:#x} Rx: {:#x}", tx as usize, rx as usize);

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

    pub fn launch(&mut self) -> Result<()> {
        if self.tui {
            self.log_level = LevelFilter::ERROR;
        }

        let shmem_provider = StdShMemProvider::new()?;

        let broker_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

        let mut run_client = |state: Option<_>,
                              mut mgr: LlmpRestartingEventManager<_, _>,
                              cpu_id|
         -> Result<(), libafl::Error> {
            debug!("Running on CPU {:?}", cpu_id);

            // let mut cmp_map = unsafe { MaybeUninit::<AFLppCmpMap>::zeroed().assume_init() };
            let mut coverage_map = OwnedMutSlice::from(vec![0; SimicsFuzzer::MAP_SIZE]);

            let config = InputConfigBuilder::default()
                .coverage_map((
                    coverage_map.as_mut_slice().as_mut_ptr(),
                    coverage_map.as_slice().len(),
                ))
                // .cmp_map(&mut cmp_map as *mut AFLppCmpMap)
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
            let client = RefCell::new(Client::new(tx, rx));

            let _output_config = client
                .borrow_mut()
                .initialize(config)
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            client
                .borrow_mut()
                .reset()
                .map_err(|e| libafl::Error::Unknown(e.to_string(), ErrorBacktrace::new()))?;

            // let cmp_observer = AFLppInprocessesCmpObserver::new("cmplog", &mut cmp_map, true);
            let counters_observer =
                HitcountsMapObserver::new(StdMapObserver::from_mut_slice("coverage", coverage_map));

            let counters_feedback = AflMapFeedback::new(&counters_observer);

            let time_observer = TimeObserver::new("time");

            // let colorization = ColorizationStage::new(&counters_observer);
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
                OnDiskMetadataFormat::Json,
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

                match client.borrow_mut().run(run_input) {
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

                if let Err(e) = client.borrow_mut().reset() {
                    error!("Error resetting SIMICS: {}", e);
                }

                debug!("Harness done");

                exit_kind
            };

            // let mut tracing_harness = |input: &BytesInput| {
            //     let target = input.target_bytes();
            //     let buf = target.as_slice();
            //     let run_input = buf.to_vec();
            //     let mut exit_kind = ExitKind::Ok;

            //     debug!("Running with input '{:?}'", run_input);

            //     debug!("Sending run signal");

            //     match client.borrow_mut().run(run_input) {
            //         Ok(reason) => match reason {
            //             StopReason::Magic((_magic, _p)) => {
            //                 exit_kind = ExitKind::Ok;
            //             }
            //             StopReason::SimulationExit(_) => {
            //                 exit_kind = ExitKind::Ok;
            //             }
            //             StopReason::Crash((fault, _p)) => {
            //                 info!("Target crashed with fault {:?}", fault);
            //                 exit_kind = ExitKind::Crash;
            //             }
            //             StopReason::TimeOut => {
            //                 info!("Target timed out");
            //                 exit_kind = ExitKind::Timeout;
            //             }
            //             StopReason::Error((e, _p)) => {
            //                 error!("An error occurred during execution: {:?}", e);
            //                 exit_kind = ExitKind::Ok;
            //             }
            //         },
            //         Err(e) => {
            //             error!("Error running SIMICS: {}", e);
            //         }
            //     }

            //     debug!("Sending reset signal");

            //     if let Err(e) = client.borrow_mut().reset() {
            //         error!("Error resetting SIMICS: {}", e);
            //     }

            //     debug!("Harness done");

            //     exit_kind
            // };

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

            // let cmp_executor = TimeoutExecutor::new(
            //     InProcessExecutor::new(
            //         &mut tracing_harness,
            //         tuple_list!(cmp_observer),
            //         &mut fuzzer,
            //         &mut state,
            //         &mut mgr,
            //     )?,
            //     // The executor's timeout can be quite long
            //     Duration::from_secs(self.executor_timeout),
            // );

            // let tracing =
            //     AFLppCmplogTracingStage::with_cmplog_observer_name(cmp_executor, "cmplog");

            // let redqueen =
            //     MultipleMutationalStage::new(AFLppRedQueen::with_cmplog_options(true, true));

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
            }

            // let cmp_cb = |_fuzzer: &mut _,
            //               _executor: &mut _,
            //               state: &mut StdState<_, CachedOnDiskCorpus<_>, _, _>,
            //               _event_manager: &mut _,
            //               corpus_id: CorpusId|
            //  -> Result<bool, libafl::Error> {
            //     let corpus = state.corpus().get(corpus_id)?.borrow();
            //     Ok(corpus.scheduled_count() == 1)
            // };

            // let cmp_stages = IfStage::new(cmp_cb, tuple_list!(colorization, tracing, redqueen));
            let mut stages = tuple_list!(calibration, /* cmp_stages , */ std_power);

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
                .borrow_mut()
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
