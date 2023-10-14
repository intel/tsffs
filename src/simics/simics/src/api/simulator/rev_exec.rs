// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Implements simulator reverse execution and micro checkpointing functions

use crate::{
    api::{
        control::PcStep,
        sys::{
            micro_checkpoint_flags_t, revexec_pos_t, VT_delete_micro_checkpoint,
            VT_get_rewind_overhead, VT_in_the_past, VT_restore_micro_checkpoint, VT_reverse,
            VT_reverse_cpu, VT_revexec_active, VT_revexec_available, VT_revexec_barrier,
            VT_revexec_cycles, VT_revexec_ignore_class, VT_revexec_steps, VT_rewind,
            VT_save_micro_checkpoint, VT_skipto_bookmark, VT_skipto_cycle, VT_skipto_step,
        },
        ConfObject, Cycles,
    },
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

pub type MicroCheckpointFlags = micro_checkpoint_flags_t;
pub type RevExecPos = revexec_pos_t;

#[simics_exception]
pub fn revexec_available() -> bool {
    unsafe { VT_revexec_available() }
}

#[simics_exception]
pub fn revexec_active() -> bool {
    unsafe { VT_revexec_active() }
}

#[simics_exception]
pub fn in_the_past() -> bool {
    unsafe { VT_in_the_past() }
}

#[simics_exception]
pub fn revexec_steps(cpu: *mut ConfObject, where_: RevExecPos) -> PcStep {
    unsafe { VT_revexec_steps(cpu, where_) }
}

#[simics_exception]
pub fn revexec_cycles(cpu: *mut ConfObject, where_: RevExecPos) -> Cycles {
    unsafe { VT_revexec_cycles(cpu, where_) }
}

#[simics_exception]
pub fn get_rewind_overhead(cpu: *mut ConfObject, abscount: PcStep) -> PcStep {
    unsafe { VT_get_rewind_overhead(cpu, abscount) }
}

#[simics_exception]
pub fn reverse(count: PcStep) -> i32 {
    unsafe { VT_reverse(count) }
}

#[simics_exception]
pub fn reverse_cpu(clock: *mut ConfObject, count: PcStep) -> i32 {
    unsafe { VT_reverse_cpu(clock, count) }
}

#[simics_exception]
pub fn skipto_step(clock: *mut ConfObject, count: PcStep) -> i32 {
    unsafe { VT_skipto_step(clock, count) }
}

#[simics_exception]
pub fn skipto_cycle(clock: *mut ConfObject, count: Cycles) -> i32 {
    unsafe { VT_skipto_cycle(clock, count) }
}

#[simics_exception]
pub fn skipto_bookmark<S>(name: S) -> Result<i32>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_skipto_bookmark(raw_cstr(name)?) })
}

#[simics_exception]
pub fn rewind(cpu: *mut ConfObject, abscount: PcStep) -> i32 {
    unsafe { VT_rewind(cpu, abscount) }
}

#[simics_exception]
/// Save a micro checkpoint with some set of flags
pub fn save_micro_checkpoint<S>(name: S, flags: MicroCheckpointFlags) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { VT_save_micro_checkpoint(raw_cstr(name)?, flags) };

    Ok(())
}

#[simics_exception]
pub fn delete_micro_checkpoint(index: i32) {
    unsafe { VT_delete_micro_checkpoint(index) }
}

#[simics_exception]
/// Restore a micro checkpoint, loading it as a snapshot
pub fn restore_micro_checkpoint(index: i32) {
    unsafe { VT_restore_micro_checkpoint(index) }
}

#[simics_exception]
pub fn revexec_ignore_class<S>(class_name: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { VT_revexec_ignore_class(raw_cstr(class_name)?) };
    Ok(())
}

#[simics_exception]
pub fn revexec_barrier() {
    unsafe { VT_revexec_barrier() }
}
