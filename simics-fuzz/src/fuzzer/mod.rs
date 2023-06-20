use self::{feedbacks::ShrinkMapFeedback, monitors::MultiOrTui, observers::SizeValueObserver};
use crate::{
    args::command::Command,
    modules::confuse::{ConfuseModuleInterface, CONFUSE_MODULE, CONFUSE_MODULE_CRATE_NAME},
};
use anyhow::{anyhow, bail, Error, Result};
use confuse_module::{
    client::Client,
    messages::{client::ClientMessage, module::ModuleMessage},
    traits::ConfuseClient,
};
use derive_builder::Builder;
use libafl::{
    bolts::core_affinity::Cores,
    feedback_and, feedback_and_fast, feedback_not, feedback_or, feedback_or_fast,
    prelude::{
        current_nanos, havoc_mutations,
        ondisk::OnDiskMetadataFormat,
        tokens_mutations,
        tui::{ui::TuiUI, TuiMonitor},
        tuple_list, AflMapFeedback, AsMutSlice, BytesInput, CachedOnDiskCorpus, Corpus,
        CrashFeedback, EventConfig, ExitKind, HitcountsMapObserver, InProcessExecutor, Launcher,
        MaxMapFeedback, Merge, MinMapFeedback, Monitor, MultiMonitor, OnDiskCorpus, OwnedMutSlice,
        OwnedSlice, RandBytesGenerator, ShMemProvider, StdMapObserver, StdRand,
        StdScheduledMutator, StdShMemProvider, TimeFeedback, TimeObserver, TimeoutExecutor,
    },
    schedulers::{
        powersched::PowerSchedule, IndexesLenTimeMinimizerScheduler, PowerQueueScheduler,
    },
    stages::{CalibrationStage, GeneralizationStage, IfStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
    ErrorBacktrace, Fuzzer, StdFuzzer,
};
use observers::MappedEdgeMapObserver;
use simics::{
    api::{
        alloc_attr_list, attr_list, create_object, get_all_modules, get_class, get_interface,
        get_object, load_module, main_loop, DeprecationLevel, GuiMode, InitArg, InitArgs,
        Interface,
    },
    project::Project,
    simics::Simics,
};
use std::{
    f32::consts::E,
    net::TcpListener,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{spawn, JoinHandle},
    time::Duration,
};
use tracing::{info, Level};

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
    shrink: bool,
    dedup: bool,
    grimoire: bool,
    timeout: u64,
    cores: Cores,
    command: Vec<Command>,
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

    pub fn simics<'a>(
        &self,
        tx: Sender<ModuleMessage>,
        rx: Receiver<ClientMessage>,
        coverage_map: OwnedMutSlice<'a, u8>,
    ) -> JoinHandle<Result<()>> {
        let path = self.project.path.path.clone();
        let command = self.command.clone();
        spawn(move || -> Result<()> {
            // TODO: Take these args from CLI, we should let users override anything including GUI
            // mode if they really want to
            let simics_args = InitArgs::default()
                .arg(InitArg::batch_mode()?)
                .arg(InitArg::deprecation_level(DeprecationLevel::NoWarnings)?)
                .arg(InitArg::gui_mode(GuiMode::None)?)
                // TODO: Maybe disable this if we can output logs through tracing?
                .arg(InitArg::log_enable()?)
                .arg(InitArg::no_windows()?)
                .arg(InitArg::project(path.to_string_lossy().to_string())?)
                // TODO: maybe disable these for verbosity reasons
                .arg(InitArg::script_trace()?)
                .arg(InitArg::verbose()?);

            Simics::try_init(simics_args)?;

            // Doesn't matter if the script has a `@SIM_load_module` in it, as long as it isn't
            // asserting that it returns not-None
            load_module(CONFUSE_MODULE_CRATE_NAME)?;

            let confuse = create_object(
                get_class(CONFUSE_MODULE_CRATE_NAME)?,
                CONFUSE_MODULE_CRATE_NAME,
                alloc_attr_list(0),
            )?;

            // Set up the sender and receiver
            let confuse_interface = get_interface::<ConfuseModuleInterface>(
                confuse,
                Interface::Other(CONFUSE_MODULE_CRATE_NAME.to_string()),
            )?;

            ((unsafe { *confuse_interface }).set_channel)(confuse, Box::new(tx), Box::new(rx));

            command.iter().try_for_each(|c| match c {
                Command::Command { command } => Simics::command(command),
                Command::Python { file } => Simics::python(file.canonicalize(&path)?),
                Command::Config { config } => Simics::config(config.canonicalize(&path)?),
            })?;

            // If the command we just ran includes `@SIM_main_loop`, the below code will be unreachable, but that is OK. The callbacks will eventually call `@SIM_quit` for us
            // and this will never be called. If the command doesn't include, `@SIM_main_loop`, then we need to enter it now.

            ((unsafe { *confuse_interface }).start)(confuse);

            Simics::run();
        })
    }

    pub fn launch(&self) -> Result<()> {
        let shmem_provider = StdShMemProvider::new()?;

        let broker_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

        let mut run_client = |mut state: Option<_>, mut mgr, cpu_id| -> Result<(), libafl::Error> {
            let coverage_map = OwnedMutSlice::from(vec![0; SimicsFuzzer::MAP_SIZE]);

            let (tx, orx) = channel::<ClientMessage>();
            let (otx, rx) = channel::<ModuleMessage>();

            let simics = self.simics(otx, orx, OwnedMutSlice::from(coverage_map.as_mut_slice()));
            let client = Client::new(tx, rx);

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
            let scheduler = IndexesLenTimeMinimizerScheduler::new(PowerQueueScheduler::new(
                &mut state,
                &counters_observer,
                PowerSchedule::FAST,
            ));
            let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

            let mut harness = |input: &BytesInput| ExitKind::Ok;

            let mut executor = TimeoutExecutor::new(
                InProcessExecutor::new(
                    &mut harness,
                    tuple_list!(counters_observer, time_observer),
                    &mut fuzzer,
                    &mut state,
                    &mut mgr,
                )?,
                Duration::from_secs(self.timeout),
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
                    let mut generator = RandBytesGenerator::from(RandBytesGenerator::new(64));
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

            Ok(())
        };

        // TODO: Deduplicate this nastiness
        if self.tui {
            let monitor = TuiMonitor::new(TuiUI::new(Self::NAME.to_owned(), true));
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

        Ok(())
    }
}
