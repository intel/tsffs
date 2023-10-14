// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{
        sys::{
            pc_step_t, SIM_break_cycle, SIM_break_simulation, SIM_break_step, SIM_continue,
            SIM_quit, SIM_shutdown, SIM_simics_is_running,
        },
        ConfObject,
    },
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

pub type PcStep = pc_step_t;

#[simics_exception]
pub fn continue_simulation(steps: i64) -> PcStep {
    unsafe { SIM_continue(steps) }
}

#[simics_exception]
pub fn simics_is_running() -> bool {
    unsafe { SIM_simics_is_running() }
}

#[simics_exception]
/// Stop the simulation
pub fn break_simulation<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_break_simulation(raw_cstr(msg.as_ref())?) };
    Ok(())
}

#[simics_exception]
pub fn break_message<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_break_simulation(raw_cstr(msg)?) };
    Ok(())
}

#[simics_exception]
pub fn shutdown() {
    unsafe { SIM_shutdown() };
}

#[simics_exception]
/// Quit simics and exit with a code
pub fn quit(exit_code: i32) {
    unsafe {
        SIM_quit(exit_code);
    }
}

#[simics_exception]
pub fn break_cycle(obj: *mut ConfObject, cycles: i64) {
    unsafe { SIM_break_cycle(obj, cycles) };
}

#[simics_exception]
pub fn break_step(obj: *mut ConfObject, steps: i64) {
    unsafe { SIM_break_step(obj, steps) };
}
