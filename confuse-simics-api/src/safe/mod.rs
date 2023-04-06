//! A small safe API around SIMICS APIs that are used in the CONFUSE project.
//! Feel free to add more safe wrappers as you see fit
//!

use anyhow::Result;
pub mod wrapper {
    use super::super::{
        micro_checkpoint_flags_t_Sim_MC_ID_User, micro_checkpoint_flags_t_Sim_MC_Persistent,
        CORE_discard_future, SIM_quit, VT_restore_micro_checkpoint, VT_save_micro_checkpoint,
    };
    use raw_cstr::raw_cstr;
    use std::ffi::{c_int, CString};

    pub fn quit() {
        unsafe {
            SIM_quit(0);
        }
    }

    pub fn restore_micro_checkpoint(index: i32) {
        unsafe {
            VT_restore_micro_checkpoint(index as c_int);
        }
    }

    pub fn save_micro_checkpoint(name: &str) {
        unsafe {
            VT_save_micro_checkpoint(
                raw_cstr!(name),
                micro_checkpoint_flags_t_Sim_MC_ID_User
                    | micro_checkpoint_flags_t_Sim_MC_Persistent,
            )
        }
    }

    pub fn discard_future() {
        unsafe {
            CORE_discard_future();
        }
    }
}

pub mod common {
    use super::super::{
        SIM_attr_list_size, SIM_continue, SIM_get_attribute, SIM_get_object, SIM_run_alone,
    };
    use raw_cstr::raw_cstr;
    use std::ffi::CString;
    use std::{mem::transmute, ptr::null_mut};

    pub fn continue_simulation() {
        unsafe {
            SIM_run_alone(
                Some(transmute(SIM_continue as unsafe extern "C" fn(_) -> _)),
                null_mut(),
            );
        }
    }

    pub fn count_micro_checkpoints() -> Result<u32> {
        let rexec = unsafe { SIM_get_object(raw_cstr!("sim.rexec")) };

        let sinfo = unsafe { SIM_get_attribute(rexec, raw_cstr!("state_info")) };

        let sinfo_size = SIM_attr_list_size(sinfo)?;

        Ok(sinfo_size)
    }
}
