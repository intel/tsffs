use simics_api_sys::SIM_version;

pub fn version() -> String {
    let c_str = CStr::from_ptr(unsafe { SIM_version() });
}
