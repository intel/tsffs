// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Handlers for HAPs in the simulator

use std::time::SystemTime;

use crate::{
    arch::ArchitectureOperations,
    magic::MagicNumber,
    state::{SolutionKind, StopReason},
    ManualStartInfo, Tsffs,
};
use anyhow::{anyhow, bail, Result};
use libafl::prelude::ExitKind;
use simics::{
    api::{
        continue_simulation, log_level, object_is_processor, quit, run_alone, set_log_level,
        AsConfObject, ConfObject, GenericTransaction, LogLevel,
    },
    debug, get_processor_number, info, trace, warn,
};

impl Tsffs {
    fn on_simulation_stopped_magic_start(&mut self, magic_number: MagicNumber) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.start_fuzzer_thread()?;

            let start_processor = self
                .start_processor()
                .ok_or_else(|| anyhow!("No start processor"))?;

            let start_info = match magic_number {
                MagicNumber::StartBufferPtrSizePtr => {
                    start_processor.get_magic_start_buffer_ptr_size_ptr()?
                }
                MagicNumber::StartBufferPtrSizeVal => {
                    start_processor.get_magic_start_buffer_ptr_size_val()?
                }
                MagicNumber::StartBufferPtrSizePtrVal => {
                    start_processor.get_magic_start_buffer_ptr_size_ptr_val()?
                }
                MagicNumber::StopNormal => unreachable!("StopNormal is not handled here"),
                MagicNumber::StopAssert => unreachable!("StopAssert is not handled here"),
            };

            debug!(self.as_conf_object(), "Start info: {start_info:?}");

            self.start_info
                .set(start_info)
                .map_err(|_| anyhow!("Failed to set start size"))?;
            self.start_time
                .set(SystemTime::now())
                .map_err(|_| anyhow!("Failed to set start time"))?;
            self.coverage_enabled = true;
            self.save_initial_snapshot()?;
            self.get_and_write_testcase()?;
            self.post_timeout_event()?;
        }

        self.execution_trace.0.clear();
        self.save_repro_bookmark_if_needed()?;

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_magic_assert(&mut self) -> Result<()> {
        self.on_simulation_stopped_solution(SolutionKind::Manual)
    }

    fn on_simulation_stopped_magic_stop(&mut self) -> Result<()> {
        if !self.have_initial_snapshot() {
            warn!(
                self.as_conf_object(),
                "Stopped normally before start was reached (no snapshot). Resuming without restoring non-existent snapshot."
            );
        } else {
            self.cancel_timeout_event()?;

            if self.repro_bookmark_set {
                self.stopped_for_repro = true;
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

            self.iterations += 1;

            if self.iteration_limit != 0 && self.iterations >= self.iteration_limit {
                let duration = SystemTime::now().duration_since(
                    *self
                        .start_time
                        .get()
                        .ok_or_else(|| anyhow!("Start time was not set"))?,
                )?;

                // Set the log level so this message always prints
                set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                info!(
                    self.as_conf_object(),
                    "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                    self.iterations,
                    duration.as_secs_f32(),
                    self.iterations as f32 / duration.as_secs_f32()
                );

                self.send_shutdown()?;

                if self.quit_on_iteration_limit {
                    quit(0)?;
                } else {
                    return Ok(());
                }
            }

            let fuzzer_tx = self
                .fuzzer_tx
                .get()
                .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

            fuzzer_tx.send(ExitKind::Ok)?;

            self.restore_initial_snapshot()?;
            self.coverage_prev_loc = 0;

            if self.start_info.get().is_some() {
                self.get_and_write_testcase()?;
            } else {
                debug!(
                    self.as_conf_object(),
                    "Missing start buffer or size, not writing testcase."
                );
            }

            self.post_timeout_event()?;
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_with_magic(&mut self, magic_number: MagicNumber) -> Result<()> {
        match magic_number {
            MagicNumber::StartBufferPtrSizePtr
            | MagicNumber::StartBufferPtrSizeVal
            | MagicNumber::StartBufferPtrSizePtrVal => {
                self.on_simulation_stopped_magic_start(magic_number)?
            }
            MagicNumber::StopNormal => self.on_simulation_stopped_magic_stop()?,
            MagicNumber::StopAssert => self.on_simulation_stopped_magic_assert()?,
        }

        Ok(())
    }

    fn on_simulation_stopped_with_manual_start(
        &mut self,
        processor: *mut ConfObject,
        info: ManualStartInfo,
    ) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.start_fuzzer_thread()?;
            self.add_processor(processor, true)?;

            let start_info = self
                .start_processor()
                .ok_or_else(|| anyhow!("No start processor"))?
                .get_manual_start_info(&info)?;

            self.start_info
                .set(start_info)
                .map_err(|_| anyhow!("Failed to set start info"))?;
            self.start_time
                .set(SystemTime::now())
                .map_err(|_| anyhow!("Failed to set start time"))?;
            self.coverage_enabled = true;
            self.save_initial_snapshot()?;

            self.get_and_write_testcase()?;

            self.post_timeout_event()?;
        }

        self.execution_trace.0.clear();
        self.save_repro_bookmark_if_needed()?;

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_manual_start_without_buffer(
        &mut self,
        processor: *mut ConfObject,
    ) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.start_fuzzer_thread()?;
            self.add_processor(processor, true)?;

            self.start_time
                .set(SystemTime::now())
                .map_err(|_| anyhow!("Failed to set start time"))?;
            self.coverage_enabled = true;
            self.save_initial_snapshot()?;

            self.post_timeout_event()?;
        }

        self.execution_trace.0.clear();
        self.save_repro_bookmark_if_needed()?;

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_manual_stop(&mut self) -> Result<()> {
        if !self.have_initial_snapshot() {
            warn!(
                self.as_conf_object(),
                "Stopped for manual stop before start was reached (no snapshot). Resuming without restoring non-existent snapshot."
            );
        } else {
            self.cancel_timeout_event()?;

            if self.repro_bookmark_set {
                self.stopped_for_repro = true;
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

            self.iterations += 1;

            if self.iteration_limit != 0 && self.iterations >= self.iteration_limit {
                let duration = SystemTime::now().duration_since(
                    *self
                        .start_time
                        .get()
                        .ok_or_else(|| anyhow!("Start time was not set"))?,
                )?;

                // Set the log level so this message always prints
                set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                info!(
                    self.as_conf_object(),
                    "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                    self.iterations,
                    duration.as_secs_f32(),
                    self.iterations as f32 / duration.as_secs_f32()
                );

                self.send_shutdown()?;

                if self.quit_on_iteration_limit {
                    quit(0)?;
                } else {
                    return Ok(());
                }
            }

            let fuzzer_tx = self
                .fuzzer_tx
                .get()
                .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

            fuzzer_tx.send(ExitKind::Ok)?;

            self.restore_initial_snapshot()?;
            self.coverage_prev_loc = 0;

            if self.start_info.get().is_some() {
                self.get_and_write_testcase()?;
            } else {
                debug!(
                    self.as_conf_object(),
                    "Missing start buffer or size, not writing testcase. This may be due to using manual no-buffer harnessing."
                );
            }

            self.post_timeout_event()?;
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_solution(&mut self, kind: SolutionKind) -> Result<()> {
        if !self.have_initial_snapshot() {
            warn!(
                self.as_conf_object(),
                "Solution {kind:?} before start was reached (no snapshot). Resuming without restoring non-existent snapshot."
            );
        } else {
            self.cancel_timeout_event()?;

            if self.repro_bookmark_set {
                self.stopped_for_repro = true;
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

            self.iterations += 1;

            if self.iteration_limit != 0 && self.iterations >= self.iteration_limit {
                let duration = SystemTime::now().duration_since(
                    *self
                        .start_time
                        .get()
                        .ok_or_else(|| anyhow!("Start time was not set"))?,
                )?;

                // Set the log level so this message always prints
                set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

                info!(
                    self.as_conf_object(),
                    "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                    self.iterations,
                    duration.as_secs_f32(),
                    self.iterations as f32 / duration.as_secs_f32()
                );

                self.send_shutdown()?;

                if self.quit_on_iteration_limit {
                    quit(0)?;
                } else {
                    return Ok(());
                }
            }

            let fuzzer_tx = self
                .fuzzer_tx
                .get()
                .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

            match kind {
                SolutionKind::Timeout => {
                    self.timeouts += 1;
                    fuzzer_tx.send(ExitKind::Timeout)?
                }
                SolutionKind::Exception | SolutionKind::Breakpoint | SolutionKind::Manual => {
                    self.solutions += 1;
                    fuzzer_tx.send(ExitKind::Crash)?
                }
            }

            self.restore_initial_snapshot()?;
            self.coverage_prev_loc = 0;

            if self.start_info.get().is_some() {
                self.get_and_write_testcase()?;
            } else {
                debug!(
                    self.as_conf_object(),
                    "Missing start buffer or size, not writing testcase."
                );
            }

            self.post_timeout_event()?;
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        debug!(self.as_conf_object(), "Resuming simulation");

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    fn on_simulation_stopped_with_reason(&mut self, reason: StopReason) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "Simulation stopped with reason {reason:?}"
        );

        match reason {
            StopReason::Magic { magic_number } => {
                self.on_simulation_stopped_with_magic(magic_number)
            }
            StopReason::ManualStart { processor, info } => {
                self.on_simulation_stopped_with_manual_start(processor, info)
            }
            StopReason::ManualStartWithoutBuffer { processor } => {
                self.on_simulation_stopped_manual_start_without_buffer(processor)
            }
            StopReason::ManualStop => self.on_simulation_stopped_manual_stop(),
            StopReason::Solution { kind } => self.on_simulation_stopped_solution(kind),
        }
    }

    fn on_simulation_stopped_without_reason(&mut self) -> Result<()> {
        if self.have_initial_snapshot() {
            // We only do anything here if we have run, otherwise the simulation was just
            // stopped for a reason unrelated to fuzzing (like the user using the CLI)
            self.cancel_timeout_event()?;

            let fuzzer_tx = self
                .fuzzer_tx
                .get()
                .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

            fuzzer_tx.send(ExitKind::Ok)?;

            info!(
                self.as_conf_object(),
                "Simulation stopped without reason, not resuming."
            );

            let duration = SystemTime::now().duration_since(
                *self
                    .start_time
                    .get()
                    .ok_or_else(|| anyhow!("Start time was not set"))?,
            )?;

            // Set the log level so this message always prints
            set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

            info!(
                self.as_conf_object(),
                "Stopped after {} iterations in {} seconds ({} exec/s).",
                self.iterations,
                duration.as_secs_f32(),
                self.iterations as f32 / duration.as_secs_f32()
            );

            if self.shutdown_on_stop_without_reason {
                self.send_shutdown()?;
            }
        }

        Ok(())
    }

    /// Called on core simulation stopped HAP
    pub fn on_simulation_stopped(&mut self) -> Result<()> {
        if self.stopped_for_repro {
            // If we are stopped for repro, we do nothing on this HAP!
            return Ok(());
        }

        //  Log information from the fuzzer
        self.log_messages()?;

        if let Some(reason) = self.stop_reason.take() {
            self.on_simulation_stopped_with_reason(reason)
        } else {
            self.on_simulation_stopped_without_reason()
        }
    }

    /// Called on core exception HAP. Check to see if this exception is configured as a solution
    /// or all exceptions are solutions and trigger a stop if so
    pub fn on_exception(&mut self, _obj: *mut ConfObject, exception: i64) -> Result<()> {
        if self.all_exceptions_are_solutions || self.exceptions.contains(&exception) {
            self.stop_simulation(StopReason::Solution {
                kind: SolutionKind::Exception,
            })?;
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
        if self.all_breakpoints_are_solutions || self.breakpoints.contains(&(breakpoint as i32)) {
            info!(
                self.as_conf_object(),
                "on_breakpoint_memop({:#x}, {}, {:#x})",
                obj as usize,
                breakpoint,
                transaction as usize
            );

            self.stop_simulation(StopReason::Solution {
                kind: SolutionKind::Breakpoint,
            })?;
        }
        Ok(())
    }

    /// Check if magic instructions are set to trigger start and stop conditions, and trigger
    /// them if needed
    pub fn on_magic_instruction(
        &mut self,
        trigger_obj: *mut ConfObject,
        magic_number: MagicNumber,
    ) -> Result<()> {
        trace!(
            self.as_conf_object(),
            "on_magic_instruction({magic_number})"
        );

        if object_is_processor(trigger_obj)? {
            let processor_number = get_processor_number(trigger_obj)?;

            if !self.processors.contains_key(&processor_number) {
                self.add_processor(trigger_obj, false)?;
            }

            let processor = self
                .processors
                .get_mut(&processor_number)
                .ok_or_else(|| anyhow!("Processor not found"))?;

            let index_selector = processor.get_magic_index_selector()?;

            if match magic_number {
                MagicNumber::StartBufferPtrSizePtr
                | MagicNumber::StartBufferPtrSizeVal
                | MagicNumber::StartBufferPtrSizePtrVal => {
                    self.start_on_harness
                        && (if self.magic_start_index == index_selector {
                            // Set this processor as the start processor now that we know it is
                            // enabled, but only set if it is not already set
                            let _ = self.start_processor_number.get_or_init(|| processor_number);
                            true
                        } else {
                            debug!(
                                "Not setting processor {} as start processor",
                                processor_number
                            );
                            false
                        })
                }
                MagicNumber::StopNormal => {
                    self.stop_on_harness && self.magic_stop_indices.contains(&index_selector)
                }
                MagicNumber::StopAssert => {
                    self.stop_on_harness && self.magic_assert_indices.contains(&index_selector)
                }
            } {
                self.stop_simulation(StopReason::Magic { magic_number })?;
            } else {
                debug!(
                    self.as_conf_object(),
                    "Magic instruction {magic_number} was triggered by processor {trigger_obj:?} with index {index_selector} but the index is not configured for this magic number or start/stop on harness was disabled. Configured indices are: start: {}, stop: {:?}, assert: {:?}",
                    self.magic_start_index,
                    self.magic_stop_indices,
                    self.magic_assert_indices
                );
            }
        } else {
            bail!("Magic instruction was triggered by a non-processor object");
        }

        Ok(())
    }
}
