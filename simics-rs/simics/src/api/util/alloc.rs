// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Allocator functionality used by Simics and which should be used by modules importing Simics
//! functionality to ensure no conflicts

use crate::{
    sys::{mm_free, mm_realloc, mm_zalloc},
    Result,
};
use raw_cstr::raw_cstr;
use std::{alloc::GlobalAlloc, ffi::c_void, mem::transmute};

#[macro_export]
/// Allocate memory with a size, of some type
macro_rules! simics_alloc {
    ($typ:ty, $sz:expr) => {
        $crate::api::alloc($sz, stringify!($typ), file!(), line!() as i32)
    };
}

/// Allocate using the SIMICS zalloc implementation
///
/// # Context
///
/// All Contexts
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
///
/// # Context
///
/// All Contexts
pub fn free<T>(ptr: *mut T) {
    unsafe { mm_free(ptr as *mut c_void) };
}

/// Global allocator that uses SIMICS' exported memory management functionality
pub struct SimicsAlloc;

// Unfortunately we don't have a good way to access the caller from global alloc, so we
// provide dummy values for these fields
const SIMICS_ALLOC_TYPE: &[u8; 8] = b"unknown\0";
const SIMICS_ALLOC_FILE: &[u8; 8] = b"unknown\0";
const SIMICS_ALLOC_LINE: i32 = 0;

unsafe impl GlobalAlloc for SimicsAlloc {
    /// Allocate using the global SIMICS allocator. Note: this allocation function
    /// may fail in circumstances where very unusual alignment is requried.
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let size = layout.size();

        unsafe {
            mm_zalloc(
                size,
                1,
                SIMICS_ALLOC_TYPE.as_ptr() as *const i8,
                SIMICS_ALLOC_FILE.as_ptr() as *const i8,
                SIMICS_ALLOC_LINE,
            ) as *mut u8
        }
    }

    /// All allocations are zeroed, so this method calls through to `alloc`
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.alloc(layout)
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        _layout: std::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        unsafe {
            mm_realloc(
                ptr as *mut c_void,
                new_size,
                1,
                SIMICS_ALLOC_TYPE.as_ptr() as *const i8,
                SIMICS_ALLOC_FILE.as_ptr() as *const i8,
                SIMICS_ALLOC_LINE,
            ) as *mut u8
        }
    }

    /// Deallocate using the global SIMICS allocator. Note: this deallocation function
    /// may fail in circumstances where very unusual alignment is required.
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: std::alloc::Layout) {
        unsafe { mm_free(ptr as *mut c_void) };
    }
}
