// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Breakpoint APIs

use crate::{
    simics_exception,
    sys::{
        access_t, breakpoint_flag, breakpoint_id_t, breakpoint_kind_t, SIM_breakpoint,
        SIM_breakpoint_remove, SIM_delete_breakpoint, SIM_disable_breakpoint,
        SIM_enable_breakpoint,
    },
    ConfObject, GenericAddress,
};

/// Alias for `breakpoint_kind_t`
pub type BreakpointKind = breakpoint_kind_t;
/// Alias for `access_t`
pub type Access = access_t;
/// Alias for `breakpoint_flag`
pub type BreakpointFlag = breakpoint_flag;
/// Alias for `breakpoint_id_t`
pub type BreakpointId = breakpoint_id_t;

#[simics_exception]
/// Add breakpoint on an object implementing the breakpoint interface. This is typically
/// a memory space object such as physical memory.
///
/// The default action for a triggered breakpoint is to return to the frontend (this can
/// be changed by using haps). On execution breakpoints Simics will return to the
/// frontend before the instructions is executed, while instructions triggering read or
/// write breakpoints will complete before control is returned to the frontend.
///
/// Several breakpoints can be set on the same address and Simics will break on them in
/// turn. If hap handlers are connected to the breakpoints they will also be executed in
/// turn. Hap handlers are called before the access is performed, allowing the user to
/// read a memory value that may be overwritten by the access. See the Simics Reference
/// Manual for a description of hap handlers.
///
/// Several attributes can be set for a breakpoint for breaking only when some
/// conditions are true. See the breakpoints attribute in the sim class.
///
/// This function returns the breakpoint id which is used for further reference to the
/// breakpoin
///
/// # Arguments
///
/// - `kind`: The kind argument sets what type of address to break on
/// - `access`: The access argument is a bit-field setting the type of access. Any
/// combination of the three alternatives can be given (added together).
/// - `address`: The address is the start of the breakpoint range
/// - `length` is its length in bytes. This range will be truncated as necessary to fit
/// in the address space. An access intersecting the given range will trigger the
/// breakpoint. If length is zero, the breakpoint range will be the entire address
/// space.
/// - `flags`: If the Sim_Breakpoint_Temporary bit is set, the breakpoint is
/// automatically disabled when triggered the first time. If the
/// Sim_Breakpoint_Simulation bit is set, the breakpoint will not show up in the
/// <bp-manager>.list command, nor can it be removed by the <bp-manager>.delete command.
/// Also, there will be no message printed on the Simics console when this breakpoint is
/// triggered. This bit should be set when using breakpoints to simulate the target
/// system; it will prevent Simics from temporarily disabling the breakpoint as an
/// optimization measure. This could otherwise happen during certain reverse execution
/// operations. If the Sim_Breakpoint_Private bit is set, the breakpoint will not show
/// up in the <bp-manager>.list command, nor can it be removed by the
/// <bp-manager>.delete command.
///
/// # Exceptions
///
/// - SimExc_General Thrown if the type or access arguments are illegal. Also thrown if
/// obj cannot handle breakpoints of the given kind.
///
/// # Context
///
/// Cell Context
pub fn breakpoint(
    obj: *mut ConfObject,
    kind: BreakpointKind,
    access: Access,
    address: u64,
    length: u64,
    flags: BreakpointFlag,
) -> BreakpointId {
    unsafe { SIM_breakpoint(obj, kind, access, address, length, flags) }
}

#[simics_exception]
/// Deletes breakpoint id as returned by SIM_breakpoint. A value of zero will delete all
/// breakpoints that were set without the Sim_Breakpoint_Simulation flag.
///
/// # Arguments
///
/// - `id`: The breakpoint ID to delete, or 0 to delete all breakpoints set without the
/// `Sim_Breakpoint_Simulation` flag
///
/// # Exceptions
///
/// - SimExc_Index Thrown if no breakpoint with the id is found.
///
/// # Context
///
/// Cell Context
pub fn delete_breakpoint(id: BreakpointId) {
    unsafe { SIM_delete_breakpoint(id) };
}

#[simics_exception]
/// Enable a breakpoint
///
/// # Context
///
/// Cell Context
pub fn enable_breakpoint(id: BreakpointId) {
    unsafe { SIM_enable_breakpoint(id) };
}

#[simics_exception]
/// Disable a breakpoint with id `id` as returned by `breakpoint`
///
/// # Arguments
///
/// - `id`: The breakpoint ID returned by `breakpoint`.
///
/// # Exceptions
///
/// - SimExc_Index Thrown if no breakpoint with the id is found.
///
/// # Context
///
/// Cell Context
pub fn disable_breakpoint(id: BreakpointId) {
    unsafe { SIM_disable_breakpoint(id) };
}

#[simics_exception]
/// Deletes a breakpoint range from an existing breakpoint. Can thus be used to create
/// holes in the breakpoint range. id is the breakpoint to operate on, as returned by
/// SIM_breakpoint. A value of zero will operate on all breakpoints that were not set
/// using the Sim_Breakpoint_Simulation flag.
///
/// # Arguments
///
/// - `access`: A bitfield describing the type of breakpoint to remove using the
/// enumeration constants of the access_t enum.
/// - `address`: The start address of the range
/// - `length`: The length of the range in bytes.
///
/// # Exceptions
///
/// - SimExc_Index Thrown if illegal breakpoint
///
/// # Context
///
/// Cell Context
pub fn breakpoint_remove(
    id: BreakpointId,
    access: Access,
    address: GenericAddress,
    length: GenericAddress,
) {
    unsafe { SIM_breakpoint_remove(id, access, address, length) };
}
