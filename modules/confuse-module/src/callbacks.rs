//! Callback handlers from SIMICS to the module

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

use std::{
    env::var,
    ffi::{c_void, CString},
    mem::transmute,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

use crate::{context::CTX, processor::Processor};

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
    let processor = ctx.get_processor().expect("No processor");
    let cpu = processor.get_cpu();
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
    let processor = ctx.get_processor().expect("No processor");
    let cpu = processor.get_cpu();
    info!("Got processor");
    let rsi_number = unsafe {
        (*processor.get_int_register())
            .get_number
            .expect("No get_number function available")(cpu, raw_cstr!("rsi"))
    };

    info!("Got number for register rsi: {}", rsi_number);

    let rdi_number = unsafe {
        (*processor.get_int_register())
            .get_number
            .expect("No get_number function available")(
            processor.get_cpu(), raw_cstr!("rdi")
        )
    };

    info!("Got number for register rdi: {}", rdi_number);

    let rsi_value = unsafe {
        (*processor.get_int_register())
            .read
            .expect("No read function available")(processor.get_cpu(), rsi_number)
    };

    info!("Got value for register rsi: {}", rsi_value);

    let rdi_value = unsafe {
        (*processor.get_int_register())
            .read
            .expect("No read function available")(processor.get_cpu(), rdi_number)
    };

    info!("Got value for register rdi: {}", rdi_value);

    info!("Got magic instruction");
}
