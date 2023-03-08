include!(concat!(env!("OUT_DIR"), "/simics_module_header.rs"));

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

use std::{
    env::var,
    ffi::{c_void, CString},
    mem::transmute,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const AFL_MAPSIZE: usize = 64 * 1024;

macro_rules! nonnull {
    ($const_ptr:expr) => {{
        if $const_ptr.is_null() {
            error!("Pointer is NULL: $const_ptr");
            Err(anyhow!("Pointer is NULL: $const_ptr"))
        } else {
            Ok($const_ptr)
        }
    }};
}

/// Container for the SIMICS structures needed to trace execution of a processor
pub struct Processor {
    cpu: *mut conf_object_t,
    cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
    cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
    cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
    processor_info_v2: *mut processor_info_v2_interface_t,
    int_register: *mut int_register_interface_t,
}

impl Processor {
    pub fn try_new(
        cpu: *mut conf_object_t,
        // For information on these interfaces, see the "Model-to-simulator interfaces" part of the
        // documentation
        cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
        cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
        cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
        processor_info_v2: *mut processor_info_v2_interface_t,
        int_register: *mut int_register_interface_t,
    ) -> Result<Self> {
        Ok(Self {
            cpu: nonnull!(cpu)?,
            cpu_instrumentation_subscribe: nonnull!(cpu_instrumentation_subscribe)?,
            cpu_instrumentation_query: nonnull!(cpu_instrumentation_query)?,
            cpu_cached_instruction: nonnull!(cpu_cached_instruction)?,
            processor_info_v2: nonnull!(processor_info_v2)?,
            int_register: nonnull!(int_register)?,
        })
    }
}

/// NOTE: This macro leaks memory, do not use it hot!
macro_rules! raw_cstr {
    ($s:expr) => {
        CString::new($s)
            .expect(concat!(
                "Failed to initialize C string from ",
                stringify!($s)
            ))
            .into_raw()
    };
}

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
}

lazy_static! {
    static ref CTX: Arc<Mutex<ModuleCtx>> = {
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

#[no_mangle]
pub extern "C" fn init_local() {
    init_logging();
    let mut ctx = CTX.lock().expect("Could not lock context!");
    ctx.init().expect("Could not initialize context");
    info!("Initialized context for {}", CLASS_NAME);
}

// TODO: right now this will error if we add more than one processor, but this limitation
// can be removed with some effort
#[no_mangle]
/// Add processor to the branch tracer for tracing along with its associated instrumentation. Right
/// now it is an error to add more than one processor, but this is intended to be improved
pub extern "C" fn set_processor(_obj: *mut conf_object_t, val: *mut attr_value_t) -> set_error_t {
    info!("Adding processor to context");
    let cpu: *mut conf_object_t =
        unsafe { SIM_attr_object_or_nil(*val) }.expect("Attribute object expected");

    info!("Got CPU");

    let cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t = unsafe {
        SIM_c_get_interface(cpu, raw_cstr!("cpu_instrumentation_subscribe"))
            as *mut cpu_instrumentation_subscribe_interface_t
    };

    info!("Subscribed to CPU instrumentation");

    let cpu_instruction_query: *mut cpu_instruction_query_interface_t = unsafe {
        SIM_c_get_interface(cpu, raw_cstr!("cpu_instruction_query"))
            as *mut cpu_instruction_query_interface_t
    };

    info!("Got CPU query interface");

    let cpu_cached_instruction: *mut cpu_cached_instruction_interface_t = unsafe {
        SIM_c_get_interface(cpu, raw_cstr!("cpu_cached_instruction"))
            as *mut cpu_cached_instruction_interface_t
    };

    info!("Subscribed to cached instructions");

    let processor_info_v2: *mut processor_info_v2_interface_t = unsafe {
        SIM_c_get_interface(cpu, raw_cstr!("processor_info_v2"))
            as *mut processor_info_v2_interface_t
    };

    info!("Subscribed to processor info");

    let int_register: *mut int_register_interface_t = unsafe {
        SIM_c_get_interface(cpu, raw_cstr!("int_register")) as *mut int_register_interface_t
    };

    info!("Subscribed to internal register queries");

    let processor = Processor::try_new(
        cpu,
        cpu_instrumentation_subscribe,
        cpu_instruction_query,
        cpu_cached_instruction,
        processor_info_v2,
        int_register,
    )
    .expect("Could not initialize processor for tracing");

    let mut ctx = CTX.lock().expect("Could not lock context!");

    ctx.set_processor(processor)
        .expect("Could not add processor");

    set_error_t_Sim_Set_Ok
}

#[no_mangle]
pub extern "C" fn get_processor(_obj: *mut conf_object_t) -> attr_value_t {
    let ctx = CTX.lock().expect("Could not lock context!");
    let processor = ctx.processor.as_ref().expect("No processor");
    let cpu = processor.cpu;
    SIM_make_attr_object(cpu)
}

#[no_mangle]
pub extern "C" fn cached_instruction_callback(
    _obj: *const conf_object_t,
    cpu: *const conf_object_t,
    cached_instruction: *const cached_instruction_handle_t,
    instruction_query: *const instruction_handle_t,
    _user_data: *const c_void,
) {
    let ctx = CTX.lock().expect("Could not lock context!");
    info!("Cached instruction callback");
}

#[no_mangle]
pub extern "C" fn core_magic_instruction_cb(
    user_data: *mut c_void,
    trigger_obj: *const conf_object_t,
    parameter: i64,
) {
    info!("Got magic instruction");
    let ctx = CTX.lock().expect("Could not lock context!");
    let processor = ctx.processor.as_ref().expect("No processor");
    let cpu = processor.cpu;
    info!("Got processor");
    let rsi_number = unsafe {
        (*processor.int_register)
            .get_number
            .expect("No get_number function available")(cpu, raw_cstr!("rsi"))
    };

    info!("Got number for register rsi: {}", rsi_number);

    let rdi_number = unsafe {
        (*processor.int_register)
            .get_number
            .expect("No get_number function available")(
            processor.cpu as *mut conf_object_t,
            raw_cstr!("rdi"),
        )
    };

    info!("Got number for register rdi: {}", rsi_number);

    let rsi_value = unsafe {
        (*processor.int_register)
            .read
            .expect("No read function available")(
            processor.cpu as *mut conf_object_t, rsi_number
        )
    };

    info!("Got value for register rsi: {}", rsi_value);

    let rdi_value = unsafe {
        (*processor.int_register)
            .read
            .expect("No read function available")(
            processor.cpu as *mut conf_object_t, rdi_number
        )
    };

    info!("Got value for register rdi: {}", rdi_value);

    info!("Got magic instruction");
}
