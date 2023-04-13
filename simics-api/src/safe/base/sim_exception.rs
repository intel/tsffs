use simics_api_sys::SIM_last_error;
use std::ffi::CStr;

pub fn last_error() -> String {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}
