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
        continue_simulation, log_level, object_is_processor, quit, run_alone, set_log_level,
        AsConfObject, ConfObject, GenericTransaction, LogLevel,
    },
    debug, info, trace,
};

impl Tsffs {
    /// Called on core simulation stopped HAP
    pub fn on_simulation_stopped(&mut self) -> Result<()> {
        if self.stopped_for_repro_deref() {
            // If we are stopped for repro, we do nothing on this HAP!
            return Ok(());
        }

        let mut messages = Vec::new();

        if let Some(fuzzer_messages) = self.fuzzer_messages_mut() {
            while let Ok(message) = fuzzer_messages.try_recv() {
                messages.push(message);
            }
        }

        messages
            .iter()
            .for_each(|m| info!(self.as_conf_object(), "{m}"));

        if let Some(reason) = self.stop_reason_mut().take() {
            debug!(self.as_conf_object(), "on_simulation_stopped({reason:?})");

            match reason {
                StopReason::MagicStart(magic_start) => {
                    if !self.have_initial_snapshot() {
                        self.start_fuzzer_thread()?;
                        self.add_processor(magic_start.processor_deref(), true)?;

                        let (start_buffer, start_size) = {
                            let start_processor = self
                                .start_processor()
                                .ok_or_else(|| anyhow!("No start processor"))?;
                            (
                                start_processor.get_magic_start_buffer()?,
                                start_processor.get_magic_start_size()?,
                            )
                        };

                        debug!(
                            self.as_conf_object(),
                            "Start buffer: {start_buffer:?} Start size: {start_size:?}"
                        );

                        *self.start_buffer_mut() = Some(start_buffer);
                        *self.start_size_mut() = Some(start_size);
                        *self.start_time_mut() = SystemTime::now();
                        *self.coverage_enabled_mut() = true;
                        self.save_initial_snapshot()?;
                        self.get_and_write_testcase()?;
                        self.post_timeout_event()?;
                    }

                    self.save_repro_bookmark_if_needed()?;
                    trace!(
                        self.as_conf_object(),
                        "Coverage hash (before): {:#x}",
                        self.cmplog_hash()
                    );
                    trace!(
                        self.as_conf_object(),
                        "Cmplog hash (before): {:#x}",
                        self.cmplog_hash()
                    );
                }
                StopReason::ManualStart(start) => {
                    if !self.have_initial_snapshot() {
                        self.start_fuzzer_thread()?;
                        self.add_processor(start.processor_deref(), true)?;

                        let (start_buffer, start_size) = {
                            let start_processor = self
                                .start_processor()
                                .ok_or_else(|| anyhow!("No start processor"))?;
                            (
                                if let Some(buffer) = start.buffer_ref() {
                                    Some(
                                        start_processor
                                            .get_manual_start_buffer(*buffer, start.virt_deref())?,
                                    )
                                } else {
                                    None
                                },
                                match start.size_ref() {
                                    ManualStartSize::MaximumSize(s) => {
                                        Some(StartSize::builder().initial_size(*s).build())
                                    }
                                    ManualStartSize::SizeAddress(a) => Some(
                                        start_processor
                                            .get_manual_start_size(*a, start.virt_deref())?,
                                    ),
                                    ManualStartSize::NoSize => None,
                                },
                            )
                        };

                        debug!(
                            self.as_conf_object(),
                            "Start buffer: {start_buffer:?} Start size: {start_size:?}"
                        );

                        *self.start_buffer_mut() = start_buffer;
                        *self.start_size_mut() = start_size;
                        *self.start_time_mut() = SystemTime::now();
                        *self.coverage_enabled_mut() = true;
                        self.save_initial_snapshot()?;

                        if self.start_buffer_ref().is_some() && self.start_size_ref().is_some() {
                            self.get_and_write_testcase()?;
                        }

                        self.post_timeout_event()?;
                    }

                    self.save_repro_bookmark_if_needed()?;
                    trace!(
                        self.as_conf_object(),
                        "Coverage hash (before): {:#x}",
                        self.cmplog_hash()
                    );
                    trace!(
                        self.as_conf_object(),
                        "Cmplog hash (before): {:#x}",
                        self.cmplog_hash()
                    );
                }
                StopReason::MagicStop(_) | StopReason::ManualStop(_) => {
                    self.cancel_timeout_event()?;
                    trace!(
                        self.as_conf_object(),
                        "Coverage hash (after): {:#x}",
                        self.cmplog_hash()
                    );
                    trace!(
                        self.as_conf_object(),
                        "Cmplog hash (after): {:#x}",
                        self.cmplog_hash()
                    );

                    if self.repro_bookmark_set_deref() {
                        *self.stopped_for_repro_mut() = true;
                        let current_log_level = log_level(self.as_conf_object_mut())?;

                        if current_log_level < LogLevel::Info as u32 {
                            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;
                        }

                        info!(
                            self.as_conf_object(),
                            "Stopped for repro. Restore to start bookmark with 'reverse-to start'"
                        );

                        // Skip the shutdown and continue, we are finished here
                        return Ok(());
                    }

                    *self.iterations_mut() += 1;

                    if self
                        .configuration_ref()
                        .iterations_deref()
                        .is_some_and(|i| self.iterations_deref() >= i)
                    {
                        let duration = SystemTime::now().duration_since(self.start_time_deref())?;

                        let current_log_level = log_level(self.as_conf_object_mut())?;
                        // Set the log level so this message always prints
                        if current_log_level < LogLevel::Info as u32 {
                            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;
                        }

                        info!(
                            self.as_conf_object(),
                            "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                            self.iterations_ref(),
                            duration.as_secs_f32(),
                            *self.iterations_ref() as f32 / duration.as_secs_f32()
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

                    if self.start_buffer_ref().is_some() && self.start_size_ref().is_some() {
                        self.get_and_write_testcase()?;
                    } else {
                        debug!(
                            self.as_conf_object(),
                            "Missing start buffer or size, not writing testcase."
                        );
                    }

                    self.post_timeout_event()?;
                }
                StopReason::Solution(solution) => {
                    self.cancel_timeout_event()?;
                    trace!(
                        self.as_conf_object(),
                        "Coverage hash (after): {:#x}",
                        self.cmplog_hash()
                    );
                    trace!(
                        self.as_conf_object(),
                        "Cmplog hash (after): {:#x}",
                        self.cmplog_hash()
                    );

                    if self.repro_bookmark_set_deref() {
                        *self.stopped_for_repro_mut() = true;
                        let current_log_level = log_level(self.as_conf_object_mut())?;

                        if current_log_level < LogLevel::Info as u32 {
                            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;
                        }

                        info!(
                            self.as_conf_object(),
                            "Stopped for repro. Restore to start bookmark with 'reverse-to start'"
                        );

                        // Skip the shutdown and continue, we are finished here
                        return Ok(());
                    }

                    *self.iterations_mut() += 1;

                    if self
                        .configuration_ref()
                        .iterations_deref()
                        .is_some_and(|i| self.iterations_deref() >= i)
                    {
                        let duration = SystemTime::now().duration_since(self.start_time_deref())?;

                        // Set the log level so this message always prints
                        set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                        info!(
                            self.as_conf_object(),
                            "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                            self.iterations_ref(),
                            duration.as_secs_f32(),
                            *self.iterations_ref() as f32 / duration.as_secs_f32()
                        );

                        self.send_shutdown()?;

                        quit(0)?;
                    }

                    let fuzzer_tx = self
                        .fuzzer_tx_mut()
                        .as_ref()
                        .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

                    match solution.kind_ref() {
                        SolutionKind::Timeout => fuzzer_tx.send(ExitKind::Timeout)?,
                        SolutionKind::Exception
                        | SolutionKind::Breakpoint
                        | SolutionKind::Manual => fuzzer_tx.send(ExitKind::Crash)?,
                    }

                    self.restore_initial_snapshot()?;

                    if self.start_buffer_ref().is_some() && self.start_size_ref().is_some() {
                        self.get_and_write_testcase()?;
                    } else {
                        debug!(
                            self.as_conf_object(),
                            "Missing start buffer or size, not writing testcase."
                        );
                    }

                    self.post_timeout_event()?;
                }
            }

            debug!(self.as_conf_object(), "Resuming simulation");

            run_alone(|| {
                continue_simulation(0)?;
                Ok(())
            })?;
        } else if self.have_initial_snapshot() {
            self.cancel_timeout_event()?;

            let fuzzer_tx = self
                .fuzzer_tx_mut()
                .as_ref()
                .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

            fuzzer_tx.send(ExitKind::Ok)?;

            info!(
                self.as_conf_object(),
                "Simulation stopped without reason, not resuming."
            );

            let duration = SystemTime::now().duration_since(self.start_time_deref())?;

            // Set the log level so this message always prints
            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

            info!(
                self.as_conf_object(),
                "Stopped after {} iterations in {} seconds ({} exec/s).",
                self.iterations_ref(),
                duration.as_secs_f32(),
                *self.iterations_ref() as f32 / duration.as_secs_f32()
            );
        }

        Ok(())
    }

    /// Called on core exception HAP. Check to see if this exception is configured as a solution
    /// or all exceptions are solutions and trigger a stop if so
    pub fn on_exception(&mut self, _obj: *mut ConfObject, exception: i64) -> Result<()> {
        if self
            .configuration_ref()
            .all_exceptions_are_solutions_deref()
            || self
                .configuration_ref()
                .exceptions_ref()
                .contains(&exception)
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
        if self
            .configuration_ref()
            .all_breakpoints_are_solutions_deref()
            || self
                .configuration_ref()
                .breakpoints_ref()
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
        trace!(
            self.as_conf_object(),
            "on_magic_instruction({magic_number})"
        );

        if object_is_processor(trigger_obj)? {
            if self.configuration_ref().start_on_harness_deref()
                && magic_number == self.configuration_ref().magic_start_deref()
            {
                self.stop_simulation(StopReason::MagicStart(
                    MagicStart::builder().processor(trigger_obj).build(),
                ))?;
            } else if self.configuration_ref().stop_on_harness_deref()
                && magic_number == self.configuration_ref().magic_stop_deref()
            {
                self.stop_simulation(StopReason::MagicStop(Stop::default()))?;
            } else if self.configuration_ref().stop_on_harness_deref()
                && magic_number == self.configuration_ref().magic_assert_deref()
            {
                self.stop_simulation(StopReason::Solution(
                    Solution::builder().kind(SolutionKind::Manual).build(),
                ))?;
            }
        }
        Ok(())
    }
}
