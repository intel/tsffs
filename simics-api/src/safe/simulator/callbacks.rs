use std::ffi::c_void;

use simics_api_sys::SIM_run_alone;

extern "C" fn run_alone_handler(cb: *mut c_void) {
    let closure: &mut &mut dyn FnMut() = unsafe { &mut *(cb as *mut &mut dyn std::ops::FnMut()) };
    closure()
}

pub fn run_alone<F>(mut cb: F)
where
    F: FnMut(),
{
    let mut cb: &mut dyn FnMut() = &mut cb;
    let cb = &mut cb;
    unsafe { SIM_run_alone(Some(run_alone_handler), cb as *mut _ as *mut c_void) }
}
