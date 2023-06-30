use self::components::{detector::Detector, tracer::Tracer};
use crate::{
    config::OutputConfig,
    magic::Magic,
    messages::{client::ClientMessage, module::ModuleMessage},
    processor::Processor,
    state::State,
    stops::StopReason,
    traits::{ConfuseInterface, ConfuseState},
    CLASS_NAME,
};
use anyhow::{anyhow, bail, Context, Result};
use raffl_macro::{callback_wrappers, params};
use tracing::{error, info, trace};

use simics_api::{
    attr_data, attr_object_or_nil_from_ptr, break_simulation, clear_exception,
    continue_simulation_alone, discard_future, get_processor_number, hap_add_callback, last_error,
    quit, register_interface, restore_micro_checkpoint, save_micro_checkpoint, AttrValue,
    ConfObject, Hap, HapCallback, MicroCheckpointFlags, SimException,
};
use simics_api::{Create, Module};
use simics_api_macro::module;
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::mpsc::{channel, Receiver, Sender},
};
use tracing_subscriber::fmt;

pub mod components;

#[module(class_name = CLASS_NAME)]
pub struct Confuse {
    /// In test mode, CONFUSE runs without a real client,
    state: State,
    tx: Sender<ModuleMessage>,
    rx: Receiver<ClientMessage>,
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
        let state = State::new();
        let detector = Detector::try_new()?;
        let tracer = Tracer::try_new()?;
        let (tx, _) = channel();
        let (_, rx) = channel();

        Ok(Confuse::new(
            module_instance,
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
}

impl Confuse {
    pub fn initialize(&mut self) -> Result<()> {
        // Add callbacks on stops and magic instructions

        // TODO: bruh
        let self_ptr = self as *mut Self as *mut ConfObject;

        info!("Adding HAPs");

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

        let mut input_config = match self.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        fmt::fmt()
            .pretty()
            .with_max_level(input_config.log_level)
            .try_init()
            .map_err(|e| anyhow!("Couldn't initialize tracing subscriber: {}", e))?;

        info!("SIMICS logger initialized");

        output_config = self
            .detector
            .on_initialize(self_ptr, &mut input_config, output_config)?;
        output_config = self
            .tracer
            .on_initialize(self_ptr, &mut input_config, output_config)?;

        info!("Sending initialized message");

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
            error!("Received Exit message, exiting and quitting");
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

        restore_micro_checkpoint(0);
        discard_future();

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
            // Write the testcase to the guest's memory
            processor.write_bytes(self.buffer_address, &input)?;
            // Write the testcase size back to rdi
            processor.set_reg_value("rdi", input.len() as u64)?;
        }

        // Run the simulation until the magic start instruction, where we will receive a stop
        // callback
        self.stop_reason = None;

        continue_simulation_alone();

        Ok(())
    }
}

impl From<*mut std::ffi::c_void> for &mut Confuse {
    /// Convert from a *mut Confuse pointer to a mutable reference to Confuse
    fn from(value: *mut std::ffi::c_void) -> &'static mut Confuse {
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
        info!(
            "Confuse got stopped simulation with reason {:?}",
            self.stop_reason
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
                            self.detector.pre_first_run(self_ptr)?;
                            self.tracer.pre_first_run(self_ptr)?;
                            self.reset_and_run(processor_number)?;
                        } else {
                            self.iterations += 1;

                            self.detector.on_run(self_ptr)?;
                            self.tracer.on_run(self_ptr)?;

                            self.stop_reason = None;
                            self.last_start_processor_number = processor_number;

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
                        self.reset_and_run(processor_number)?;
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
        trace!("Got Magic instruction callback");
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
        info!("Adding processor");
        self.detector.on_add_processor(processor)?;
        self.tracer.on_add_processor(processor)?;

        let processor_obj: *mut ConfObject = attr_object_or_nil_from_ptr(processor)?;
        let processor_number = get_processor_number(processor_obj);

        let processor = Processor::try_new(processor_number, processor_obj)?
            .try_with_int_register(processor)?
            .try_with_processor_info_v2(processor)?;

        self.processors.insert(processor_number, processor);

        info!("Added processor #{}", processor_number);

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject)]
    pub fn on_start(&mut self) -> Result<()> {
        info!("Received start signal, initializing CONFUSE state.");
        self.initialize()?;

        // Trigger anything that needs to happen before we start up (run for the first time)
        self.detector.on_start()?;
        self.tracer.on_start()?;

        // Run -- we will get a callback on the Magic::Start instruction
        // trace!("Running until first `Magic::Start`");

        // continue_simulation_alone();
        // The user needs to

        // trace!("Registered continue to run at next opportunity");
        info!("CONFUSE ready. Continue or run to the harness and fuzzing will begin.");

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject, ...)]
    ///
    /// # Safety
    ///
    /// This function dereferences the raw pointers passed into it through the interface. These
    /// pointers must be valid.
    pub fn on_add_channels(&mut self, tx: *mut AttrValue, rx: *mut AttrValue) -> Result<()> {
        info!(
            "Setting up channels Tx: {:#x} Rx: {:#x}",
            tx as usize, rx as usize
        );

        self.tx = attr_data(unsafe { *tx }).map_err(|e| {
            error!("Couldn't make attr data from pointer for tx");
            e
        })?;
        self.rx = attr_data(unsafe { *rx }).map_err(|e| {
            error!("Couldn't make attr data from pointer for tx");
            e
        })?;
        info!("Set up channels");

        match clear_exception()? {
            SimException::NoException => Ok(()),
            exception => {
                bail!(
                    "Error running simics config: {:?}: {}",
                    exception,
                    last_error()
                );
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
/// This is the rust definition for the confuse_module_interface_t declaration in the stubs, which
/// are used to generate the interface module. This struct definition must match that one exactly
pub struct ConfuseModuleInterface {
    pub start: extern "C" fn(obj: *mut ConfObject),
    pub add_processor: extern "C" fn(obj: *mut ConfObject, processor: *mut AttrValue),
    pub add_fault: extern "C" fn(obj: *mut ConfObject, fault: i64),
    pub add_channels: extern "C" fn(obj: *mut ConfObject, tx: *mut AttrValue, rx: *mut AttrValue),
}

impl Default for ConfuseModuleInterface {
    fn default() -> Self {
        Self {
            start: confuse_callbacks::on_start,
            add_processor: confuse_callbacks::on_add_processor,
            add_fault: confuse_callbacks::on_add_fault,
            add_channels: confuse_callbacks::on_add_channels,
        }
    }
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    eprintln!("Initializing CONFUSE");
    let cls = Confuse::create()
        .unwrap_or_else(|e| panic!("Failed to create class {}: {}", CLASS_NAME, e));

    eprintln!("Created CONFUSE class at {:#x}", cls as usize);

    register_interface::<_, ConfuseModuleInterface>(cls, CLASS_NAME).unwrap_or_else(|e| {
        panic!(
            "Failed to register interface for class {}: {}",
            CLASS_NAME, e
        )
    });

    eprintln!("Registered CONFUSE interface");
}
