use self::components::{detector::Detector, tracer::Tracer};
use crate::{
    client::test::TestClient,
    config::OutputConfig,
    magic::Magic,
    messages::{client::ClientMessage, module::ModuleMessage},
    processor::Processor,
    state::State,
    stops::StopReason,
    traits::{ConfuseClient, ConfuseInterface, ConfuseState},
    BOOTSTRAP_SOCKNAME, CLASS_NAME, TESTMODE_VARNAME,
};
use anyhow::{bail, ensure, Context, Result};
use const_format::concatcp;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use log::{debug, info, trace, Level, LevelFilter};
use raffl_macro::{callback_wrappers, params};
use simics_api::{
    attr_object_or_nil_from_ptr, break_simulation, continue_simulation_alone, get_processor_number,
    hap_add_callback, quit, register_interface, save_micro_checkpoint, AttrValue, ConfObject, Hap,
    HapCallback, MicroCheckpointFlags, SimicsLogger,
};
use simics_api::{Create, Module};
use simics_api_macro::module;
use std::{collections::HashMap, env::var, ffi::c_void, str::FromStr};

pub mod components;

pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");
pub const DEFAULT_LOGLEVEL: Level = Level::Trace;

#[module(class_name = CLASS_NAME)]
pub struct Confuse {
    /// In test mode, CONFUSE runs without a real client,
    test_mode_client: Option<Box<dyn ConfuseClient>>,
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    tracer: Tracer,
    detector: Detector,
    processors: HashMap<i32, Processor>,
    stop_reason: Option<StopReason>,
    iterations: usize,
    buffer_address: u64,
    buffer_size: u64,
    last_start_processor_number: i32,
}

impl Module for Confuse {
    fn init(module_instance: *mut ConfObject) -> Result<*mut ConfObject> {
        let log_level = LevelFilter::from_str(&var(LOGLEVEL_VARNAME).unwrap_or_default())
            .unwrap_or(DEFAULT_LOGLEVEL.to_level_filter());

        let test_mode = if let Ok(name) = var(TESTMODE_VARNAME) {
            matches!(name.to_ascii_lowercase().as_str(), "1" | "true" | "on")
        } else {
            false
        };

        SimicsLogger::new()
            // Dev is a misnomer here -- that's what SIMICS calls it but really it should just be
            // `object` because that's all we are doing here is creating a logger for our module object
            .with_dev(module_instance)
            .with_level(log_level)
            .init()?;

        let state = State::new();
        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;
        let test_client = if test_mode {
            Some(TestClient::new_boxed(otx, orx))
        } else {
            info!("Initializing CONFUSE");

            let sockname = var(BOOTSTRAP_SOCKNAME)?;
            debug!("Connecting to bootstrap socket {}", sockname);

            let bootstrap = IpcSender::connect(sockname)?;

            debug!("Connected to bootstrap socket");

            bootstrap.send((otx, orx))?;

            debug!("Sent primary socket over bootstrap socket");

            None
        };
        let detector = Detector::try_new()?;
        let tracer = Tracer::try_new()?;

        Ok(Confuse::new(
            module_instance,
            test_client,
            state,
            tx,
            rx,
            tracer,
            detector,
            HashMap::new(),
            None,
            0,
            0,
            0,
            -1,
        ))
    }

    fn objects_finalized(module_instance: *mut ConfObject) -> Result<()> {
        let confuse: &mut Confuse = module_instance.into();
        confuse.initialize()?;

        Ok(())
    }
}

impl Confuse {
    pub fn initialize(&mut self) -> Result<()> {
        let input_config = match self.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        // Add callbacks on stops and magic instructions

        // TODO: bruh
        let self_ptr = self as *mut Self as *mut ConfObject;

        hap_add_callback(
            Hap::CoreSimulationStopped,
            HapCallback::CoreSimulationStopped(confuse_callbacks::on_simulation_stopped),
            Some(self_ptr as *mut c_void),
        )?;

        hap_add_callback(
            Hap::CoreMagicInstruction,
            HapCallback::CoreMagicInstruction(confuse_callbacks::on_magic_instruction),
            Some(self_ptr as *mut c_void),
        )?;

        let mut output_config = OutputConfig::default();

        output_config = self
            .detector
            .on_initialize(self_ptr, &input_config, output_config)?;
        output_config = self
            .tracer
            .on_initialize(self_ptr, &input_config, output_config)?;

        self.send_msg(ModuleMessage::Initialized(output_config))?;

        Ok(())
    }
}

impl Confuse {
    /// Send a message to the client
    fn send_msg(&mut self, msg: ModuleMessage) -> Result<()> {
        trace!("Sending module message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming sent message {:?}", msg))?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the client
    fn recv_msg(&mut self) -> Result<ClientMessage> {
        trace!("Waiting to receive client message");
        let msg = self.rx.recv()?;
        trace!("Received client message {:?}", msg);

        if matches!(msg, ClientMessage::Exit) {
            let self_ptr = self as *mut Self as *mut ConfObject;
            self.detector.on_exit(self_ptr)?;
            self.tracer.on_exit(self_ptr)?;
            quit(0);
        }

        self.state
            .consume(&msg)
            .context(format!("Error consuming received message {:?}", msg))?;
        Ok(msg)
    }

    fn reset_and_run(&mut self, processor_number: i32) -> Result<()> {
        let self_ptr = self as *mut Self as *mut ConfObject;
        // Tasks to do on reset
        if !matches!(self.recv_msg()?, ClientMessage::Reset) {
            bail!("Unexpected message. Expected Reset.");
        }

        self.detector.on_ready(self_ptr)?;
        self.tracer.on_ready(self_ptr)?;

        self.send_msg(ModuleMessage::Ready)?;

        let mut input = if let ClientMessage::Run(input) = self.recv_msg()? {
            input
        } else {
            bail!("Unexpected message. Expected Run.");
        };

        input.truncate(self.buffer_size as usize);
        {
            let processor = self
                .processors
                .get_mut(&processor_number)
                .with_context(|| format!("No processor number {}", processor_number))?;
            processor.write_bytes(self.buffer_address, &input)?;
        }

        self.detector.on_run(self_ptr)?;
        self.tracer.on_run(self_ptr)?;

        self.stop_reason = None;
        self.last_start_processor_number = processor_number;

        continue_simulation_alone();

        Ok(())
    }
}

impl<'a> From<*mut std::ffi::c_void> for &'a mut Confuse {
    /// Convert from a *mut Confuse pointer to a mutable reference to Confuse
    fn from(value: *mut std::ffi::c_void) -> &'a mut Confuse {
        let confuse_ptr: *mut Confuse = value as *mut Confuse;
        unsafe { &mut *confuse_ptr }
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl Confuse {
    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_simulation_stopped(
        &mut self,
        _trigger_obj: *mut ConfObject,
        // Exception is always SimExc_No_Exception
        _exception: i64,
        // Error string is always NULL
        _error_string: *mut std::ffi::c_char,
    ) -> Result<()> {
        ensure!(
            !(self.detector.stop_reason.is_some() && self.stop_reason.is_some()),
            "Confuse and Detector both have a stop reason - this should be impossible"
        );

        let reason = if let Some(detector_reason) = &self.detector.stop_reason {
            detector_reason
        } else if let Some(reason) = &self.stop_reason {
            reason
        } else {
            bail!("Stopped without a reason - this should be impossible");
        }
        .clone();

        // TODO: bruh
        let self_ptr = self as *mut Self as *mut ConfObject;
        self.detector.on_stopped(self_ptr, reason.clone())?;
        self.tracer.on_stopped(self_ptr, reason.clone())?;

        match reason {
            StopReason::Magic((magic, processor_number)) => {
                match magic {
                    Magic::Start(_) => {
                        if self.iterations == 0 {
                            self.iterations += 1;
                            // Tasks to do before first run
                            {
                                let processor =
                                    self.processors.get_mut(&processor_number).with_context(
                                        || format!("No processor number {}", processor_number),
                                    )?;
                                self.buffer_address = processor.get_reg_value("rsi")?;
                                self.buffer_size = processor.get_reg_value("rdi")?;
                            }
                            save_micro_checkpoint(
                                "origin",
                                &[
                                    MicroCheckpointFlags::IdUser,
                                    MicroCheckpointFlags::Persistent,
                                ],
                            )?;
                            self.reset_and_run(processor_number)?;
                        } else {
                            self.iterations += 1;
                            continue_simulation_alone();
                        }
                    }
                    Magic::Stop((code, _)) => {
                        let processor = self
                            .processors
                            .get_mut(&processor_number)
                            .with_context(|| format!("No processor number {}", processor_number))?;
                        let stop_value = processor.get_reg_value("rsi")?;
                        let magic = Magic::Stop((code, Some(stop_value)));
                        self.send_msg(ModuleMessage::Stopped(StopReason::Magic((
                            magic,
                            processor_number,
                        ))))?;
                    }
                }
            }
            StopReason::SimulationExit(processor_number) => {
                self.send_msg(ModuleMessage::Stopped(StopReason::SimulationExit(
                    processor_number,
                )))?;
                self.reset_and_run(processor_number)?;
            }
            StopReason::Crash((fault, processor_number)) => {
                self.send_msg(ModuleMessage::Stopped(StopReason::Crash((
                    fault,
                    processor_number,
                ))))?;
                self.reset_and_run(processor_number)?;
            }
            StopReason::TimeOut => {
                self.send_msg(ModuleMessage::Stopped(StopReason::TimeOut))?;
                let processor_number = self.last_start_processor_number;
                self.reset_and_run(processor_number)?;
            }
            StopReason::Error((_error, _processor_number)) => {
                // TODO: Error reporting
                let self_ptr = self as *mut Self as *mut ConfObject;
                self.detector.on_exit(self_ptr)?;
                self.tracer.on_exit(self_ptr)?;
                quit(1);
            }
        }

        Ok(())
    }

    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_magic_instruction(
        &mut self,
        trigger_obj: *mut ConfObject,
        parameter: i64,
    ) -> Result<()> {
        // The trigger obj is a CPU
        let processor_number = get_processor_number(trigger_obj);

        if let Ok(magic) = Magic::try_from(parameter) {
            self.stop_reason = Some(StopReason::Magic((magic, processor_number)));

            break_simulation("on_magic_instruction")?;
        }

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject, ...)]
    pub fn on_add_fault(&mut self, fault: i64) -> Result<()> {
        self.detector.on_add_fault(fault)?;
        self.tracer.on_add_fault(fault)?;

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject, ...)]
    pub fn on_add_processor(&mut self, processor: *mut AttrValue) -> Result<()> {
        self.detector.on_add_processor(processor)?;
        self.tracer.on_add_processor(processor)?;

        let processor_obj: *mut ConfObject = attr_object_or_nil_from_ptr(processor)?;
        let processor_number = get_processor_number(processor_obj);
        let processor = Processor::try_new(processor_number, processor_obj)?
            .try_with_int_register(processor)?
            .try_with_processor_info_v2(processor)?;

        self.processors.insert(processor_number, processor);

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject)]
    pub fn on_start(&mut self) -> Result<()> {
        // Trigger anything that needs to happen before we start up (run for the first time)
        self.detector.on_start()?;
        self.tracer.on_start()?;

        // Run -- we will get a callback on the Magic::Start instruction
        continue_simulation_alone();

        Ok(())
    }
}

/// This is the rust definition for the confuse_module_interface_t declaration in the stubs, which
/// are used to generate the interface module. This struct definition must match that one exactly
pub struct ConfuseModuleInterface {
    pub start: extern "C" fn(obj: *mut ConfObject),
    pub add_processor: extern "C" fn(obj: *mut ConfObject, processor: *mut AttrValue),
    pub add_fault: extern "C" fn(obj: *mut ConfObject, fault: i64),
}

impl ConfuseModuleInterface {
    pub const INTERFACE_NAME: &str = CLASS_NAME;
    pub const INTERFACE_TYPENAME: &str =
        concatcp!(ConfuseModuleInterface::INTERFACE_NAME, "_interface_t");
}

impl Default for ConfuseModuleInterface {
    fn default() -> Self {
        Self {
            start: confuse_callbacks::on_start,
            add_processor: confuse_callbacks::on_add_processor,
            add_fault: confuse_callbacks::on_add_fault,
        }
    }
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    let cls = Confuse::create().unwrap_or_else(|_| panic!("Failed to create class {}", CLASS_NAME));

    register_interface::<_, ConfuseModuleInterface>(unsafe { &*cls }, CLASS_NAME)
        .unwrap_or_else(|_| panic!("Failed to register interface for class {}", CLASS_NAME));
}
