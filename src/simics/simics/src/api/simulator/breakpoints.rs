// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::{
    sys::{
        access_t, breakpoint_flag, breakpoint_id_t, breakpoint_kind_t, SIM_breakpoint,
        SIM_breakpoint_remove, SIM_delete_breakpoint, SIM_disable_breakpoint,
        SIM_enable_breakpoint,
    },
    ConfObject, GenericAddress,
};
use simics_macro::simics_exception;

pub type BreakpointKind = breakpoint_kind_t;
pub type Access = access_t;
pub type BreakpointFlag = breakpoint_flag;
pub type BreakpointId = breakpoint_id_t;

#[simics_exception]
/// Set a breakpoint
pub fn breakpoint(
    obj: *mut ConfObject,
    breakpoint_type: BreakpointKind,
    access: Access,
    address: u64,
    length: u64,
    flags: BreakpointFlag,
) -> BreakpointId {
    unsafe { SIM_breakpoint(obj, breakpoint_type, access, address, length, flags) }
}

#[simics_exception]
/// Delete a breakpoint
pub fn delete_breakpoint(id: BreakpointId) {
    unsafe { SIM_delete_breakpoint(id) };
}

#[simics_exception]
/// Enable a breakpoint
pub fn enable_breakpoint(id: BreakpointId) {
    unsafe { SIM_enable_breakpoint(id) };
}

#[simics_exception]
/// Disable a breakpoint
pub fn disable_breakpoint(id: BreakpointId) {
    unsafe { SIM_disable_breakpoint(id) };
}

#[simics_exception]
/// Remove a breakpoint
pub fn breakpoint_remove(
    id: BreakpointId,
    access: Access,
    address: GenericAddress,
    length: GenericAddress,
) {
    unsafe { SIM_breakpoint_remove(id, access, address, length) };
}
