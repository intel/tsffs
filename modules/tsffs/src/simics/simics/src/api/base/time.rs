// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Time management APIs

use crate::api::{
    sys::{
        SIM_cycle_count, SIM_object_clock, SIM_picosecond_clock, SIM_stall, SIM_stall_count,
        SIM_stall_cycle, SIM_stalled_until, SIM_time,
    },
    ConfObject, Cycles,
};
use simics_macro::simics_exception;

#[simics_exception]
/// Get the current cycle count of an object
pub fn cycle_count(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_cycle_count(obj) }
}

#[simics_exception]
/// Get the current time of an object.
pub fn current_time(obj: *mut ConfObject) -> f64 {
    unsafe { SIM_time(obj) }
}

#[simics_exception]
/// Stall an object's execution for a number of cycles. `obj` must implement the cycle and
/// stall interfaces
pub fn stall_cycle(obj: *mut ConfObject, cycles: Cycles) {
    unsafe { SIM_stall_cycle(obj, cycles) }
}

#[simics_exception]
/// Stall an object's execution for a number of seconds. Must implement stall and cycle interfaces
pub fn stall(obj: *mut ConfObject, seconds: f64) {
    unsafe { SIM_stall(obj, seconds) }
}

#[simics_exception]
/// Return the cycle number an object is stalled until
pub fn stalled_until(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_stalled_until(obj) }
}

#[simics_exception]
/// Return the number of cycles an object has been stalled
pub fn stall_count(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_stall_count(obj) }
}

#[simics_exception]
/// Get the clock of an object that implements the required clock interface
pub fn object_clock(obj: *const ConfObject) -> *mut ConfObject {
    unsafe { SIM_object_clock(obj as *const ConfObject) }
}

#[simics_exception]
/// Return the picosecond clock used by an object
pub fn picosecond_clock(obj: *mut ConfObject) -> *mut ConfObject {
    unsafe { SIM_picosecond_clock(obj) }
}
