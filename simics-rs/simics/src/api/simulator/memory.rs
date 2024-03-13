// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Memory access APIs

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    simics_exception,
    sys::{
        SIM_load_binary, SIM_load_file, SIM_read_byte, SIM_read_phys_memory,
        SIM_read_phys_memory_tags, SIM_write_byte, SIM_write_phys_memory,
        SIM_write_phys_memory_tags,
    },
    ConfObject, Error, PhysicalAddress, Result,
};
use raw_cstr::raw_cstr;
use std::path::Path;

#[simics_exception]
/// Read physical memory at a physical address with a given length. Length must be less than or
/// equal to 8 bytes
pub fn read_phys_memory(cpu: *mut ConfObject, paddr: PhysicalAddress, length: i32) -> u64 {
    unsafe { SIM_read_phys_memory(cpu, paddr, length) }
}

#[simics_exception]
/// Write bytes to physical memory. `value` must be less than or equal to 8 bytes in length.
/// Bytes are written in little-endian format.
pub fn write_phys_memory(cpu: *mut ConfObject, paddr: PhysicalAddress, value: &[u8]) -> Result<()> {
    let mut value_buffer = [0u8; 8];
    if value.len() > value_buffer.len() {
        return Err(Error::ValueTooLarge {
            expected: value_buffer.len(),
            actual: value.len(),
        });
    }
    value_buffer[0..value.len()].copy_from_slice(value);
    let length = value.len() as i32;
    let value = u64::from_le_bytes(value_buffer);
    unsafe { SIM_write_phys_memory(cpu, paddr, value, length) };
    Ok(())
}

#[simics_exception]
/// Read a byte from a physical address
pub fn read_byte(physical_memory: *mut ConfObject, physical_addr: u64) -> u8 {
    unsafe { SIM_read_byte(physical_memory, physical_addr) }
}

#[simics_exception]
/// Write a byte to a physical address
pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
    unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
}

#[simics_exception]
/// Retrieve physical memory tags from memory supporting physical tags, such as CHERI supported
/// memory.
pub fn read_phys_memory_tags(
    mem_space: *mut ConfObject,
    paddr: PhysicalAddress,
    ntags: u32,
) -> u64 {
    unsafe { SIM_read_phys_memory_tags(mem_space, paddr, ntags) }
}

#[simics_exception]
/// Set physical memory tags from memory supporting physical tags, such as CHERI supported
/// memory.
pub fn write_phys_memory_tags(
    mem_space: *mut ConfObject,
    paddr: PhysicalAddress,
    tag_bits: u64,
    ntags: u32,
) {
    unsafe { SIM_write_phys_memory_tags(mem_space, paddr, tag_bits, ntags) }
}

#[simics_exception]
/// Load a binary file into the address space
pub fn load_binary<P>(
    obj: *mut ConfObject,
    file: P,
    offset: PhysicalAddress,
    use_pa: bool,
    verbose: bool,
) -> Result<PhysicalAddress>
where
    P: AsRef<Path>,
{
    Ok(unsafe {
        SIM_load_binary(
            obj,
            raw_cstr(file.as_ref().to_str().ok_or(Error::ToString)?)?,
            offset,
            use_pa,
            verbose,
        )
    })
}

#[simics_exception]
/// Load a (not necessarily binary) file into the physical address space
pub fn load_file<P>(
    obj: *mut ConfObject,
    file: P,
    paddr: PhysicalAddress,
    verbose: bool,
) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_load_file(
            obj,
            raw_cstr(file.as_ref().to_str().ok_or(Error::ToString)?)?,
            paddr,
            verbose,
        )
    };
    Ok(())
}
