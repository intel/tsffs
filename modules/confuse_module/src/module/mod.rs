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
    attr_object_or_nil_from_ptr, break_simulation, continue_simulation_alone, discard_future,
    get_processor_number, hap_add_callback, quit, register_interface, restore_micro_checkpoint,
    save_micro_checkpoint, AttrValue, ConfObject, Hap, HapCallback, MicroCheckpointFlags,
};
use simics_api::{Create, Module};
use simics_api_macro::module;
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::mpsc::{Receiver, Sender},
};

pub mod components;

#[module(class_name = CLASS_NAME)]
pub struct Confuse<'a> {
    /// In test mode, CONFUSE runs without a real client,
    state: State,
    tx: Option<Sender<ModuleMessage>>,
    rx: Option<Receiver<ClientMessage>>,
    tracer: Tracer<'a>,
    detector: Detector,
    processors: HashMap<i32, Processor>,
    stop_reason: Option<StopReason>,
    iterations: usize,
    buffer_address: u64,
    buffer_size: u64,
    last_start_processor_number: i32,
}

impl<'a> Module for Confuse<'a> {
    fn init(module_instance: *mut ConfObject) -> Result<*mut ConfObject> {
        info!("Simics logger initialized");

        let state = State::new();
        let detector = Detector::try_new()?;
        let tracer = Tracer::try_new()?;

        Ok(Confuse::new(
            module_instance,
            state,
            None,
            None,
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

    fn objects_finalized(_module_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }
}

impl<'a> Confuse<'a> {
    pub fn initialize(&mut self) -> Result<()> {
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

        let mut input_config = match self.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        output_config = self
            .detector
            .on_initialize(self_ptr, &mut input_config, output_config)?;
        output_config = self
            .tracer
            .on_initialize(self_ptr, &mut input_config, output_config)?;

        self.send_msg(ModuleMessage::Initialized(output_config))?;

        Ok(())
    }
}

impl<'a> Confuse<'a> {
    /// Send a message to the client
    fn send_msg(&mut self, msg: ModuleMessage) -> Result<()> {
        trace!("Sending module message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming sent message {:?}", msg))?;
        self.tx
            .as_ref()
            .map(|tx| tx.send(msg))
            .ok_or_else(|| anyhow!("Attempted to send a message before channels were set"))??;
        Ok(())
    }

    /// Receive a message from the client
    fn recv_msg(&mut self) -> Result<ClientMessage> {
        trace!("Waiting to receive client message");
        let msg =
            self.rx.as_ref().map(|rx| rx.recv()).ok_or_else(|| {
                anyhow!("Attempted to receive a message before channels were set")
            })??;
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
        }

        // Run the simulation until the magic start instruction, where we will receive a stop
        // callback
        self.stop_reason = None;

        continue_simulation_alone();

        Ok(())
    }
}

impl<'a> From<*mut std::ffi::c_void> for &'a mut Confuse<'a> {
    /// Convert from a *mut Confuse pointer to a mutable reference to Confuse
    fn from(value: *mut std::ffi::c_void) -> &'a mut Confuse<'a> {
        let confuse_ptr: *mut Confuse = value as *mut Confuse;
        unsafe { &mut *confuse_ptr }
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl<'a> Confuse<'a> {
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
        self.initialize()?;

        info!("Got start signal from client");
        // Trigger anything that needs to happen before we start up (run for the first time)
        self.detector.on_start()?;
        self.tracer.on_start()?;

        // Run -- we will get a callback on the Magic::Start instruction
        trace!("Running until first `Magic::Start`");
        continue_simulation_alone();

        trace!("Registered continue to run at next opportunity");

        Ok(())
    }

    #[params(!slf: *mut simics_api::ConfObject, ...)]
    // TODO: Enhance raffl-macro to unbox void * passed locals
    #[allow(clippy::boxed_local)]
    pub fn on_set_channel(
        &mut self,
        tx: Box<Sender<ModuleMessage>>,
        rx: Box<Receiver<ClientMessage>>,
    ) -> Result<()> {
        info!("Got channel");
        self.tx = Some(*tx);
        self.rx = Some(*rx);

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
/// This is the rust definition for the confuse_module_interface_t declaration in the stubs, which
/// are used to generate the interface module. This struct definition must match that one exactly
pub struct ConfuseModuleInterface {
    pub start: extern "C" fn(obj: *mut ConfObject),
    pub add_processor: extern "C" fn(obj: *mut ConfObject, processor: *mut AttrValue),
    pub add_fault: extern "C" fn(obj: *mut ConfObject, fault: i64),
    pub set_channel: extern "C" fn(
        obj: *mut ConfObject,
        tx: Box<Sender<ModuleMessage>>,
        rx: Box<Receiver<ClientMessage>>,
    ),
}

impl Default for ConfuseModuleInterface {
    fn default() -> Self {
        Self {
            start: confuse_callbacks::on_start,
            add_processor: confuse_callbacks::on_add_processor,
            add_fault: confuse_callbacks::on_add_fault,
            set_channel: confuse_callbacks::on_set_channel,
        }
    }
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    eprintln!("Initializing CONFUSE");
    // log_info(0, null_mut(), 0, "Initializing CONFUSE").expect("Couldn't initialize confuse");
    // println!("Logged initializing confuse");
    let cls = Confuse::create().unwrap_or_else(|_| panic!("Failed to create class {}", CLASS_NAME));

    eprintln!("Created CONFUSE class at {:#x}", cls as usize);

    register_interface::<_, ConfuseModuleInterface>(cls, CLASS_NAME)
        .unwrap_or_else(|_| panic!("Failed to register interface for class {}", CLASS_NAME));
}
