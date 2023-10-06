// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::ffi::c_void;

use simics_api_sys::SIM_run_alone;

extern "C" fn run_alone_handler<F>(cb: *mut c_void)
where
    F: FnOnce() + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

pub fn run_alone<F>(cb: F)
where
    F: FnOnce() + 'static,
{
    let cb = Box::new(cb);
    let cb_box = Box::new(cb);
    let cb_raw = Box::into_raw(cb_box);

    debug_assert!(
        std::mem::size_of_val(&cb_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    unsafe {
        SIM_run_alone(
            Some(run_alone_handler::<F>),
            cb_raw as *mut _ as *mut c_void,
        )
    }
}
