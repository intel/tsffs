// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::run_alone;
use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::{SIM_break_simulation, SIM_continue, SIM_quit};

/// Quit simics and exit with a code
pub fn quit(exit_code: i32) {
    unsafe {
        SIM_quit(exit_code);
    }
}

/// Stop the simulation
pub fn break_simulation<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_break_simulation(raw_cstr(msg.as_ref())?) };
    Ok(())
}

/// Runs SIM_continue in the SIM_run_alone context, because it cannot be called directly from a
/// module thread
pub fn continue_simulation_alone() {
    run_alone(|| unsafe {
        // NOTE: This returns 0 if the simulation was not started, but it can't be caught
        // in the run_alone callback
        SIM_continue(0);
    });
}
