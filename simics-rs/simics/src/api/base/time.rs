// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Time management APIs

use crate::{
    simics_exception,
    sys::{
        SIM_cycle_count, SIM_object_clock, SIM_picosecond_clock, SIM_stall, SIM_stall_count,
        SIM_stall_cycle, SIM_stalled_until, SIM_time,
    },
    ConfObject, Cycles,
};

#[simics_exception]
/// [`cycle_count`] returns the current simulated clock cycle count at obj.
///
/// If `obj` is a cycle counter implementing either the cycle_event interface or the cycle
/// interface, then the returned count is the number of elapsed cycles according to that
/// object. If obj is not a cycle counter, then the default clock associated with the
/// object is queried for its cycle count.
///
/// # Arguments
///
/// * `obj` - The object to get a cycle count for
///
/// # Return Value
///
/// The number of cycles for the object
///
/// # Context
///
/// Cell Context
pub fn cycle_count(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_cycle_count(obj) }
}

#[simics_exception]
/// Get the current time of an object.
///
/// # Arguments
///
/// * `obj` - The object to get a time for
///
/// # Return Value
///
/// The current time of the object
///
/// # Context
///
/// Unknown
pub fn current_time(obj: *mut ConfObject) -> f64 {
    unsafe { SIM_time(obj) }
}

#[simics_exception]
/// Stall an object's execution for a number of cycles. `obj` must implement the cycle and
/// stall interfaces
///
/// # Cycle
///
/// Unknown
pub fn stall_cycle(obj: *mut ConfObject, cycles: Cycles) {
    unsafe { SIM_stall_cycle(obj, cycles) }
}

#[simics_exception]
/// Stall an object's execution for a number of seconds. Must implement stall and cycle interfaces
///
/// # Cycle
///
/// Unknown
pub fn stall(obj: *mut ConfObject, seconds: f64) {
    unsafe { SIM_stall(obj, seconds) }
}

#[simics_exception]
/// Return the cycle number an object is stalled until
///
/// # Context
///
/// Cell Context
pub fn stalled_until(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_stalled_until(obj) }
}

#[simics_exception]
/// Return the number of cycles an object has been stalled
///
/// # Context
///
/// Cell Context
pub fn stall_count(obj: *mut ConfObject) -> Cycles {
    unsafe { SIM_stall_count(obj) }
}

#[simics_exception]
/// Retrieve the default clock used by an object. This is set by the queue attribute and
/// is used as time reference for the object.
///
/// # Arguments
///
/// * `obj` - The object to get the clock for
///
/// # Context
///
/// All Contexts
pub fn object_clock(obj: *const ConfObject) -> *mut ConfObject {
    unsafe { SIM_object_clock(obj as *const ConfObject) }
}

#[simics_exception]
/// Retrieve the picosecond clock used by an object.
///
/// The returned clock uses a cycle period of exactly 1 ps. It has full picosecond
/// resolution even if the processor (or clock) driving the simulation uses a lower
/// resolution. An event posted at a particular picosecond triggers always at that
/// precise time, without any rounding issues.
///
/// The returned object is the vtime.ps port object of the default clock for the object,
/// and it implements the cycle_event interface.
///
/// # Arguments
///
/// * `obj` - The object to get the clock for
///
/// # Context
///
/// All Contexts
pub fn picosecond_clock(obj: *mut ConfObject) -> *mut ConfObject {
    unsafe { SIM_picosecond_clock(obj) }
}
