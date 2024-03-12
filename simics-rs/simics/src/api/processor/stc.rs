// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Cache handling

use crate::{
    simics_exception,
    sys::{
        SIM_STC_flush_cache, SIM_flush_D_STC_logical, SIM_flush_D_STC_physical,
        SIM_flush_I_STC_logical,
    },
    ConfObject, LogicalAddress, PhysicalAddress, ReadOrWrite,
};

#[simics_exception]
/// Flush instruction logical
pub fn flush_i_stc_logical(cpu: *mut ConfObject, vaddr: LogicalAddress, length: LogicalAddress) {
    unsafe { SIM_flush_I_STC_logical(cpu, vaddr, length) };
}

#[simics_exception]
/// Flush data logical
pub fn flush_d_stc_logical(cpu: *mut ConfObject, vaddr: LogicalAddress, length: LogicalAddress) {
    unsafe { SIM_flush_D_STC_logical(cpu, vaddr, length) };
}

#[simics_exception]
/// Flush instruction physical
pub fn flush_i_stc_physical(cpu: *mut ConfObject, paddr: PhysicalAddress, length: PhysicalAddress) {
    unsafe { SIM_flush_I_STC_logical(cpu, paddr, length) };
}

#[simics_exception]
/// Flush data physical
pub fn flush_d_stc_physical(
    cpu: *mut ConfObject,
    paddr: PhysicalAddress,
    length: PhysicalAddress,
    read_or_write: ReadOrWrite,
) {
    unsafe { SIM_flush_D_STC_physical(cpu, paddr, length, read_or_write) };
}

#[simics_exception]
/// Full flush
pub fn stc_flush_cache(cpu: *mut ConfObject) {
    unsafe { SIM_STC_flush_cache(cpu) };
}
