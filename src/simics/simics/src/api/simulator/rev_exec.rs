// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Implements simulator reverse execution and micro checkpointing functions

use anyhow::{anyhow, Error, Result};
use num::ToPrimitive as _;
use num_derive::{FromPrimitive, ToPrimitive};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    micro_checkpoint_flags_t_Sim_MC_Automatic, micro_checkpoint_flags_t_Sim_MC_ID_Breakpoint,
    micro_checkpoint_flags_t_Sim_MC_ID_Last_States, micro_checkpoint_flags_t_Sim_MC_ID_Mask,
    micro_checkpoint_flags_t_Sim_MC_ID_N_States, micro_checkpoint_flags_t_Sim_MC_ID_Tmp,
    micro_checkpoint_flags_t_Sim_MC_ID_User, micro_checkpoint_flags_t_Sim_MC_Persistent,
    VT_delete_micro_checkpoint, VT_restore_micro_checkpoint, VT_save_micro_checkpoint,
};

#[derive(FromPrimitive, ToPrimitive, Copy, Clone)]
#[repr(u32)]
pub enum MicroCheckpointFlags {
    IdTemp = micro_checkpoint_flags_t_Sim_MC_ID_Tmp,
    IdMask = micro_checkpoint_flags_t_Sim_MC_ID_Mask,
    IdUser = micro_checkpoint_flags_t_Sim_MC_ID_User,
    Automatic = micro_checkpoint_flags_t_Sim_MC_Automatic,
    Persistent = micro_checkpoint_flags_t_Sim_MC_Persistent,
    NStates = micro_checkpoint_flags_t_Sim_MC_ID_N_States,
    Breakpoint = micro_checkpoint_flags_t_Sim_MC_ID_Breakpoint,
    LastStates = micro_checkpoint_flags_t_Sim_MC_ID_Last_States,
}

impl TryFrom<MicroCheckpointFlags> for u32 {
    type Error = Error;

    fn try_from(value: MicroCheckpointFlags) -> Result<Self> {
        value
            .to_u32()
            .ok_or_else(|| anyhow!("Invalid value for MicroCheckpointFlags"))
    }
}

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
    let mut checkpoint_flags = 0;
    for flag in flags {
        checkpoint_flags |= *flag as u32;
    }
    unsafe { VT_save_micro_checkpoint(raw_cstr(name)?, checkpoint_flags) };
    Ok(())
}
