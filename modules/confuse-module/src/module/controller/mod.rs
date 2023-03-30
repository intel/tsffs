use self::{
    magic::{Magic, MagicCode},
    messages::{client::ClientMessage, module::ModuleMessage},
    target_buffer::TargetBuffer,
};
use super::{
    component::{Component, ComponentInterface},
    config::{InitializeConfig, InitializedConfig},
    cpu::Cpu,
    stop_reason::StopReason,
};
use crate::{
    module::{
        components::{detector::FaultDetector, tracer::AFLCoverageTracer},
        entrypoint::{BOOTSTRAP_SOCKNAME, CLASS_NAME, LOGLEVEL_VARNAME},
    },
    nonnull,
};
use anyhow::{bail, ensure, Context, Result};
use confuse_simics_api::{
    attr_value_t, class_data_t, class_info_t, class_kind_t_Sim_Class_Kind_Pseudo,
    class_kind_t_Sim_Class_Kind_Session, conf_object_t, micro_checkpoint_flags_t_Sim_MC_ID_User,
    micro_checkpoint_flags_t_Sim_MC_Persistent, CORE_discard_future, SIM_attr_list_size,
    SIM_break_simulation, SIM_continue, SIM_create_class, SIM_get_attribute, SIM_get_class,
    SIM_get_object, SIM_hap_add_callback, SIM_quit, SIM_register_class, SIM_register_interface,
    SIM_run_alone, VT_restore_micro_checkpoint, VT_save_micro_checkpoint,
};
use const_format::{concatcp, formatcp};
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
    ffi::{c_void, CString},
    mem::transmute,
    ptr::null_mut,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
};

pub mod fault;
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
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    log_handle: Handle,
    stop_reason: Option<StopReason>,
    buffer: TargetBuffer,
    first_time_init_done: bool,
    cpus: Vec<Box<RefCell<Cpu>>>,
}

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
            .encoder(Box::new(PatternEncoder::new("[SIMICS] {m}{n}")))
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
            tx,
            rx,
            log_handle,
            stop_reason: None,
            buffer: TargetBuffer::default(),
            first_time_init_done: false,
            cpus: vec![],
        })
    }

    /// Initialize the controller. This is called during module initialization after all
    /// components have been added
    pub fn initialize(&mut self) -> Result<()> {
        // Wait for an initialize message
        let initialize_config = match self.rx.recv()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        let initialized_config = InitializedConfig::default();

        _ = self.on_initialize(&initialize_config, initialized_config)?;

        Ok(())
    }

    pub unsafe fn take_snapshot(&mut self) -> Result<()> {
        unsafe {
            VT_save_micro_checkpoint(
                raw_cstr!("origin"),
                micro_checkpoint_flags_t_Sim_MC_ID_User
                    | micro_checkpoint_flags_t_Sim_MC_Persistent,
            )
        };

        info!("Took snapshot");

        let rexec = unsafe { SIM_get_object(raw_cstr!("sim.rexec")) };

        let sinfo = unsafe { SIM_get_attribute(rexec, raw_cstr!("state_info")) };

        let sinfo_size = SIM_attr_list_size(sinfo)?;

        ensure!(
            sinfo_size == 1,
            "Invalid size of state_info: {}",
            sinfo_size
        );

        Ok(())
    }

    pub unsafe fn quit_simulation(&mut self) {
        SIM_quit(0);
    }

    pub unsafe fn restore_snapshot(&mut self) -> Result<()> {
        unsafe {
            VT_restore_micro_checkpoint(0);
            CORE_discard_future();
        }

        Ok(())
    }

    /// Continue the simulation
    pub unsafe fn continue_simulation(&mut self) {
        SIM_run_alone(
            Some(transmute(SIM_continue as unsafe extern "C" fn(_) -> _)),
            null_mut(),
        );
    }

    /// Stop the simulation with some reason for stopping
    pub unsafe fn stop_simulation(&mut self, reason: StopReason) {
        trace!("Stopped with reason: {:?}", reason);
        self.stop_reason = Some(reason.clone());
        let reason_string = raw_cstr!(format!("{:?}", reason));
        SIM_break_simulation(reason_string);
    }
}

/// Implementation for methods the interface calls on us
impl Controller {
    /// Run the module
    pub unsafe fn interface_run(&mut self) -> Result<()> {
        self.continue_simulation();
        Ok(())
    }

    /// Add a processor to the controller
    pub unsafe fn interface_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        unsafe { self.on_add_processor(obj, processor) }
    }

    /// Add a fault to the controller
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
        unsafe { self.stop_simulation(StopReason::Magic(magic)) };
        Ok(())
    }

    /// Called by SIMICS on magic instruction
    pub fn on_simulation_stopped_cb(&mut self) -> Result<()> {
        let reason = self.stop_reason.clone();
        self.on_stop(reason)
    }
}

impl Component for Controller {
    /// Callback on initialization. For the controller, this is called directly, and it calls
    /// the on_initialize callbacks for all the other `Component`s
    fn on_initialize(
        &mut self,
        initialize_config: &InitializeConfig,
        mut initialized_config: InitializedConfig,
    ) -> Result<InitializedConfig> {
        // Before initializing ourself, we go ahead and initialize all of our components

        let mut tracer = AFLCoverageTracer::get()?;

        initialized_config = tracer.on_initialize(initialize_config, initialized_config)?;

        drop(tracer);

        let mut detector = FaultDetector::get()?;

        initialized_config = detector.on_initialize(initialize_config, initialized_config)?;

        drop(detector);

        // Now we register our class and interface

        let class_data = class_data_t {
            alloc_object: None,
            init_object: None,
            finalize_instance: None,
            pre_delete_instance: None,
            delete_instance: None,
            description: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
            class_desc: raw_cstr!(Controller::CLASS_DESCRIPTION),
            kind: class_kind_t_Sim_Class_Kind_Session,
        };

        info!("Creating class {}", Controller::CLASS_NAME);

        let cls = unsafe {
            SIM_register_class(
                raw_cstr!(Controller::CLASS_NAME),
                &class_data as *const class_data_t,
            )
        };

        ensure!(!cls.is_null(), "Unable to register class");

        // Create the interface to access this component through simics scripting
        let interface = Box::<confuse_module_controller_interface_t>::default();
        let interface = Box::into_raw(interface);

        ensure!(
            unsafe {
                SIM_register_interface(
                    cls,
                    raw_cstr!(confuse_module_controller_interface_t::INTERFACE_NAME),
                    interface as *mut _,
                )
            } == 0,
            "Could not register controller interface"
        );

        // Note: We do *NOT* want to free the interface, the allocated pointers are used directly
        // by simics, not copied

        info!(
            "Registered interface {}",
            confuse_module_controller_interface_t::INTERFACE_NAME
        );

        // Next, we register callbacks for the two events we care about for this component:
        // - Core_Magic_Instruction: this lets us catch when we should pause to prep fuzzing
        //   and stop simulation for reset
        // - Core_Simulation_Stopped: If for some reason we don't get a magic stop, we need
        //   to know about other stops to handle simulation errors and normal exits.

        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Magic_Instruction"),
                transmute(callbacks::core_magic_instruction_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };

        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Simulation_Stopped"),
                transmute(
                    callbacks::core_simulation_stopped_cb as unsafe extern "C" fn(_, _, _, _),
                ),
                null_mut(),
            )
        };

        // The components initialized above may modify the initialized config, and after all of
        // them have a chance to do so we send the final config back to the client

        self.tx
            .send(ModuleMessage::Initialized(initialized_config))?;

        // We're the controller, so our config isn't used - we send it ourself just above
        Ok(InitializedConfig::default())
    }

    fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.pre_run(data)?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.pre_run(data)?;
        drop(detector);

        Ok(())
    }

    fn on_reset(&mut self) -> Result<()> {
        // Before we tell our components we have reset, we need to actually do it
        match self.rx.recv()? {
            ClientMessage::Reset => {
                unsafe { self.restore_snapshot() }?;
            }
            ClientMessage::Stop => {
                unsafe { self.quit_simulation() };
            }
            _ => bail!("Unexpected message. Expected Reset"),
        }

        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_reset()?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.on_reset()?;
        drop(detector);

        self.tx.send(ModuleMessage::Ready)?;

        match self.rx.recv()? {
            ClientMessage::Run(mut input) => {
                let buffer = &self.buffer;
                input.truncate(buffer.size as usize);
                self.cpus
                    .first()
                    .context("No cpu available")?
                    .borrow()
                    .write_bytes(&buffer.address, &input)?;
            }
            ClientMessage::Stop => {
                unsafe { self.quit_simulation() };
            }
            _ => bail!("Unexpected message. Expected Run"),
        }

        unsafe { self.continue_simulation() };

        Ok(())
    }

    /// Callback when the simulation stops.
    fn on_stop(&mut self, reason: Option<StopReason>) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get()?;
        tracer.on_stop(reason.clone())?;
        drop(tracer);

        let mut detector = FaultDetector::get()?;
        detector.on_stop(reason.clone())?;
        drop(detector);

        match reason {
            None => {}
            Some(StopReason::Magic(magic)) => match magic {
                Magic::Start(_) => {
                    if self.first_time_init_done {
                        unsafe { self.continue_simulation() };
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
                    self.tx
                        .send(ModuleMessage::Stopped(StopReason::Magic(magic)))?;
                    self.on_reset()?;
                }
            },
            Some(StopReason::Crash(fault)) => {
                self.tx
                    .send(ModuleMessage::Stopped(StopReason::Crash(fault)))?;
                self.on_reset()?;
            }
            Some(StopReason::SimulationExit) => {
                self.tx
                    .send(ModuleMessage::Stopped(StopReason::SimulationExit))?;
                self.on_reset()?;
            }
            Some(StopReason::TimeOut) => {
                self.tx.send(ModuleMessage::Stopped(StopReason::TimeOut))?;
                self.on_reset()?;
            }
        }

        self.stop_reason = None;

        Ok(())
    }

    fn pre_first_run(&mut self) -> Result<()> {
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

        unsafe { self.take_snapshot() }?;

        self.first_time_init_done = true;

        Ok(())
    }
}

impl ComponentInterface for Controller {
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

        self.cpus
            .push(Box::new(RefCell::new(Cpu::try_new(processor)?)));

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
pub struct confuse_module_controller_interface_t {
    run: unsafe extern "C" fn(obj: *mut conf_object_t),
    add_processor: unsafe extern "C" fn(obj: *mut conf_object_t, processor: *mut attr_value_t),
    add_fault: unsafe extern "C" fn(obj: *mut conf_object_t, fault: i64),
}

impl confuse_module_controller_interface_t {
    // TODO: Can we autogenerate this with bindgen and tree-sitter?
    /// Write the C binding for this interface here
    pub const INTERFACE_NAME: &str = concatcp!(CLASS_NAME, "_controller");
    pub const INTERFACE_TYPENAME: &str = concatcp!(
        confuse_module_controller_interface_t::INTERFACE_NAME,
        "_interface_t"
    );
    pub const C_HEADER_BINDING: &str = formatcp!(
        r#"
            #ifndef CONFUSE_CONTROLLER_INTERFACE_H
            #define CONFUSE_CONTROLLER_INTERFACE_H

            #include <simics/device-api.h>
            #include <simics/pywrap.h>
            #include <simics/simulator-api.h> 

            SIM_INTERFACE({}) {{
                void (*run)(conf_object_t *obj);
                void (*add_processor)(conf_object_t *obj, attr_value_t *processor);
                void (*add_fault)(conf_object_t *obj, int64 fault);
            }};
            #define CONFUSE_MODULE_CONTROLLER_INTERFACE "{}"

            #endif /* ! CONFUSE_CONTROLLER_INTERFACE_H */
        "#,
        confuse_module_controller_interface_t::INTERFACE_NAME,
        confuse_module_controller_interface_t::INTERFACE_NAME
    );
    pub const DML_BINDING: &str = formatcp!(
        r#"
            dml 1.4;
            header %{{
                #include "{}.h"
            }}

            extern typedef struct {{
                void (*run)(conf_object_t *obj);
                void (*add_processor)(conf_object_t *obj, attr_value_t *processor);
                void (*add_fault)(conf_object_t *obj, int64 fault);
            }} {};
        "#,
        confuse_module_controller_interface_t::INTERFACE_NAME,
        confuse_module_controller_interface_t::INTERFACE_TYPENAME,
    );
}

impl Default for confuse_module_controller_interface_t {
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
    use super::{
        magic::{Magic, MagicCode},
        Controller,
    };
    use confuse_simics_api::{attr_value_t, conf_object_t};
    use log::{info, trace};
    use std::ffi::{c_char, c_void};

    #[no_mangle]
    /// Invoked by SIMICs through the interface binding. This function signals the module to run
    pub extern "C" fn controller_interface_run(_obj: *mut conf_object_t) {
        trace!("Interface call: run");
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe { controller.interface_run() }.expect("Failed to trigger run");
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
