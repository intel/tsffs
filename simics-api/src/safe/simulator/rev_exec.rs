//! Implements simulator reverse execution and micro checkpointing functions

use anyhow::{Context, Error, Result};
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

#[derive(FromPrimitive, ToPrimitive)]
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
            .context("Invalid value for MicroCheckpointFlags")
    }
}

pub fn delete_micro_checkpoint(index: i32) {
    unsafe { VT_delete_micro_checkpoint(index) }
}

pub fn restore_micro_checkpoint(index: i32) {
    unsafe { VT_restore_micro_checkpoint(index) }
}

pub fn save_micro_checkpoint<S: AsRef<str>>(name: S, flags: MicroCheckpointFlags) -> Result<()> {
    unsafe { VT_save_micro_checkpoint(raw_cstr(name)?, flags.try_into()?) };
    Ok(())
}
