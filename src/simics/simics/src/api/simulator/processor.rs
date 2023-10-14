// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use simics_macro::simics_exception;

use crate::api::{
    sys::{
        tuple_int_string_t, SIM_current_clock, SIM_current_processor, SIM_disassemble_address,
        SIM_get_all_processors, SIM_get_processor, SIM_get_processor_number, SIM_number_processors,
        SIM_object_is_processor, SIM_processor_privilege_level, SIM_reset_processor,
    },
    AttrValue, ConfObject, GenericAddress,
};

pub type TupleIntString = tuple_int_string_t;

#[simics_exception]
pub fn object_is_processor(obj: *mut ConfObject) -> bool {
    unsafe { SIM_object_is_processor(obj) }
}

#[simics_exception]
pub fn number_processors() -> i32 {
    unsafe { SIM_number_processors() }
}

#[simics_exception]
pub fn reset_processor(cpu: *mut ConfObject, hard_reset: i32) {
    unsafe { SIM_reset_processor(cpu, hard_reset) }
}

#[simics_exception]
pub fn get_all_processors() -> AttrValue {
    unsafe { SIM_get_all_processors() }
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
pub fn current_processor() -> *mut ConfObject {
    unsafe { SIM_current_processor() }
}

#[simics_exception]
pub fn processor_privilege_level(cpu: *mut ConfObject) -> i32 {
    unsafe { SIM_processor_privilege_level(cpu) }
}

#[simics_exception]
pub fn disassemble_address(
    cpu: *mut ConfObject,
    address: GenericAddress,
    logical: i32,
    sub: i32,
) -> TupleIntString {
    unsafe { SIM_disassemble_address(cpu, address, logical, sub) }
}

#[simics_exception]
pub fn current_clock() -> *mut ConfObject {
    unsafe { SIM_current_clock() }
}
