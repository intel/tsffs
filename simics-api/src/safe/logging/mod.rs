use crate::ConfObject;
use anyhow::Result;
use simics_api_sys::{SIM_log_error, SIM_log_info, SIM_log_level, SIM_set_log_level};
use std::ffi::CString;

/// Log an info-level message through the SIMICS logging functions
pub fn log_info<S: AsRef<str>>(
    level: i32,
    device: *mut ConfObject,
    group: i32,
    msg: S,
) -> Result<()> {
    let msg_cstring = CString::new(msg.as_ref())?;

    unsafe {
        SIM_log_info(level, device.into(), group, msg_cstring.as_ptr());
    };

    Ok(())
}

/// Log an error-level message through the SIMICS logging functions
pub fn log_error(device: *mut ConfObject, group: i32, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_error(device.into(), group, msg_cstring.as_ptr());
    };

    Ok(())
}

/// Get the current log level of an object
pub fn log_level(obj: *mut ConfObject) -> u32 {
    unsafe { SIM_log_level((obj as *const ConfObject).into()) }
}

/// Set the global SIMICS log level
pub fn set_log_level(obj: *mut ConfObject, level: u32) {
    unsafe { SIM_set_log_level(obj.into(), level) };
}
