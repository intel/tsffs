// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::{mm_free, mm_zalloc};
use std::{ffi::c_void, mem::transmute};

#[macro_export]
/// Allocate memory with a size, of some type
macro_rules! simics_alloc {
    ($typ:ty, $sz:expr) => {
        $crate::api::alloc($sz, stringify!($typ), file!(), line!() as i32)
    };
}

/// Allocate using the SIMICS zalloc implementation
pub fn alloc<T, S: AsRef<str>>(
    size: usize,
    typename: S,
    filename: S,
    line_number: i32,
) -> Result<*mut T> {
    unsafe {
        let res = mm_zalloc(
            size,
            size,
            raw_cstr(typename)?,
            raw_cstr(filename)?,
            line_number,
        );
        Ok(transmute(res))
    }
}

/// Free a pointer that was allocated with [`alloc`]
pub fn free<T>(ptr: *mut T) {
    unsafe { mm_free(ptr as *mut c_void) };
}
