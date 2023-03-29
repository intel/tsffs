use self::{
    magic::Magic,
    messages::{client::ClientMessage, module::ModuleMessage},
    target_buffer::TargetBuffer,
};
use super::{
    component::Component,
    config::{InitializeConfig, InitializedConfig},
    stop_reason::StopReason,
};
use crate::{
    module::{
        components::tracer::AFLCoverageTracer,
        entrypoint::{BOOTSTRAP_SOCKNAME, CLASS_NAME, LOGLEVEL_VARNAME},
    },
    nonnull,
};
use anyhow::{bail, ensure, Result};
use confuse_simics_api::{
    attr_value_t, class_data_t, class_info_t, class_kind_t_Sim_Class_Kind_Pseudo,
    class_kind_t_Sim_Class_Kind_Session, conf_object_t, SIM_break_simulation, SIM_continue,
    SIM_create_class, SIM_get_class, SIM_hap_add_callback, SIM_register_class,
    SIM_register_interface, SIM_run_alone,
};
use const_format::{concatcp, formatcp};
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use lazy_static::lazy_static;
use log::{info, Level, LevelFilter};
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

pub mod magic;
pub mod message;
pub mod messages;
mod target_buffer;

lazy_static! {
    pub static ref CONTROLLER: Arc<Mutex<Controller>> = Arc::new(Mutex::new(
        Controller::try_new().expect("Could not initialize Controller")
    ));
    pub static ref TRACER: Arc<Mutex<AFLCoverageTracer>> = Arc::new(Mutex::new(
        AFLCoverageTracer::try_new().expect("Could not initialize AFLCoverageTracer")
    ));
}

/// Controller for the Confuse simics module. The controller is reponsible for communicating with
/// the client, dispatching messages, and implementing the overall state machine for the module
pub struct Controller {
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
    log_handle: Handle,
    stop_reason: Option<StopReason>,
    buffer: Option<TargetBuffer>,
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
            buffer: None,
        })
    }

    /// Add a processor to the controller
    pub unsafe fn add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        unsafe { self.on_add_processor(obj, processor) }
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

    /// Called by SIMICS when the simulation stops
    pub fn on_stop_callback(&mut self) -> Result<()> {
        let reason = self.stop_reason.clone();
        self.on_stop(&reason)
    }

    /// Called when the stop reason is `StopReason::Magic`
    pub fn on_stop_magic(&mut self, magic: &Magic) -> Result<()> {
        match magic {
            Magic::Start => {
                // if self.buffer.is_none() {
                // } else {
                // }
            }
            Magic::Stop => {}
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
        let reason_string = raw_cstr!(format!("{:?}", reason));
        SIM_break_simulation(reason_string);
        self.stop_reason = Some(reason);
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
        // On initialize, the controller registers a class and an interface for configuration
        // through SIMICS scripting
        // let class_info = class_info_t {
        //     alloc: None,
        //     init: None,
        //     finalize: None,
        //     objects_finalized: None,
        //     deinit: None,
        //     dealloc: None,
        //     description: raw_cstr!(Controller::CLASS_DESCRIPTION),
        //     short_desc: raw_cstr!(Controller::CLASS_SHORT_DESCRIPTION),
        //     kind: class_kind_t_Sim_Class_Kind_Pseudo,
        // };

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
        let interface = Box::new(confuse_module_controller_interface_t::new());
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

        unsafe {
            // Free interface
            Box::from_raw(interface)
        };

        // Next, we register callbacks for the two events we care about for this component:
        // - Core_Magic_Instruction: this lets us catch when we should pause to prep fuzzing
        //   and stop simulation for reset
        // - Core_Simulation_Stopped: If for some reason we don't get a magic stop, we need
        //   to know about other stops to handle simulation errors and normal exits.

        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Magic_Instruction"),
                transmute(simics::core_magic_instruction_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };

        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Simulation_Stopped"),
                transmute(simics::core_simulation_stopped_cb as unsafe extern "C" fn(_, _, _, _)),
                null_mut(),
            )
        };

        // Finally, after initializing ourself, we go ahead and initialize all of our components

        let mut tracer = AFLCoverageTracer::get().expect("Could not get tracer");

        initialized_config = tracer.on_initialize(initialize_config, initialized_config)?;

        // The components may modify the initialized config, and after all of them have a chance
        // to do so we send the final config back to the client

        self.tx
            .send(ModuleMessage::Initialized(initialized_config))?;

        // We're the controller, so our config isn't used - we send it ourself just above
        Ok(InitializedConfig::default())
    }

    fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        todo!()
    }

    fn on_reset(&mut self) -> Result<()> {
        todo!()
    }

    /// Callback when the simulation stops.
    fn on_stop(&mut self, reason: &Option<StopReason>) -> Result<()> {
        match reason {
            None => {}
            Some(StopReason::Magic(magic)) => {
                self.on_stop_magic(magic)?;
            }
            Some(StopReason::Crash(fault)) => {
                self.tx
                    .send(ModuleMessage::Stopped(StopReason::Crash(*fault)))?;
            }
            Some(StopReason::SimulationExit) => {
                self.tx
                    .send(ModuleMessage::Stopped(StopReason::SimulationExit))?;
            }
            Some(StopReason::TimeOut) => {
                self.tx.send(ModuleMessage::Stopped(StopReason::TimeOut))?;
            }
        }
        Ok(())
    }

    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        let mut tracer = AFLCoverageTracer::get().expect("Could not get tracer");
        tracer.on_add_processor(obj, processor)?;

        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
/// The interface for the controller
pub struct confuse_module_controller_interface_t {
    run: unsafe extern "C" fn(obj: *mut conf_object_t),
    add_processor: unsafe extern "C" fn(obj: *mut conf_object_t, processor: *mut attr_value_t),
}

impl confuse_module_controller_interface_t {
    // TODO: Can we autogenerate this with bindgen and tree-sitter?
    /// Write the C binding for this interface here
    pub const INTERFACE_NAME: &str = concatcp!(CLASS_NAME, "_controller_interface");
    pub const C_HEADER_BINDING: &str = formatcp!(
        r#"
            #ifndef CONFUSE_CONTROLLER_INTERFACE_H
            #define CONFUSE_CONTROLLER_INTERFACE_H

            #include <simics/device-api.h>
            #include <simics/pywrap.h>
            #include <simics/simulator-api.h> 

            SIM_INTERFACE(confuse_controller) {{
                void (*run)(conf_object_t *obj);
                void (*add_processor)(conf_object_t *obj, attr_value_t *processor);
            }};
            #define CONFUSE_CONTROLLER_INTERFACE "{}"

            #endif /* ! CONFUSE_CONTROLLER_INTERFACE_H */
        "#,
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
            }} {}_t
        "#,
        confuse_module_controller_interface_t::INTERFACE_NAME,
        confuse_module_controller_interface_t::INTERFACE_NAME
    );

    pub fn new() -> Self {
        Self {
            run: simics::controller_interface_run,
            add_processor: simics::add_processor_cb,
        }
    }
}

/// This module contains all code that is invoked by SIMICS
mod simics {
    use std::ffi::{c_char, c_void};

    use crate::module::{component::Component, stop_reason::StopReason};

    use super::{magic::Magic, Controller};
    use confuse_simics_api::{attr_value_t, conf_object_t};

    #[no_mangle]
    /// Invoked by SIMICs through the interface binding. This function signals the module to run
    pub extern "C" fn controller_interface_run(obj: *mut conf_object_t) {
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe { controller.continue_simulation() };
    }

    #[no_mangle]
    pub extern "C" fn core_magic_instruction_cb(
        _user_data: *mut c_void,
        _trigger_obj: *const conf_object_t,
        parameter: i64,
    ) {
        match Magic::try_from(parameter) {
            Ok(Magic::Start) => {
                let mut controller = Controller::get().expect("Could not get controller");
                unsafe { controller.stop_simulation(StopReason::Magic(Magic::Start)) };
            }
            Ok(Magic::Stop) => {
                let mut controller = Controller::get().expect("Could not get controller");
                unsafe { controller.stop_simulation(StopReason::Magic(Magic::Start)) };
            }
            _ => {
                // Do nothing, there are lots of CPUID uses
            }
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
        let mut controller = Controller::get().expect("Could not get controller");
        controller
            .on_stop_callback()
            .expect("Failed to handle stop callback");
    }

    #[no_mangle]
    pub extern "C" fn add_processor_cb(obj: *mut conf_object_t, processor: *mut attr_value_t) {
        let mut controller = Controller::get().expect("Could not get controller");
        unsafe {
            controller
                .add_processor(obj, processor)
                .expect("Failed to add processor")
        };
    }
}
