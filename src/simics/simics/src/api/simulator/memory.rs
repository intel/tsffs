// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::api::sys::{SIM_read_byte, SIM_write_byte};
use crate::api::ConfObject;
use simics_macro::simics_exception;

#[simics_exception]
/// Write a byte to a physical address
pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
    unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
}

#[simics_exception]
/// Read a byte from a physical address
pub fn read_byte(physical_memory: *mut ConfObject, physical_addr: u64) -> u8 {
    unsafe { SIM_read_byte(physical_memory, physical_addr) }
}
