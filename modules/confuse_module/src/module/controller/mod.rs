use self::{
    instance::ControllerInstance,
    magic::Magic,
    messages::{client::ClientMessage, module::ModuleMessage},
    target_buffer::TargetBuffer,
};
use super::{
    component::{Component, ComponentInterface},
    config::{InputConfig, OutputConfig},
    cpu::Cpu,
    stop_reason::StopReason,
};
use crate::{
    module::{
        components::{detector::FaultDetector, tracer::AFLCoverageTracer},
        controller::instance::{alloc_controller_conf_object, init_controller_conf_object},
        entrypoint::{BOOTSTRAP_SOCKNAME, CLASS_NAME, LOGLEVEL_VARNAME},
    },
    state::State,
};
use anyhow::{bail, ensure, Context, Result};
use confuse_simics_api::{
    attr_value_t, class_data_t, class_kind_t_Sim_Class_Kind_Vanilla, conf_object_t,
    safe::{
        common::{
            continue_simulation, count_micro_checkpoints, hap_add_callback_magic_instruction,
            hap_add_callback_simulation_stopped,
        },
        wrapper::{
            break_simulation, discard_future, quit, register_class, register_interface,
            restore_micro_checkpoint, save_micro_checkpoint,
        },
    },
};
use const_format::concatcp;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use lazy_static::lazy_static;
use log::{info, trace, Level, LevelFilter};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    init_config, Handle,
};
use raw_cstr::raw_cstr;
use std::{
    cell::RefCell,
    env::var,
    ffi::CString,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
};

pub mod fault;
pub mod instance;
pub mod magic;
pub mod messages;
mod target_buffer;

lazy_static! {
    pub static ref CONTROLLER: Arc<Mutex<Controller>> = Arc::new(Mutex::new(
        Controller::try_new().expect("Could not initialize Controller")
    ));
    pub static ref TRACER: Arc<Mutex<AFLCoverageTracer>> = Arc::new(Mutex::new(
        AFLCoverageTracer::try_new().expect("Could not initialize AFLCoverageTracer")
    ));
    pub static ref DETECTOR: Arc<Mutex<FaultDetector>> = Arc::new(Mutex::new(
        FaultDetector::try_new().expect("Could not initialize fault detector")
    ));
}

/// Controller for the Confuse simics module. The controller is reponsible for communicating with
/// the client, dispatching messages, and implementing the overall state machine for the module
pub struct Controller {
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    _log_handle: Handle,
    stop_reason: Option<StopReason>,
    buffer: TargetBuffer,
    first_time_init_done: bool,
    cpus: Vec<RefCell<Cpu>>,
    instance: RefCell<ControllerInstance>,
}

// unsafe impl Send for Controller {}
// unsafe impl Sync for Controller {}

impl Controller {
    /// Retrieve the global controller object
    pub fn get<'a>() -> Result<MutexGuard<'a, Self>> {
        let controller = CONTROLLER.lock().expect("Could not lock controller");
        Ok(controller)
    }
}

impl Controller {
    pub const CLASS_NAME: &str = CLASS_NAME;
    pub const CLASS_DESCRIPTION: &str = r#"CONFUSE module controller class. This class controls general actions for the
        CONFUSE SIMICS module including configuration and run controls."#;
    pub const CLASS_SHORT_DESCRIPTION: &str = "CONFUSE controller";

    /// Try to create a new controller object by starting up the communication with the client
    pub fn try_new() -> Result<Self> {
        let level = LevelFilter::from_str(
            &var(LOGLEVEL_VARNAME).unwrap_or_else(|_| Level::Trace.as_str().to_string()),
        )
        .unwrap_or(LevelFilter::Trace);
        let stderr = ConsoleAppender::builder()
            .target(Target::Stderr)
            // For SIMICS we just output the message because we're going to get stuck into a log
            // message anyway, and we need a newline or all the outputs will get buffered. lol
            .encoder(Box::new(PatternEncoder::new("[{l:5}] {m}{n}")))
            .build();
        // let level = LevelFilter::Info;
        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .build(Root::builder().appender("stderr").build(level))?;
        let log_handle = init_config(config)?;

        let sockname = var(BOOTSTRAP_SOCKNAME)?;
        let bootstrap = IpcSender::connect(sockname)?;

        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;

        bootstrap.send((otx, orx))?;

        Ok(Self {
            state: State::new(),
            tx,
            rx,
            _log_handle: log_handle,
            stop_reason: None,
            buffer: TargetBuffer::default(),
            first_time_init_done: false,
            cpus: vec![],
            instance: RefCell::new(ControllerInstance::default()),
        })
    }

    /// Send a message to the module
    fn send_msg(&mut self, msg: ModuleMessage) -> Result<()> {
        trace!("Sending module message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming sent message {:?}", msg))?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the module
    fn recv_msg(&mut self) -> Result<ClientMessage> {
        trace!("Waiting to receive client message");
        let msg = self.rx.recv()?;
        trace!("Received client message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming received message {:?}", msg))?;
        Ok(msg)
    }

    /// Initialize the controller. This is called during module initialization after all
    /// components have been added
    pub fn initialize(&mut self) -> Result<()> {
        let input_config = match self.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        let output_config = OutputConfig::default();

        _ = self.on_initialize(&input_config, output_config)?;

        Ok(())
    }

    pub fn take_snapshot(&mut self) -> Result<()> {
        save_micro_checkpoint("origin");

        let sinfo_size = count_micro_checkpoints()?;

        info!("Took snapshot");

        ensure!(
            sinfo_size == 1,
            "Invalid size of state_info: {}",
            sinfo_size
        );

        Ok(())
    }

    pub fn quit_simulation(&mut self) {
        quit();
    }

    pub fn restore_snapshot(&mut self) -> Result<()> {
        restore_micro_checkpoint(0);
        discard_future();
        trace!("Restored snapshot");

        Ok(())
    }

    /// Stop the simulation with some reason for stopping
    pub fn stop_simulation(&mut self, reason: StopReason) {
        trace!("Stopped with reason: {:?}", reason);
        self.stop_reason = Some(reason.clone());
        break_simulation(format!("{:?}", reason));
    }
}

/// Implementation for methods the interface calls on us
impl Controller {
    /// Run the module
    ///
    /// # Safety
    ///
    /// This function is safe as long as `obj` is actually a non-null pointer to a `conf_object_t`
    pub unsafe fn interface_run(&mut self, obj: *mut conf_object_t) -> Result<()> {
        let instance = ControllerInstance::try_from_obj(obj)?;
        self.instance = RefCell::new(instance);

        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_run(&self.instance.borrow())?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.on_run(&self.instance.borrow())?;
        drop(detector);

        continue_simulation();

        Ok(())
    }

    /// Add a processor to the controller
    ///
    /// # Safety
    ///
    /// This function is safe as long as `obj` is actually a pointer to a non-null `conf_object_t`
    /// and `processor` is a pointer to an `attr_value_t`. If `processor` is not actually a
    /// processor, an error will occur, but the result is safe
    pub unsafe fn interface_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        unsafe { self.on_add_processor(obj, processor) }
    }

    /// Add a fault to the controller
    ///
    /// # Safety
    ///
    /// This function is safe as long as `obj` is actually a pointer to a non-null `conf_object_t`
    pub unsafe fn interface_add_fault(
        &mut self,
        obj: *mut conf_object_t,
        fault: i64,
    ) -> Result<()> {
        unsafe { self.on_add_fault(obj, fault) }
    }
}

/// Implementation for methods callbacks call on us
impl Controller {
    /// Called by SIMICS when the simulation stops
    pub fn on_magic_instruction_cb(&mut self, magic: Magic) -> Result<()> {
        self.stop_simulation(StopReason::Magic(magic));
        Ok(())
    }

    /// Called by SIMICS on magic instruction
    pub fn on_simulation_stopped_cb(&mut self) -> Result<()> {
        let reason = self.stop_reason.clone();
        unsafe { self.on_stop(reason, None) }?;
        Ok(())
    }
}

impl Component for Controller {
    /// Callback on initialization. For the controller, this is called directly, and it calls
    /// the on_initialize callbacks for all the other `Component`s
    fn on_initialize(
        &mut self,
        input_config: &InputConfig,
        mut output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        // First we register our class and interface

        let class_data = class_data_t {
            alloc_object: Some(alloc_controller_conf_object),
            init_object: Some(init_controller_conf_object),
            finalize_instance: None,
            pre_delete_instance: None,
            delete_instance: None,
            description: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
            class_desc: raw_cstr!(Controller::CLASS_DESCRIPTION),
            kind: class_kind_t_Sim_Class_Kind_Vanilla,
        };
        // let class_info = class_info_t {
        //     alloc: Some(alloc_controller_conf_object_for_create),
        //     init: Some(init_controller_conf_object_for_create),
        //     finalize: None,
        //     objects_finalized: None,
        //     deinit: None,
        //     dealloc: None,
        //     description: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
        //     short_desc: raw_cstr!(Controller::CLASS_DESCRIPTION),
        //     kind: class_kind_t_Sim_Class_Kind_Vanilla,
        // };

        info!("Creating class {}", Controller::CLASS_NAME);

        let cls = register_class(Controller::CLASS_NAME, class_data)?;

        // let cls =
        //     create_class(Controller::CLASS_NAME, class_info);

        let mut tracer = AFLCoverageTracer::get()?;
        output_config = tracer.on_initialize(input_config, output_config)?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        output_config = detector.on_initialize(input_config, output_config)?;
        drop(detector);

        register_interface::<_, confuse_module_interface_t>(cls, Controller::CLASS_NAME)?;

        info!(
            "Registered interface {}",
            confuse_module_interface_t::INTERFACE_NAME
        );

        // Next, we register callbacks for the two events we care about for this component:
        // - Core_Magic_Instruction: this lets us catch when we should pause to prep fuzzing
        //   and stop simulation for reset
        // - Core_Simulation_Stopped: If for some reason we don't get a magic stop, we need
        //   to know about other stops to handle simulation errors and normal exits.

        hap_add_callback_magic_instruction(callbacks::core_magic_instruction_cb)?;

        hap_add_callback_simulation_stopped(callbacks::core_simulation_stopped_cb)?;

        // The components initialized above may modify the initialized config, and after all of
        // them have a chance to do so we send the final config back to the client

        self.send_msg(ModuleMessage::Initialized(output_config))?;

        // We're the controller, so our config isn't used - we send it ourself just above
        Ok(OutputConfig::default())
    }

    unsafe fn pre_first_run(&mut self) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.pre_first_run()?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.pre_first_run()?;
        drop(detector);

        // We need to get our buffer information before we run for the first time so we can write
        // our testcases to it
        self.buffer = TargetBuffer {
            address: self
                .cpus
                .first()
                .context("No cpu present")?
                .borrow()
                .get_reg_value("rsi")?,
            size: self
                .cpus
                .first()
                .context("No cpu present")?
                .borrow()
                .get_reg_value("rdi")?,
        };

        self.take_snapshot()?;

        self.first_time_init_done = true;

        Ok(())
    }

    unsafe fn pre_run(
        &mut self,
        data: &[u8],
        _instance: Option<&mut ControllerInstance>,
    ) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.pre_run(data, Some(&mut self.instance.borrow_mut()))?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.pre_run(data, Some(&mut self.instance.borrow_mut()))?;
        drop(detector);

        Ok(())
    }

    unsafe fn on_reset(&mut self) -> Result<()> {
        // Before we tell our components we have reset, we need to actually do it
        match self.recv_msg()? {
            ClientMessage::Reset => {
                self.restore_snapshot()?;
            }
            ClientMessage::Exit => {
                self.quit_simulation();
            }
            _ => bail!("Unexpected message. Expected Reset"),
        }

        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_reset()?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.on_reset()?;
        drop(detector);

        self.send_msg(ModuleMessage::Ready)?;

        trace!("Sent ready message");

        match self.recv_msg()? {
            ClientMessage::Run(mut input) => {
                let buffer = self.buffer;
                input.truncate(buffer.size as usize);
                unsafe { self.pre_run(&input, None) }?;
                self.cpus
                    .first()
                    .context("No cpu available")?
                    .borrow()
                    .write_bytes(&buffer.address, &input)?;
            }
            ClientMessage::Exit => {
                self.quit_simulation();
            }
            _ => bail!("Unexpected message. Expected Run"),
        }

        continue_simulation();

        Ok(())
    }

    /// Callback when the simulation stops.
    unsafe fn on_stop(
        &mut self,
        reason: Option<StopReason>,
        _instance: Option<&mut ControllerInstance>,
    ) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_stop(reason.clone(), Some(&mut self.instance.borrow_mut()))?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.on_stop(reason.clone(), Some(&mut self.instance.borrow_mut()))?;
        drop(detector);

        match reason {
            None => {}
            Some(StopReason::Magic(magic)) => match magic {
                Magic::Start(_) => {
                    if self.first_time_init_done {
                        continue_simulation();
                    } else {
                        self.pre_first_run()?;
                        self.on_reset()?;
                    }
                }
                Magic::Stop((code, _)) => {
                    let val = self
                        .cpus
                        .first()
                        .context("No cpu available")?
                        .borrow()
                        .get_reg_value("rsi")?;
                    let magic = Magic::Stop((code, Some(val)));
                    trace!("Stopped with magic: {:?}", magic);
                    self.send_msg(ModuleMessage::Stopped(StopReason::Magic(magic)))?;
                    self.on_reset()?;
                }
            },
            Some(StopReason::Crash(fault)) => {
                self.send_msg(ModuleMessage::Stopped(StopReason::Crash(fault)))?;
                self.on_reset()?;
            }
            Some(StopReason::SimulationExit) => {
                self.send_msg(ModuleMessage::Stopped(StopReason::SimulationExit))?;
                self.on_reset()?;
            }
            Some(StopReason::TimeOut) => {
                self.send_msg(ModuleMessage::Stopped(StopReason::TimeOut))?;
                self.on_reset()?;
            }
        }

        self.stop_reason = None;

        Ok(())
    }
}

impl ComponentInterface for Controller {
    unsafe fn on_run(&mut self, _instance: &ControllerInstance) -> Result<()> {
        Ok(())
    }
    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_add_processor(obj, processor)?;

        let mut detector = FaultDetector::get()?;
        detector.on_add_processor(obj, processor)?;

        ensure!(
            self.cpus.is_empty(),
            "A CPU has already been added! This module only supports 1 vCPU at this time."
        );

        self.cpus.push(RefCell::new(Cpu::try_new(processor)?));

        Ok(())
    }

    unsafe fn on_add_fault(&mut self, obj: *mut conf_object_t, fault: i64) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_add_fault(obj, fault)?;

        let mut detector = FaultDetector::get()?;
        detector.on_add_fault(obj, fault)?;
        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
/// The interface for the controller
pub struct confuse_module_interface_t {
    run: unsafe extern "C" fn(obj: *mut conf_object_t),
    add_processor: unsafe extern "C" fn(obj: *mut conf_object_t, processor: *mut attr_value_t),
    add_fault: unsafe extern "C" fn(obj: *mut conf_object_t, fault: i64),
}

impl confuse_module_interface_t {
    // TODO: Can we autogenerate this with bindgen and tree-sitter?
    /// Write the C binding for this interface here
    pub const INTERFACE_NAME: &str = CLASS_NAME;
    pub const INTERFACE_TYPENAME: &str =
        concatcp!(confuse_module_interface_t::INTERFACE_NAME, "_interface_t");
}

impl Default for confuse_module_interface_t {
    fn default() -> Self {
        Self {
            run: callbacks::controller_interface_run,
            add_processor: callbacks::controller_interface_add_processor,
            add_fault: callbacks::controller_interface_add_fault,
        }
    }
}

/// This module contains all code that is invoked by SIMICS
mod callbacks {
    use super::{magic::Magic, Controller};
    use confuse_simics_api::{attr_value_t, conf_object_t};
    use log::trace;
    use std::ffi::{c_char, c_void};

    #[no_mangle]
    /// Invoked by SIMICs through the interface binding. This function signals the module to run
    pub extern "C" fn controller_interface_run(obj: *mut conf_object_t) {
        trace!("Interface call: run");
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe { controller.interface_run(obj) }.expect("Failed to trigger run");
    }

    #[no_mangle]
    pub extern "C" fn controller_interface_add_processor(
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) {
        trace!("Interface call: add_processor");
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe {
            controller
                .interface_add_processor(obj, processor)
                .expect("Failed to add processor")
        };
    }

    #[no_mangle]
    pub extern "C" fn controller_interface_add_fault(obj: *mut conf_object_t, fault: i64) {
        trace!("Interface call: add_fault");
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe {
            controller
                .interface_add_fault(obj, fault)
                .expect("Failed to add fault")
        };
    }

    #[no_mangle]
    pub extern "C" fn core_magic_instruction_cb(
        _user_data: *mut c_void,
        _trigger_obj: *const conf_object_t,
        parameter: i64,
    ) {
        if let Ok(magic) = Magic::try_from(parameter) {
            trace!("Got magic: {:?}", magic);
            let mut controller = Controller::get().expect("Could not get controller");
            controller
                .on_magic_instruction_cb(magic)
                .expect("Failed to handle magic instruction callback");
        }
    }

    #[no_mangle]
    pub extern "C" fn core_simulation_stopped_cb(
        _data: *mut c_void,
        _trigger_obj: *mut conf_object_t,
        // Exception is always SimExc_No_Exception
        _exception: i64,
        // Error string is always NULL
        _error_string: *mut c_char,
    ) {
        trace!("Simulation stopped");
        let mut controller = Controller::get().expect("Could not get controller");
        controller
            .on_simulation_stopped_cb()
            .expect("Failed to handle simulation stopped callback");
    }
}
