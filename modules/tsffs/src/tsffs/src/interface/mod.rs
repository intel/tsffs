// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{tracer::CoverageMode, Tsffs};
use ffi_macro::ffi;
use simics::{
    api::{AsConfObject, BreakpointId, GenericAddress},
    error, info, Result,
};
use simics_macro::interface_impl;
use std::{
    ffi::{c_char, CStr},
    str::FromStr,
};

// Emit the interface header/dml files in the "modules" directory in the module subdirectory
// of the package
#[interface_impl(modules_path = "../../../")]
impl Tsffs {
    /// Interface method to enable or disable the fuzzer to start automatically when it
    /// reaches the default start condition for the architecture of the processor that
    /// is running when the default start condition occurs. Note that this method will
    /// not resume or run the simulation, the SIMICS script containing this call should
    /// resume execution afterward.
    ///
    /// These conditions are:
    ///
    /// # x86_64
    ///
    /// - Magic instruction executed with `n=1`
    /// - `rsi` set to the address the fuzzer should write the testcase to each execution
    /// - `rdi` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # x86_32
    ///
    /// - Magic instruction executed with `n=1`
    /// - `esi` set to the address the fuzzer should write the testcase to each execution
    /// - `edi` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # RISC-V
    ///
    /// - Magic instruction executed with `n=1`
    /// - `x10` set to the address the fuzzer should write the testcase to each execution
    /// - `x11` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARM
    ///
    /// - Magic instruction executed with `n=1`
    /// - `r0` set to the address the fuzzer should write the testcase to each execution
    /// - `r1` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARM Thumb-2
    ///
    /// - Magic instruction executed with `n=1`
    /// - `r0` set to the address the fuzzer should write the testcase to each execution
    /// - `r1` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARMv8
    ///
    /// - Magic instruction executed with `n=1`
    /// - `x0` set to the address the fuzzer should write the testcase to each execution
    /// - `x1` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARC
    ///
    /// - Magic instruction executed with `n=1`
    /// - `r0` set to the address the fuzzer should write the testcase to each execution
    /// - `r1` set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    pub fn set_start_on_harness(&self, start_on_harness: bool) {}

    /// Interface method to enable or disable the fuzzer to stop automatically when it
    /// reaches the default stop condition for the architecture of the processor that is
    /// running when the default stop condition occurs. Note that this method will not
    /// resume or run the simulation, the SIMICS script containing this call should
    /// resume execution afterward.
    pub fn set_stop_on_harness(&self, stop_on_harness: bool) {}

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving the
    /// testcase and size address and resuming execution of the simulation.
    ///
    /// # Arguments
    ///
    /// - `testcase_address`: The address to write test cases to
    /// - `size_address`: The address to write the size of each test case to (optional,
    /// `max_size` must be given if not provided).
    /// - `virt`: Whether the provided addresses should be interpreted as virtual or physical
    ///
    /// If your target cannot take advantage of the written-back size pointer, use
    /// `start_with_max_size` instead.
    pub fn start(
        &mut self,
        testcase_address: GenericAddress,
        size_address: GenericAddress,
        virt: bool,
    ) {
        info!(
            self.as_conf_object_mut(),
            "start({testcase_address:#x}, {size_address:#x})"
        );
    }

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving the
    /// testcase and size address and resuming execution of the simulation.
    ///
    /// # Arguments
    ///
    /// - `testcase_address`: The address to write test cases to
    /// - `maximum_size`: The maximum size of the test case. The actual size of each test case will
    ///   not be written back to the target software
    /// - `virt`: Whether the provided addresses should be interpreted as virtual or physical
    pub fn start_with_maximum_size(
        &mut self,
        testcase_address: GenericAddress,
        maximum_size: usize,
        virt: bool,
    ) {
        info!(
            self.as_conf_object_mut(),
            "start_with_maximum_size({testcase_address:#x}, {maximum_size:#x})"
        );
    }

    /// Interface method to manually signal to stop a testcase execution. When this
    /// method is called, the current testcase execution will be stopped as if it had
    /// finished executing normally, and the state will be restored to the state at the
    /// initial snapshot. This method is particularly useful in callbacks triggered on
    /// breakpoints or other complex conditions.
    pub fn stop(&mut self) {
        info!(self.as_conf_object_mut(), "stop");
    }

    /// Interface method to set the fuzzer to use the experimental snapshots interface
    /// instead of the micro checkpoints interface for snapshot save and restore operations
    pub fn set_use_snapshots(&mut self, use_snapshots: bool) {
        info!(self.as_conf_object_mut(), "use_snapshots({use_snapshots})");

        *self.driver_mut().configuration_mut().use_snapshots_mut() = use_snapshots;
    }

    /// Interface method to manually signal to stop execution with a solution condition.
    /// When this method is called, the current testcase execution will be stopped as if
    /// it had finished executing with an exception or timeout, and the state will be
    /// restored to the state at the initial snapshot.
    pub fn solution(&mut self, id: u64, message: *mut c_char) -> Result<()> {
        let message = unsafe { CStr::from_ptr(message) }.to_str()?.to_string();

        info!(self.as_conf_object_mut(), "solution({id:#x}, {message})");

        Ok(())
    }

    /// Interface method to set the execution timeout in seconds
    pub fn set_timeout(&mut self, timeout: f64) {
        info!(self.as_conf_object_mut(), "set_timeout({timeout})");

        *self.detector_mut().config_mut().timeout_mut() = timeout;
    }

    /// Interface method to add an exception-type solution number to the set of
    /// exception-type solution numbers currently being monitored for. If any exception in
    /// the set of exceptions currently monitored occurs, the testcase will be saved and
    /// reported as a solution.
    ///
    /// For example on x86_64, `add_exception_solution(14)` would treat any page fault as
    /// a solution.
    pub fn add_exception_solution(&mut self, exception: i64) {
        info!(
            self.as_conf_object_mut(),
            "add_exception_solution({exception})"
        );

        self.detector_mut()
            .config_mut()
            .exceptions_mut()
            .insert(exception);
    }

    /// Interface method to remove an exception-type solution number from the set of
    /// exception-type solution numbers currently being monitored for. If any exception in
    /// the set of solutions currently monitored occurs, the testcase will be saved and
    /// reported as a solution.
    pub fn remove_exception_solution(&mut self, exception: i64) {
        info!(
            self.as_conf_object_mut(),
            "remove_exception_solution({exception})"
        );

        self.detector_mut()
            .config_mut()
            .exceptions_mut()
            .remove(&exception);
    }

    /// Set whether all CPU exceptions are considered solutions. If set to true, any
    /// exception encountered during fuzzing will be saved as a solution. This is typically
    /// not desired.
    pub fn set_all_exceptions_are_solutions(&mut self, all_exceptions_are_solutions: bool) {
        info!(
            self.as_conf_object_mut(),
            "set_all_exceptions_are_solutions({all_exceptions_are_solutions})"
        );

        *self
            .detector_mut()
            .config_mut()
            .all_exceptions_are_solutions_mut() = all_exceptions_are_solutions;
    }

    /// Set whether all SIMICS breakpoints are considered solutions. If set to true, any
    /// breakpoint (read, write, or execute) encountered during fuzzing will be saved as
    /// a solution.
    pub fn set_all_breakpoints_are_solutions(&mut self, all_breakpoints_are_solutions: bool) {
        info!(
            self.as_conf_object_mut(),
            "set_all_breakpoints_are_solutions({all_breakpoints_are_solutions})"
        );

        *self
            .detector_mut()
            .config_mut()
            .all_breakpoints_are_solutions_mut() = all_breakpoints_are_solutions;
    }

    /// Set a specific breakpoint number to be considered a solution. If a breakpoint with
    /// this ID is encountered during fuzzing, the input will be saved as a solution.
    pub fn add_breakpoint_solution(&mut self, breakpoint: BreakpointId) {
        info!(
            self.as_conf_object_mut(),
            "add_breakpoint_solution({breakpoint})"
        );

        self.detector_mut()
            .config_mut()
            .breakpoints_mut()
            .insert(breakpoint);
    }

    /// Remove a specific breakpoint from consideration as a solution. If a breakpoint with
    /// this ID is encountered during fuzzing, the input will be saved as a solution.
    pub fn remove_breakpoint_solution(&mut self, breakpoint: BreakpointId) {
        info!(
            self.as_conf_object_mut(),
            "remove_breakpoint_solution({breakpoint})"
        );
        self.detector_mut()
            .config_mut()
            .breakpoints_mut()
            .remove(&breakpoint);
    }

    /// Set the coverage tracing mode to either "hit-count" (the default) or "once". The hit-count
    /// mode is slower, but much more accurate. "once" mode is faster, but is unable to capture
    /// coverage changes from multiple executions of the same code path (e.g. loops).
    pub fn set_tracing_mode(&mut self, mode: *mut c_char) -> Result<()> {
        let mode = unsafe { CStr::from_ptr(mode) }.to_str()?.to_string();

        info!(self.as_conf_object_mut(), "set_tracing_mode({mode})");

        match CoverageMode::from_str(&mode) {
            Ok(mode) => *self.tracer_mut().configuration_mut().coverage_mode_mut() = mode,
            Err(e) => error!(self.as_conf_object_mut(), "Error setting tracing mode: {e}"),
        }

        Ok(())
    }

    /// Set whether cmplog is enabled or disabled. Cmplog adds stages to trace and
    /// analyze comparison operands during target software execution and mutate test
    /// cases strategically using the logged operands. Execution speed is lower when
    /// running with cmplog enabled, but the efficiency gain from improved mutations
    /// typically makes up for the lost speed by many orders of magnitude. It is
    /// particularly well suited for software which performs magic value checks, large
    /// value and string comparisons, and sums.
    pub fn set_cmplog_enabled(&mut self, enabled: bool) {
        info!(self.as_conf_object_mut(), "set_cmplog_enabled({enabled})");

        *self.tracer_mut().configuration_mut().cmplog_mut() = enabled;
    }

    /// Set the directory path where the input corpus should be taken from when the
    /// fuzzer first starts, and where new corpus items will be saved. This path may be
    /// a SIMICS relative path prefixed with "%simics%". It is an error to provide no
    /// corpus directory when `set_generate_random_corpus(True)` has not been called
    /// prior to fuzzer startup. It is also an error to provide an *empty* corpus
    /// directory without calling `set_generate_random_corpus(True)`.  If not provided,
    /// "%simics%/corpus" will be used by default.
    pub fn set_corpus_directory(&mut self, corpus_directory: *mut c_char) -> Result<()> {
        let corpus_directory = unsafe { CStr::from_ptr(corpus_directory) }
            .to_str()?
            .to_string();

        info!(
            self.as_conf_object_mut(),
            "set_corpus_directory({corpus_directory})"
        );
        Ok(())
    }

    /// Set the directory path where solutions should be saved when the fuzzer finds them. This
    /// directory will contain the fuzzer inputs which triggered any solution condition that had
    /// been configured for the fuzzing campaign. These entries can be used to reproduce
    /// and traige defects using the `reproduce` method. If no solutions directory is provided,
    /// "%simics%/solutions" will be used by default.
    pub fn set_solutions_directory(&mut self, solutions_directory: *mut c_char) -> Result<()> {
        let solutions_directory = unsafe { CStr::from_ptr(solutions_directory) }
            .to_str()?
            .to_string();

        info!(
            self.as_conf_object_mut(),
            "set_solutions_directory({solutions_directory})"
        );
        Ok(())
    }

    /// Set whether a random corpus should be generated in the event that a corpus directory is
    /// not provided, or an empty corpus directory is provided. This option defaults to false
    /// because the penalty for using a random corpus is extremely high and corpus entries should
    /// be customized for the target software wherever possible. By setting this option, you
    /// should be aware your fuzz campaign's efficiency will be lowered. This is, however, very
    /// useful for demonstration and test purposes.
    pub fn set_generate_random_corpus(&mut self, generate_random_corpus: bool) -> Result<()> {
        info!(
            self.as_conf_object_mut(),
            "set_generate_random_corpus({generate_random_corpus})"
        );

        Ok(())
    }
}
