// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::ffi::c_void;

use simics_api_sys::SIM_run_alone;

extern "C" fn run_alone_handler<F>(cb: *mut c_void)
where
    F: FnMut(),
{
    let mut closure: Box<F> = unsafe { Box::from_raw(cb as *mut F) };
    closure()
}

pub fn run_alone<F>(cb: F)
where
    F: FnMut(),
{
    let cb = Box::new(cb);
    let cb = Box::into_raw(cb);
    unsafe { SIM_run_alone(Some(run_alone_handler::<F>), cb as *mut _ as *mut c_void) }
}
