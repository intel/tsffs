// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Implements simulator reverse execution and micro checkpointing functions

use crate::api::sys::{
    VT_delete_micro_checkpoint, VT_restore_micro_checkpoint, VT_save_micro_checkpoint,
};
use crate::error::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::micro_checkpoint_flags_t;

pub type MicroCheckpointFlags = micro_checkpoint_flags_t;

/// Remove a micro checkpoint
pub fn delete_micro_checkpoint(index: i32) {
    unsafe { VT_delete_micro_checkpoint(index) }
}

/// Restore a micro checkpoint, loading it as a snapshot
pub fn restore_micro_checkpoint(index: i32) {
    unsafe { VT_restore_micro_checkpoint(index) }
}

/// Save a micro checkpoint with some set of flags
pub fn save_micro_checkpoint<S>(name: S, flags: &[MicroCheckpointFlags]) -> Result<()>
where
    S: AsRef<str>,
{
    let mut checkpoint_flags = MicroCheckpointFlags::Sim_MC_ID_User;

    for flag in flags {
        checkpoint_flags |= *flag;
    }

    unsafe { VT_save_micro_checkpoint(raw_cstr(name)?, checkpoint_flags) };

    Ok(())
}
