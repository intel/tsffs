use anyhow::{anyhow, bail, ensure, Context, Result};
use confuse_fuzz::message::{FuzzerEvent, Message, SimicsEvent};
use confuse_simics_api::{
    attr_attr_t_Sim_Attr_Pseudo, attr_value_t, cached_instruction_handle_t, class_data_t,
    class_kind_t_Sim_Class_Kind_Session, conf_class, conf_class_t, conf_object_t,
    cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, instruction_handle_t, int_register_interface_t,
    obj_hap_func_t, processor_info_v2_interface_t, set_error_t, set_error_t_Sim_Set_Ok,
    SIM_attr_object_or_nil, SIM_c_get_interface, SIM_hap_add_callback, SIM_make_attr_object,
    SIM_register_attribute, SIM_register_class,
};
use const_format::concatcp;
use env_logger::init as init_logging;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use ipc_shm::{IpcShm, IpcShmWriter};
use lazy_static::lazy_static;
use log::{error, info};
use raw_cstr::raw_cstr;

use crate::callbacks::{get_processor, set_processor};
use crate::nonnull;

use crate::processor::Processor;
use crate::{callbacks::core_magic_instruction_cb, interface::CLASS_NAME};

use std::{
    env::var,
    ffi::{c_void, CString},
    mem::transmute,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};
pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const AFL_MAPSIZE: usize = 64 * 1024;

/// Container for the SIMICS structures needed to trace execution of a processor
/// Context for the module. This module is responsible for:
/// - Handling messages from SIMICS
/// - Branch tracing
/// - Detecting errors
pub struct ModuleCtx {
    cls: *mut conf_class,
    tx: IpcSender<Message>,
    rx: IpcReceiver<Message>,
    shm: IpcShm,
    writer: IpcShmWriter,
    processor: Option<Processor>,
}

unsafe impl Send for ModuleCtx {}
unsafe impl Sync for ModuleCtx {}

impl ModuleCtx {
    pub fn try_new(cls: *mut conf_class) -> Result<Self> {
        let bootstrap = IpcSender::connect(var(BOOTSTRAP_SOCKNAME)?)?;

        info!("Bootstrapped connection for IPC");

        let (otx, rx) = channel::<Message>()?;
        let (tx, orx) = channel::<Message>()?;

        info!("Sending fuzzer IPC channel");

        bootstrap.send((otx, orx))?;

        info!("Waiting for initialize command");

        ensure!(
            matches!(rx.recv()?, Message::FuzzerEvent(FuzzerEvent::Initialize)),
            "Did not receive Initialize command."
        );

        let mut shm = IpcShm::default();

        let mut writer = shm.writer()?;

        for i in 0..writer.len() {
            writer.write_at(&[(i % u8::MAX as usize) as u8], i)?;
        }

        info!("Sending fuzzer memory map");

        tx.send(Message::SimicsEvent(SimicsEvent::SharedMem(
            shm.try_clone()?,
        )))?;

        Ok(Self {
            cls,
            tx,
            rx,
            shm,
            writer,
            processor: None,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        let _start_cb_handle = unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Magic_Instruction"),
                transmute(core_magic_instruction_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };

        info!("Added callback for magic instruction");

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
}

lazy_static! {
    pub static ref CTX: Arc<Mutex<ModuleCtx>> = {
        let class_name: CString = CString::new(CLASS_NAME).expect("CString::new failed");
        let class_data_desc = CString::new("Minimal module").expect("CString::new failed");
        let class_data_class_desc =
            CString::new("Minimal module class").expect("CString::new failed");

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
            SIM_register_class(class_name.into_raw(), &class_data as *const class_data_t)
        };

        unsafe {
            SIM_register_attribute(
                cls,
                raw_cstr!("processor"),
                Some(get_processor),
                Some(set_processor),
                attr_attr_t_Sim_Attr_Pseudo,
                raw_cstr!("o|n"),
                raw_cstr!("The <i>processor</i> to trace."),
            );
        };

        info!("Registered processor attribute");

        Arc::new(Mutex::new(
            ModuleCtx::try_new(cls).expect("Failed to initialize module"),
        ))
    };
}
