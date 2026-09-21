// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Handlers for HAPs in the simulator

use std::{collections::HashSet, time::SystemTime};

use crate::{
    arch::ArchitectureOperations,
    magic::MagicNumber,
    os::DebugInfoConfig,
    state::{SolutionKind, StopReason},
    ManualStartInfo, Tsffs,
};
use anyhow::{anyhow, bail, Result};
use intervaltree::IntervalTree;
use libafl::prelude::ExitKind;
use simics::{
    api::{
        continue_simulation, log_level, object_is_processor, quit, run_alone, set_log_level,
        AsConfObject, ConfObject, GenericTransaction, LogLevel,
    },
    debug, get_processor_number, info, trace, warn,
};

enum IterationControl {
    Continue,
    StopRequested,
}

enum IterationCount {
    NoCount,
    Timeout,
    Solution,
}

enum SnapshotRestoreMode {
    PolicyControlled,
    Always,
}

impl Tsffs {
    /// Collect UEFI/SMM source coverage info, if `uefi` and `symbolic_coverage` are
    /// both set. Called once, at `HARNESS_START` (the same three call sites Windows
    /// uses for its own initial collection), rather than on a recurring trigger --
    /// see `crate::uefi::collect_symbols`'s doc comment for why UEFI/SMM has no
    /// CR3-write-equivalent refresh signal the way Windows does.
    ///
    /// The resulting interval tree is stored in `self.windows_os_info`, alongside
    /// Windows's own per-processor symbol lookup trees, keyed the same way (by
    /// processor number). `self.uefi` and `self.windows` are mutually exclusive in
    /// practice (a target is either a Windows kernel or a UEFI/SMM BIOS, not both),
    /// so this reuses the exact same storage and the tracer's existing OS-agnostic
    /// coverage lookup (`src/tracer/mod.rs`, the `self.coverage_enabled &&
    /// self.symbolic_coverage` branch, which does not itself check `self.windows`)
    /// rather than duplicating a second lookup path just for `uefi`.
    fn collect_uefi_symbolic_coverage(&mut self, processor: *mut ConfObject) -> Result<()> {
        if !(self.uefi && self.symbolic_coverage) {
            return Ok(());
        }

        info!(
            self.as_conf_object(),
            "Collecting initial UEFI/SMM source coverage info"
        );

        let elements = crate::uefi::collect_symbols(
            &self.uefi_tracker_object,
            &self.uefi_debug_info_directory,
            &self.source_file_cache,
        )?;

        let mut filtered_ranges = HashSet::new();

        // Deduplicate elements by their range, mirroring
        // `WindowsOsInfo::collect`'s own deduplication.
        let elements = elements
            .into_iter()
            .filter(|e| filtered_ranges.insert(e.range.clone()))
            .collect::<Vec<_>>();

        // Populate elements into the coverage record set, mirroring
        // `WindowsOsInfo::collect`'s own population of `user_debug_info.coverage`.
        elements.iter().map(|e| &e.value).for_each(|si| {
            if let Some(first) = si.lines.first() {
                let record = self.coverage.get_or_insert_mut(&first.file_path);
                record.add_function_if_not_exists(
                    first.start_line as usize,
                    si.lines.last().map(|l| l.end_line as usize),
                    &si.name,
                );
                si.lines.iter().for_each(|l| {
                    (l.start_line..=l.end_line).for_each(|line| {
                        record.add_line_if_not_exists(line as usize);
                    });
                });
            }
        });

        let processor_nr = get_processor_number(processor)?;

        self.windows_os_info.symbol_lookup_trees.insert(
            processor_nr,
            elements.into_iter().collect::<IntervalTree<_, _>>(),
        );

        Ok(())
    }

    fn on_simulation_stopped_magic_start(&mut self, magic_number: MagicNumber) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.start_fuzzer_thread()?;

            let start_processor = self
                .start_processor()
                .ok_or_else(|| anyhow!("No start processor"))?;
            let start_processor_raw = start_processor.cpu();

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
            // Collect windows coverage info if enabled
            if self.windows && self.symbolic_coverage {
                info!(self.as_conf_object(), "Collecting initial coverage info");
                self.windows_os_info.collect(
                    start_processor_raw,
                    &self.debuginfo_download_directory,
                    &mut DebugInfoConfig {
                        system: self.symbolic_coverage_system,
                        user_debug_info: &self.debug_info,
                        coverage: &mut self.coverage,
                    },
                    &self.source_file_cache,
                )?;
            }
            self.collect_uefi_symbolic_coverage(start_processor_raw)?;
            self.get_and_write_testcase()?;
            self.post_timeout_event()?;
        }

        self.execution_trace.0.clear();
        self.save_repro_bookmark_if_needed()?;

        self.continue_after_repro_prepared()?;

        Ok(())
    }

    fn on_simulation_stopped_magic_assert(&mut self) -> Result<()> {
        self.on_simulation_stopped_solution(SolutionKind::Manual)
    }

    fn finish_iteration(
        &mut self,
        exit_kind: ExitKind,
        iteration_count: IterationCount,
        snapshot_restore_mode: SnapshotRestoreMode,
        missing_start_info_message: &str,
    ) -> Result<IterationControl> {
        // 1) Count this iteration as complete.
        self.iterations += 1;

        // 2) Enforce iteration cap before scheduling/resuming work for next iteration.
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
                return Ok(IterationControl::StopRequested);
            }
        }

        // 3) Update outcome counters where this stop reason contributes to stats.
        match iteration_count {
            IterationCount::NoCount => {}
            IterationCount::Timeout => self.timeouts += 1,
            IterationCount::Solution => self.solutions += 1,
        }

        let fuzzer_tx = self
            .fuzzer_tx
            .get()
            .ok_or_else(|| anyhow!("No fuzzer tx channel"))?;

        // 4) Publish this iteration result back to the fuzzer loop.
        fuzzer_tx.send(exit_kind)?;

        // 5) Restore to initial snapshot according to the stop-specific restore policy.
        if match snapshot_restore_mode {
            SnapshotRestoreMode::PolicyControlled => self.should_restore_snapshot_this_iteration(),
            SnapshotRestoreMode::Always => true,
        } {
            self.restore_initial_snapshot()?;
        }

        // 6) Reset AFL edge chaining state for the next execution.
        self.coverage_prev_loc = 0;

        // 7) Persist testcase bytes when start metadata is available.
        if self.start_info.get().is_some() {
            self.get_and_write_testcase()?;
        } else {
            debug!(self.as_conf_object(), "{missing_start_info_message}");
        }

        // 8) Arm timeout for the next iteration run.
        self.post_timeout_event()?;

        Ok(IterationControl::Continue)
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
                    "Stopped for repro. Restore origin state with '{}'",
                    Tsffs::repro_restore_command()
                );

                // Skip the shutdown and continue, we are finished here
                return Ok(());
            }

            // Normal stop path: report successful completion without solution/timeout counters.
            if let IterationControl::StopRequested = self.finish_iteration(
                ExitKind::Ok,
                IterationCount::NoCount,
                SnapshotRestoreMode::PolicyControlled,
                "Missing start buffer or size, not writing testcase.",
            )? {
                return Ok(());
            }
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        if self.symbolic_coverage {
            self.save_symbolic_coverage()?;
        }

        self.continue_after_repro_prepared()?;

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

            // Collect windows coverage info if enabled
            if self.windows && self.symbolic_coverage {
                info!(self.as_conf_object(), "Collecting initial coverage info");
                self.windows_os_info.collect(
                    processor,
                    &self.debuginfo_download_directory,
                    &mut DebugInfoConfig {
                        system: self.symbolic_coverage_system,
                        user_debug_info: &self.debug_info,
                        coverage: &mut self.coverage,
                    },
                    &self.source_file_cache,
                )?;
            }

            self.collect_uefi_symbolic_coverage(processor)?;

            self.get_and_write_testcase()?;

            self.post_timeout_event()?;
        }

        self.execution_trace.0.clear();
        self.save_repro_bookmark_if_needed()?;

        self.continue_after_repro_prepared()?;

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

            // Collect windows coverage info if enabled
            if self.windows && self.symbolic_coverage {
                info!(self.as_conf_object(), "Collecting initial coverage info");
                self.windows_os_info.collect(
                    processor,
                    &self.debuginfo_download_directory,
                    &mut DebugInfoConfig {
                        system: self.symbolic_coverage_system,
                        user_debug_info: &self.debug_info,
                        coverage: &mut self.coverage,
                    },
                    &self.source_file_cache,
                )?;
            }

            self.collect_uefi_symbolic_coverage(processor)?;

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
                    "Stopped for repro. Restore origin state with '{}'",
                    Tsffs::repro_restore_command()
                );

                // Skip the shutdown and continue, we are finished here
                return Ok(());
            }

            // Manual stop behaves like normal completion for accounting purposes.
            if let IterationControl::StopRequested = self.finish_iteration(
                ExitKind::Ok,
                IterationCount::NoCount,
                SnapshotRestoreMode::PolicyControlled,
                "Missing start buffer or size, not writing testcase. This may be due to using manual no-buffer harnessing.",
            )? {
                return Ok(());
            }
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        if self.symbolic_coverage {
            self.save_symbolic_coverage()?;
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
                    "Stopped for repro. Restore origin state with '{}'",
                    Tsffs::repro_restore_command()
                );

                // Skip the shutdown and continue, we are finished here
                return Ok(());
            }

            let (exit_kind, iteration_count) = match kind {
                SolutionKind::Timeout => (ExitKind::Timeout, IterationCount::Timeout),
                SolutionKind::Exception { .. }
                | SolutionKind::Breakpoint { .. }
                | SolutionKind::Manual => (ExitKind::Crash, IterationCount::Solution),
            };

            // Solution/timeout path: classify exit kind and increment corresponding counters.
            if let IterationControl::StopRequested = self.finish_iteration(
                exit_kind,
                iteration_count,
                SnapshotRestoreMode::Always,
                "Missing start buffer or size, not writing testcase.",
            )? {
                return Ok(());
            }
        }

        if self.save_all_execution_traces {
            self.save_execution_trace()?;
        }

        if self.symbolic_coverage {
            self.save_symbolic_coverage()?;
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
                kind: SolutionKind::Exception { number: exception },
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
                kind: SolutionKind::Breakpoint { number: breakpoint },
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
            "Got magic instruction with magic #{magic_number})"
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

    pub fn on_control_register_write(
        &mut self,
        trigger_obj: *mut ConfObject,
        register_nr: i64,
        value: i64,
    ) -> Result<()> {
        self.on_control_register_write_windows_symcov(trigger_obj, register_nr, value)?;

        Ok(())
    }
}
