// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use crate::{
    arch::ArchitectureOperations,
    state::{MagicStart, ManualStartSize, Solution, SolutionKind, Stop, StopReason},
    StartSize, Tsffs,
};
use anyhow::{anyhow, Result};
use libafl::prelude::ExitKind;
use simics::{
    api::{
        continue_simulation, object_is_processor, quit, run_alone, set_log_level, AsConfObject,
        ConfObject, GenericTransaction, LogLevel,
    },
    info,
};

impl Tsffs {
    /// Called on core simulation stopped HAP
    pub fn on_simulation_stopped(&mut self) -> Result<()> {
        if let Some(reason) = self.stop_reason_mut().take() {
            info!(self.as_conf_object(), "on_simulation_stopped({reason:?})");

            match reason {
                StopReason::MagicStart(magic_start) => {
                    if !self.have_initial_snapshot() {
                        self.start_fuzzer_thread()?;
                        self.add_processor(*magic_start.processor(), true)?;

                        let (start_buffer, start_size) = {
                            let start_processor = self
                                .start_processor()
                                .ok_or_else(|| anyhow!("No start processor"))?;
                            (
                                start_processor.get_magic_start_buffer()?,
                                start_processor.get_magic_start_size()?,
                            )
                        };

                        *self.start_buffer_mut() = Some(start_buffer);
                        *self.start_size_mut() = Some(start_size);
                        *self.start_time_mut() = SystemTime::now();
                        *self.coverage_enabled_mut() = true;
                        self.save_initial_snapshot()?;
                        self.get_and_write_testcase()?;
                    }
                }
                StopReason::ManualStart(start) => {
                    if !self.have_initial_snapshot() {
                        self.start_fuzzer_thread()?;
                        self.add_processor(*start.processor(), true)?;

                        let (start_buffer, start_size) = {
                            let start_processor = self
                                .start_processor()
                                .ok_or_else(|| anyhow!("No start processor"))?;
                            (
                                start_processor.get_manual_start_buffer(*start.buffer())?,
                                match start.size() {
                                    ManualStartSize::MaximumSize(s) => {
                                        StartSize::builder().initial_size(*s).build()
                                    }
                                    ManualStartSize::SizeAddress(a) => {
                                        start_processor.get_manual_start_size(*a)?
                                    }
                                },
                            )
                        };

                        *self.start_buffer_mut() = Some(start_buffer);
                        *self.start_size_mut() = Some(start_size);
                        *self.start_time_mut() = SystemTime::now();
                        *self.coverage_enabled_mut() = true;
                        self.save_initial_snapshot()?;
                        self.get_and_write_testcase()?;
                        self.post_timeout_event()?;
                    }
                }
                StopReason::MagicStop(_) | StopReason::ManualStop(_) => {
                    self.cancel_timeout_event()?;

                    *self.iterations_mut() += 1;

                    if self
                        .configuration()
                        .iterations()
                        .is_some_and(|i| *self.iterations() >= i)
                    {
                        let duration = SystemTime::now().duration_since(*self.start_time())?;

                        // Set the log level so this message always prints
                        set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                        info!(
                            self.as_conf_object(),
                            "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                            self.iterations(),
                            duration.as_secs_f32(),
                            *self.iterations() as f32 / duration.as_secs_f32()
                        );

                        self.send_shutdown()?;

                        quit(0)?;
                    }

                    let fuzzer_tx = self
                        .fuzzer_tx_mut()
                        .as_ref()
                        .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

                    fuzzer_tx.send(ExitKind::Ok)?;

                    self.restore_initial_snapshot()?;

                    self.get_and_write_testcase()?;

                    self.post_timeout_event()?;
                }
                StopReason::Solution(solution) => {
                    self.cancel_timeout_event()?;

                    *self.iterations_mut() += 1;

                    if self
                        .configuration()
                        .iterations()
                        .is_some_and(|i| *self.iterations() >= i)
                    {
                        let duration = SystemTime::now().duration_since(*self.start_time())?;

                        // Set the log level so this message always prints
                        set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                        info!(
                            self.as_conf_object(),
                            "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                            self.iterations(),
                            duration.as_secs_f32(),
                            *self.iterations() as f32 / duration.as_secs_f32()
                        );

                        self.send_shutdown()?;

                        quit(0)?;
                    }

                    let fuzzer_tx = self
                        .fuzzer_tx_mut()
                        .as_ref()
                        .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

                    match solution.kind() {
                        SolutionKind::Timeout => fuzzer_tx.send(ExitKind::Timeout)?,
                        SolutionKind::Exception
                        | SolutionKind::Breakpoint
                        | SolutionKind::Manual => fuzzer_tx.send(ExitKind::Crash)?,
                    }

                    self.restore_initial_snapshot()?;

                    self.get_and_write_testcase()?;

                    self.post_timeout_event()?;
                }
            }

            info!(self.as_conf_object(), "Resuming simulation");

            run_alone(|| {
                continue_simulation(0)?;
                Ok(())
            })?;
        } else {
            info!(
                self.as_conf_object(),
                "Simulation stopped without reason, not resuming."
            );

            let duration = SystemTime::now().duration_since(*self.start_time())?;

            // Set the log level so this message always prints
            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

            info!(
                self.as_conf_object(),
                "Stopped after {} iterations in {} seconds ({} exec/s).",
                self.iterations(),
                duration.as_secs_f32(),
                *self.iterations() as f32 / duration.as_secs_f32()
            );
        }

        Ok(())
    }

    /// Called on core exception HAP. Check to see if this exception is configured as a solution
    /// or all exceptions are solutions and trigger a stop if so
    pub fn on_exception(&mut self, _obj: *mut ConfObject, exception: i64) -> Result<()> {
        if *self.configuration().all_exceptions_are_solutions()
            || self.configuration().exceptions().contains(&exception)
        {
            self.stop_simulation(StopReason::Solution(
                Solution::builder().kind(SolutionKind::Exception).build(),
            ))?;
        }
        Ok(())
    }

    /// Called on breakpoint memory operation HAP. Check to see if this breakpoint is configured
    /// as a solution or if all breakpoints are solutions and trigger a stop if so
    pub fn on_breakpoint_memop(
        &mut self,
        obj: *mut ConfObject,
        breakpoint: i64,
        transaction: *mut GenericTransaction,
    ) -> Result<()> {
        if *self.configuration().all_breakpoints_are_solutions()
            || self
                .configuration()
                .breakpoints()
                .contains(&(breakpoint as i32))
        {
            info!(
                self.as_conf_object(),
                "on_breakpoint_memop({:#x}, {}, {:#x})",
                obj as usize,
                breakpoint,
                transaction as usize
            );

            self.stop_simulation(StopReason::Solution(
                Solution::builder().kind(SolutionKind::Breakpoint).build(),
            ))?;
        }
        Ok(())
    }

    /// Check if magic instructions are set to trigger start and stop conditions, and trigger
    /// them if needed
    pub fn on_magic_instruction(
        &mut self,
        trigger_obj: *mut ConfObject,
        magic_number: i64,
    ) -> Result<()> {
        info!(
            self.as_conf_object(),
            "on_magic_instruction({magic_number})"
        );

        if object_is_processor(trigger_obj)? {
            if *self.configuration().start_on_harness()
                && magic_number == *self.configuration().magic_start()
            {
                self.stop_simulation(StopReason::MagicStart(
                    MagicStart::builder().processor(trigger_obj).build(),
                ))?;
            } else if *self.configuration().stop_on_harness()
                && magic_number == *self.configuration().magic_stop()
            {
                self.stop_simulation(StopReason::MagicStop(Stop::default()))?;
            }
        }
        Ok(())
    }
}