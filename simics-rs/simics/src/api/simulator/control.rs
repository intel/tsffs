// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Control of the base simulator

use crate::{
    simics_exception,
    sys::{
        pc_step_t, SIM_break_cycle, SIM_break_message, SIM_break_simulation, SIM_break_step,
        SIM_continue, SIM_quit, SIM_shutdown, SIM_simics_is_running,
    },
    ConfObject, Result,
};
use raw_cstr::raw_cstr;

/// Alias for `pc_step_t`
pub type PcStep = pc_step_t;

#[simics_exception]
/// Continue the simulation.
///
/// Run the simulation. In typical usage with steps being 0, the simulation will run
/// forward until it is stopped, either by a breakpoint, internal event, or through the
/// user interface.
///
/// With a non-zero steps, Simics will make sure that at least one processor runs steps
/// steps and then stop the simulation. As with steps being 0, the function can also
/// return early if other break criteria are met.
///
/// In order to properly control when simulation stops in time, it is advisable to use
/// step or cycle breakpoints on one or more objects.
///
/// The function returns non-zero if the simulation was started, and 0 otherwise.
///
/// This typically needs to be run in global scope using:
///
/// ```rust,ignore
/// use simics::api::{continue_simulation, run_alone};
///
/// run_alone(|| { continue_simulation(); });
/// ```
///
/// # Arguments
///
/// * `steps` - Zero to run until stopped, or a number of steps to cintinue for
///
/// # Context
///
/// Global Context
pub fn continue_simulation(steps: i64) -> PcStep {
    unsafe { SIM_continue(steps) }
}

#[simics_exception]
/// Check whether SIMICS is currently running
///
/// Returns true if the simulation is running, e.g. if it has been started using
/// continue_simulation, or false otherwise. It also returns true when the simulation is
/// reversing.
///
/// # Context
///
/// Cell Context
pub fn simics_is_running() -> bool {
    unsafe { SIM_simics_is_running() }
}

#[simics_exception]
/// Stop the simulation with a message
///
/// Ask Simics to stop the simulation as soon as possible, displaying the supplied
/// message.
///
/// Simics will normally stop before the next instruction is executed. If this function
/// is called when an instruction has started executing, and the instruction can be
/// aborted, it will rewind to before the instruction. This might leave the simulation
/// in a state where some repeatable part of the instruction is already executed.
///
/// # Context
///
/// Cell Context
pub fn break_simulation<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_break_simulation(raw_cstr(msg.as_ref())?) };
    Ok(())
}

#[simics_exception]
/// Set the message whhen SIMICs next breaks execution
///
/// Display the reason why Simics will stop simulation.  This is similar to
/// break_simulation, with the difference that it doesn't actually break the
/// simulation. It can be used by code that wants to display a break message and stop
/// the simulation by some other means.
///
/// # Context
///
/// Cell Context
pub fn break_message<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_break_message(raw_cstr(msg)?) };
    Ok(())
}

#[simics_exception]
/// Shutdown simics gracefully without exiting the process.
///
/// Perform the same clean up as quit, but do not exit the process. After having
/// called this function, no Simics API function can be called.
///
/// # Context
///
/// Cell Context
pub fn shutdown() {
    unsafe { SIM_shutdown() };
}

#[simics_exception]
/// Quit simics and exit with a code
///
/// Quit Simics in an orderly fashion. The Simics process will return the value
/// exit_code. See the Core_Clean_At_Exit and Core_At_Exit haps for ways to run user
/// code when Simics exits. Callbacks for the Core_Clean_At_Exit hap will only run if
/// quit is called from Global Context, while Core_At_Exit is always called.
///
/// # Context
///
/// Cell Context
pub fn quit(exit_code: i32) {
    unsafe {
        SIM_quit(exit_code);
    }
}

#[simics_exception]
/// Insert a breakpoint event at cycles clock cycles from now, causing simulation to
/// stop when reached by obj.
///
/// # Arguments
///
/// * `obj` - The object whose cycles will be monitored to break on
/// * `cycles` - The number of cycles until the break occurs
///
/// # Context
///
/// _Cell Context_
pub fn break_cycle(obj: *mut ConfObject, cycles: i64) {
    unsafe { SIM_break_cycle(obj, cycles) };
}

#[simics_exception]
/// Sets a step breakpoint on a processor.
///
/// # Arguments
///
/// * `obj` - The object whose steps will be monitored to break on
/// * `steps` - The number of instructions until the break occurs
///
/// # Exceptions
///
/// * [`SimException::SimExc_InterfaceNotFound`] - Thrown if the obj object doesn't implement the step
/// interface.
///
/// # Context
///
/// _Cell Context_
pub fn break_step(obj: *mut ConfObject, steps: i64) {
    unsafe { SIM_break_step(obj, steps) };
}
