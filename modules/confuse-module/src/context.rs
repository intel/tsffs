use anyhow::{bail, ensure, Context, Result};
use confuse_fuzz::message::{FuzzerEvent, Message, SimicsEvent};
use confuse_simics_api::{
    attr_attr_t_Sim_Attr_Pseudo, class_data_t, class_kind_t_Sim_Class_Kind_Session, conf_class,
    conf_class_t, conf_object_t, micro_checkpoint_flags_t_Sim_MC_ID_User,
    micro_checkpoint_flags_t_Sim_MC_Persistent, SIM_attr_integer, SIM_attr_list_size,
    SIM_get_attribute, SIM_get_object, SIM_get_port_interface, SIM_hap_add_callback,
    SIM_register_attribute, SIM_register_class, VT_restore_micro_checkpoint,
    VT_save_micro_checkpoint, CORE_discard_future
};
use const_format::concatcp;

use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use ipc_shm::{IpcShm, IpcShmWriter};
use lazy_static::lazy_static;
use log::info;
use raw_cstr::raw_cstr;

use crate::callbacks::{
    core_simulation_stopped_cb, get_processor, get_signal, resume_simulation, set_processor,
    set_signal,
};

use crate::magic::Magic;
use crate::processor::Processor;
use crate::signal::Signal;
use crate::stop_action::StopReason;
use crate::{
    callbacks::core_magic_instruction_cb,
    interface::{BOOTSTRAP_SOCKNAME, CLASS_NAME},
};

use std::ffi::c_void;
use std::{
    env::var,
    ffi::CString,
    mem::transmute,
    ptr::null_mut,
    sync::{Arc, Mutex},
};
pub const AFL_MAPSIZE: usize = 64 * 1024;

/// Container for the SIMICS structures needed to trace execution of a processor
/// Context for the module. This module is responsible for:
/// - Handling messages from SIMICS
/// - Branch tracing
/// - Detecting errors
pub struct ModuleCtx {
    cls: *mut conf_class,
    tx: IpcSender<SimicsEvent>,
    rx: IpcReceiver<FuzzerEvent>,
    shm: IpcShm,
    writer: IpcShmWriter,
    processor: Option<Processor>,
    stop_reason: Option<StopReason>,
}

unsafe impl Send for ModuleCtx {}
unsafe impl Sync for ModuleCtx {}

impl ModuleCtx {
    pub fn try_new(cls: *mut conf_class) -> Result<Self> {
        let bootstrap = IpcSender::connect(var(BOOTSTRAP_SOCKNAME)?)?;

        info!("Bootstrapped connection for IPC");

        let (otx, rx) = channel::<FuzzerEvent>()?;
        let (tx, orx) = channel::<SimicsEvent>()?;

        info!("Sending fuzzer IPC channel");

        bootstrap.send((otx, orx))?;

        info!("Waiting for initialize command");

        ensure!(
            matches!(rx.recv()?, FuzzerEvent::Initialize),
            "Did not receive Initialize command."
        );

        let mut shm = IpcShm::default();

        let mut writer = shm.writer()?;

        for i in 0..writer.len() {
            writer.write_at(&[(i % u8::MAX as usize) as u8], i)?;
        }

        info!("Sending fuzzer memory map");

        tx.send(SimicsEvent::SharedMem(shm.try_clone()?))?;

        Ok(Self {
            cls,
            tx,
            rx,
            shm,
            writer,
            processor: None,
            stop_reason: None,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        info!("Initialized Module Context");

        Ok(())
    }

    pub fn set_processor(&mut self, processor: Processor) -> Result<()> {
        self.processor = Some(processor);
        Ok(())
    }

    pub fn get_processor(&self) -> Result<&Processor> {
        Ok(self.processor.as_ref().context("No processor available")?)
    }

    pub fn handle_signal(&self, signal: Signal) {
        match signal {
            Signal::Start => self.start(),
            _ => {}
        }
    }

    pub fn handle_stop(&mut self) -> Result<()> {
        match &self.stop_reason {
            Some(StopReason::Magic(m)) => match m {
                Magic::Start => {
                    // Start harness stop means we need to take a snapshot!
                    unsafe {
                        VT_save_micro_checkpoint(
                            raw_cstr!("origin"),
                            micro_checkpoint_flags_t_Sim_MC_ID_User
                                | micro_checkpoint_flags_t_Sim_MC_Persistent,
                        )
                    };

                    info!("Took snapshot");

                    self.tx.send(SimicsEvent::Ready)?;

                    // We'll wait for a signal to start
                    match self.rx.recv()? {
                        FuzzerEvent::Run => {
                            unsafe { resume_simulation() };
                        }
                        _ => {
                            bail!("Unexpected event");
                        }
                    }
                }
                Magic::Stop => {
                    // Stop harness stop means we need to reset to the snapshot and be ready to
                    // run
                    self.tx.send(SimicsEvent::Stopped)?;

                    let rexec = unsafe { SIM_get_object(raw_cstr!("sim.rexec")) };

                    let sinfo = unsafe { SIM_get_attribute(rexec, raw_cstr!("state_info")) };

                    let sinfo_size = SIM_attr_list_size(sinfo);

                    ensure!(
                        sinfo_size == 1,
                        "Invalid size of state_info: {}",
                        sinfo_size
                    );
                    info!("Waiting for reset signal to restore state");

                    match self.rx.recv()? {
                        FuzzerEvent::Reset => {
                            unsafe { VT_restore_micro_checkpoint(0) };
                            unsafe { CORE_discard_future() };

                            info!("Restored checkpoint");
                        }
                        FuzzerEvent::Stop => {
                            info!("Got stop signal, we want to stop cleanly here");
                        }
                        _ => {
                            bail!("Unexpected event");
                        }
                    }

                    self.tx.send(SimicsEvent::Ready)?;

                    // We'll wait for a signal to start
                    match self.rx.recv()? {
                        FuzzerEvent::Run => {
                            unsafe { resume_simulation() };
                        }
                        FuzzerEvent::Stop => {
                            info!("Got stop signal, we want to stop cleanly here");
                        }
                        _ => {
                            bail!("Unexpected event");
                        }
                    }
                }
            },
            None => {}
        }

        self.stop_reason = None;

        Ok(())
    }

    pub fn set_stopped_reason(&mut self, reason: Option<StopReason>) -> Result<()> {
        self.stop_reason = reason;
        Ok(())
    }

    pub fn start(&self) {
        info!("Starting module");
        unsafe { resume_simulation() };
    }
}

lazy_static! {
    pub static ref CTX: Arc<Mutex<ModuleCtx>> = {

        // reference-manual-api/device-api-data-types.html
        let class_data = class_data_t {
            alloc_object: None,
            init_object: None,
            finalize_instance: None,
            pre_delete_instance: None,
            delete_instance: None,
            // Leaked
            description: raw_cstr!(CLASS_NAME),
            // Leaked
            class_desc: raw_cstr!("Confuse module"),
            kind: class_kind_t_Sim_Class_Kind_Session,
        };

        let cls: *mut conf_class_t  = unsafe {
            // Class name Leaked
            SIM_register_class(raw_cstr!(CLASS_NAME), &class_data as *const class_data_t)
        };

        unsafe {
            SIM_register_attribute(
                cls,
                raw_cstr!("processor"),
                Some(get_processor),
                Some(set_processor),
                attr_attr_t_Sim_Attr_Pseudo,
                // https://docs.python.org/3/c-api/arg.html#parsing-arguments
                raw_cstr!("o|n"),
                raw_cstr!("The <i>processor</i> to trace."),
            );
        };

        unsafe {
            SIM_register_attribute(
                cls,
                raw_cstr!("signal"),
                Some(get_signal),
                Some(set_signal),
                attr_attr_t_Sim_Attr_Pseudo,
                raw_cstr!("i"),
                raw_cstr!("Pseudo interface for sending a signal"),
            );
        };

        info!("Registered processor attribute");

        let _magic_cb_handle = unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Magic_Instruction"),
                transmute(core_magic_instruction_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };

        let _stop_cb_handle = unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Simulation_Stopped"),
                transmute(core_simulation_stopped_cb as unsafe extern "C" fn(_, _, _, _)),
                null_mut(),
            )
        };

        info!("Added callback for magic instruction");


        Arc::new(Mutex::new(
            ModuleCtx::try_new(cls).expect("Failed to initialize module"),
        ))
    };
}

/*

// TODO: Don't use global state, instead we should follow simics' methodology and use the object
// format it defines:

#[no_mangle]
pub extern "C" fn alloc_ctx(_data: *mut c_void) -> *mut conf_object_t {
    info!("Alloc called");
    null_mut()
}

#[no_mangle]
pub extern "C" fn init_ctx(_obj: *mut conf_object_t, _data: *mut c_void) -> *mut c_void {
    info!("Init called");
    null_mut()
}

#[no_mangle]
pub extern "C" fn finalize_ctx(_obj: *mut conf_object_t) {
    info!("Finalize called");
}

#[no_mangle]
pub extern "C" fn pre_delete_ctx(_obj: *mut conf_object_t) {
    info!("Pre delete called");
}

#[no_mangle]
pub extern "C" fn delete_instance_ctx(_obj: *mut conf_object_t) -> i32 {
    info!("Delete called");
    0
}

*/
