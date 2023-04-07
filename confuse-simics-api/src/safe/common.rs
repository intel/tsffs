use crate::safe::wrapper::last_error;
use crate::{
    conf_object_t, safe::types::HapHandle, SIM_attr_list_size, SIM_continue, SIM_get_attribute,
    SIM_get_object, SIM_hap_add_callback, SIM_run_alone,
};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use std::ffi::{c_char, c_void, CString};
use std::{mem::transmute, ptr::null_mut};

pub fn continue_simulation() {
    unsafe {
        SIM_run_alone(
            Some(transmute(SIM_continue as unsafe extern "C" fn(_) -> _)),
            null_mut(),
        );
    }
}

pub fn count_micro_checkpoints() -> Result<u32> {
    let rexec = unsafe { SIM_get_object(raw_cstr!("sim.rexec")) };

    let sinfo = unsafe { SIM_get_attribute(rexec, raw_cstr!("state_info")) };

    let sinfo_size = SIM_attr_list_size(sinfo)?;

    Ok(sinfo_size)
}

fn hap_add_callback<S: AsRef<str>>(name: S, func: unsafe extern "C" fn()) -> Result<HapHandle> {
    let name_raw = raw_cstr!(name.as_ref());

    let handle = unsafe { SIM_hap_add_callback(name_raw, Some(func), null_mut()) };

    if handle == -1 {
        bail!("Error adding {} callback: {}", name.as_ref(), last_error());
    } else {
        Ok(handle)
    }
}

const HAP_CORE_MAGIC_INSTRUCTION: &str = "Core_Magic_Instruction";
const HAP_CORE_SIMULATION_STOPPED: &str = "Core_Simulation_Stopped";
const HAP_CORE_EXCEPTION: &str = "Core_Exception";

pub fn hap_add_callback_magic_instruction(
    func: unsafe extern "C" fn(*mut c_void, *const conf_object_t, i64),
) -> Result<HapHandle> {
    hap_add_callback(HAP_CORE_MAGIC_INSTRUCTION, unsafe { transmute(func) })
}

pub fn hap_add_callback_simulation_stopped(
    func: unsafe extern "C" fn(*mut c_void, *mut conf_object_t, i64, *mut c_char),
) -> Result<HapHandle> {
    hap_add_callback(HAP_CORE_SIMULATION_STOPPED, unsafe { transmute(func) })
}

pub fn hap_add_callback_core_exception(
    func: unsafe extern "C" fn(*mut c_void, *mut conf_object_t, i64),
) -> Result<HapHandle> {
    hap_add_callback(HAP_CORE_EXCEPTION, unsafe { transmute(func) })
}
