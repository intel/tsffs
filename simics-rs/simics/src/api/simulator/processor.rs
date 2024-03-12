// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Processor control

use crate::{
    simics_exception,
    sys::{
        tuple_int_string_t, SIM_current_clock, SIM_disassemble_address, SIM_get_all_processors,
        SIM_get_processor, SIM_get_processor_number, SIM_number_processors,
        SIM_object_is_processor, SIM_processor_privilege_level, SIM_reset_processor,
    },
    AttrValue, ConfObject, GenericAddress,
};

/// Alias for `tuple_int_string_t`
pub type TupleIntString = tuple_int_string_t;

#[simics_exception]
/// Check if an object is a processor
pub fn object_is_processor(obj: *mut ConfObject) -> bool {
    unsafe { SIM_object_is_processor(obj) }
}

#[simics_exception]
/// Get the number of processors in the simulation
pub fn number_processors() -> i32 {
    unsafe { SIM_number_processors() }
}

#[simics_exception]
/// Reset (hard or soft) a processor
pub fn reset_processor(cpu: *mut ConfObject, hard_reset: i32) {
    unsafe { SIM_reset_processor(cpu, hard_reset) }
}

#[simics_exception]
/// Get the list of all processors in the simulation
pub fn get_all_processors() -> AttrValue {
    unsafe { SIM_get_all_processors() }.into()
}

#[simics_exception]
/// Get the processor from its number
pub fn get_processor(number: i32) -> *mut ConfObject {
    unsafe { SIM_get_processor(number) }
}

#[simics_exception]
/// Get the number of a particular processor
pub fn get_processor_number(cpu: *mut ConfObject) -> i32 {
    unsafe { SIM_get_processor_number(cpu as *const ConfObject) }
}

#[simics_exception]
/// Get the processor's current privilege level
pub fn processor_privilege_level(cpu: *mut ConfObject) -> i32 {
    unsafe { SIM_processor_privilege_level(cpu) }
}

#[simics_exception]
/// Disassemble an address to a size and string
pub fn disassemble_address(
    cpu: *mut ConfObject,
    address: GenericAddress,
    logical: i32,
    sub: i32,
) -> TupleIntString {
    unsafe { SIM_disassemble_address(cpu, address, logical, sub) }
}

#[simics_exception]
/// Get the current active clock
pub fn current_clock() -> *mut ConfObject {
    unsafe { SIM_current_clock() }
}
