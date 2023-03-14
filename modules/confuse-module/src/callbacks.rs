//! Callback handlers from SIMICS to the module

use anyhow::Result;
use confuse_fuzz::Fault;
use confuse_simics_api::{
    attr_value_t, cached_instruction_handle_t, conf_object_t, cpu_cached_instruction_interface_t,
    cpu_instruction_query_interface_t, cpu_instrumentation_subscribe_interface_t,
    exception_interface_t, instruction_handle_t, int_register_interface_t,
    processor_info_v2_interface_t, set_error_t, set_error_t_Sim_Set_Ok, SIM_attr_integer,
    SIM_attr_object_or_nil, SIM_break_simulation, SIM_c_get_interface, SIM_continue,
    SIM_make_attr_object, SIM_run_alone,
};
use log::{debug, error, info};
use raw_cstr::raw_cstr;
use std::{
    ffi::{c_char, c_void, CString},
    mem::transmute,
    ptr::null_mut,
};

use crate::{
    context::CTX, magic::Magic, processor::Processor, signal::Signal, stop_reason::StopReason,
};

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

    if let Some(register) = unsafe { *cpu_instrumentation_subscribe }.register_cached_instruction_cb
    {
        unsafe {
            register(
                cpu,
                null_mut(),
                Some(cached_instruction_callback),
                null_mut(),
            )
        };
    }

    let exception: *mut exception_interface_t =
        unsafe { SIM_c_get_interface(cpu, raw_cstr!("exception")) as *mut exception_interface_t };

    info!("Subscribed to exception queries");

    let processor = Processor::try_new(
        cpu,
        cpu_instrumentation_subscribe,
        cpu_instruction_query,
        cpu_cached_instruction,
        processor_info_v2,
        int_register,
        exception,
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
    _obj: *mut conf_object_t,
    cpu: *mut conf_object_t,
    _cached_instruction: *mut cached_instruction_handle_t,
    instruction_query: *mut instruction_handle_t,
    _user_data: *mut c_void,
) {
    let mut ctx = CTX.lock().expect("Could not lock context!");
    let processor = ctx.get_processor().expect("Could not get processor");
    match processor.is_branch(cpu, instruction_query) {
        Ok(Some(pc)) => {
            ctx.log(pc).expect("Failed to log pc");
        }
        Err(e) => {
            error!("Error checking whether instruction is branch: {}", e);
        }
        _ => {}
    }
}

pub unsafe fn resume_simulation() {
    SIM_run_alone(
        Some(transmute(SIM_continue as unsafe extern "C" fn(_) -> _)),
        null_mut(),
    );
}

#[no_mangle]
pub extern "C" fn core_magic_instruction_cb(
    _user_data: *mut c_void,
    _trigger_obj: *const conf_object_t,
    parameter: i64,
) {
    match Magic::try_from(parameter) {
        Ok(Magic::Start) => {
            let mut ctx = CTX.lock().expect("Could not lock context!");

            unsafe { SIM_break_simulation(raw_cstr!("Stopping for snapshot")) };

            info!("Stopped simulation");

            ctx.set_stopped_reason(Some(StopReason::Magic(Magic::Start)))
                .expect("Couldn't set stop reason");

            // Send ready event
        }
        Ok(Magic::Stop) => {
            info!("Got magic stop signal, stopping simulation");
            let mut ctx = CTX.lock().expect("Could not lock context!");
            unsafe { SIM_break_simulation(raw_cstr!("Stopping to restore snapshot")) };
            ctx.set_stopped_reason(Some(StopReason::Magic(Magic::Stop)))
                .expect("Couldn't set stop reason");

            // Send stopped event
        }
        _ => {
            // info!("Unrecognized magic parameter: {}", parameter);
        }
    }
}

#[no_mangle]
pub extern "C" fn set_signal(_obj: *mut conf_object_t, val: *mut attr_value_t) -> set_error_t {
    let signal = Signal::try_from(unsafe { SIM_attr_integer(*val).expect("Error getting value") })
        .expect("No such signal");
    info!("Got signal {:?}", signal);
    let ctx = CTX.lock().expect("Could not lock context!");
    ctx.handle_signal(signal);
    set_error_t_Sim_Set_Ok
}

#[no_mangle]
pub extern "C" fn get_signal(_obj: *mut conf_object_t) -> attr_value_t {
    info!("Signal retrieved (no-op");
    SIM_make_attr_object(null_mut())
}

#[no_mangle]
pub extern "C" fn core_simulation_stopped_cb(
    _data: *mut c_void,
    _trigger_obj: *mut conf_object_t,
    _exception: i64,
    _error_string: *mut c_char,
) {
    info!("Simulation has stopped");
    let mut ctx = CTX.lock().expect("Could not lock context!");
    ctx.handle_stop().expect("Failed to handle stop");
}

fn handle_fault(fault: Fault) -> Result<()> {
    let mut ctx = CTX.lock().expect("Could not lock context!");

    if ctx.is_fault(fault) {
        // Only stop the simulation if this is a legit fault
        unsafe { SIM_break_simulation(raw_cstr!("Stopping to report crash")) };

        debug!("Crash detected, reporting!");

        ctx.set_stopped_reason(Some(StopReason::Crash))?;
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn core_exception_cb(
    _data: *mut c_void,
    _trigger_obj: *mut conf_object_t,
    exception_number: i64,
) {
    // Exception! If it's one we care about, we want to break which according to the docs:

    // Interrupting the simulation by calling SIM_break_simulation inside the hap will cause the
    // simulation to stop right before the exception (and the trapping instruction, if any). The
    // simulation state will then be as it was prior to the execution of the instruction or
    // exception. Continuing the simulation will then re-run the exception, this time without
    // calling hap functions.

    if let Ok(fault) = Fault::try_from(exception_number) {
        handle_fault(fault).expect("Could not handle fault");
    }
}

#[no_mangle]
pub extern "C" fn x86_triple_fault_cb(_data: *mut c_void, _trigger_obj: *mut conf_object_t) {
    handle_fault(Fault::Triple).expect("Could not handle triple fault");
}
