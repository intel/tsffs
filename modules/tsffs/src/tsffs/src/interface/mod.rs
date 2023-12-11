// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    arch::ArchitectureHint,
    fuzzer::tokenize::{tokenize_executable_file, tokenize_src_file},
    state::{ManualStart, ManualStartSize, Solution, SolutionKind, Stop, StopReason},
    Tsffs,
};
use anyhow::anyhow;
use ffi_macro::ffi;
use simics::{
    api::{
        continue_simulation, get_processor_number, lookup_file, run_alone, sys::attr_value_t,
        version, AsConfObject, AttrValue, AttrValueType, BreakpointId, ConfObject, GenericAddress,
    },
    debug, error, trace, Result,
};
use simics_macro::interface_impl;
use std::{
    ffi::{c_char, CStr},
    fs::read,
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
    /// * `rsi` - set to the address the fuzzer should write the testcase to each execution
    /// * `rdi` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # x86_32
    ///
    /// - Magic instruction executed with `n=1`
    /// * `esi` - set to the address the fuzzer should write the testcase to each execution
    /// * `edi` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # RISC-V
    ///
    /// - Magic instruction executed with `n=1`
    /// * `x10` - set to the address the fuzzer should write the testcase to each execution
    /// * `x11` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARM
    ///
    /// - Magic instruction executed with `n=1`
    /// * `r0` - set to the address the fuzzer should write the testcase to each execution
    /// * `r1` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARM Thumb-2
    ///
    /// - Magic instruction executed with `n=1`
    /// * `r0` - set to the address the fuzzer should write the testcase to each execution
    /// * `r1` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARMv8
    ///
    /// - Magic instruction executed with `n=1`
    /// * `x0` - set to the address the fuzzer should write the testcase to each execution
    /// * `x1` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    ///
    /// # ARC
    ///
    /// - Magic instruction executed with `n=1`
    /// * `r0` - set to the address the fuzzer should write the testcase to each execution
    /// * `r1` - set to the address of a variable containing the maximum size of a testcase,
    ///   which will be overwritten each execution with the current actual size of the testcase
    pub fn set_start_on_harness(&mut self, start_on_harness: bool) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "set_start_on_harness({start_on_harness})"
        );

        // self.set_start_on_harness(start_on_harness)?;
        *self.configuration_mut().start_on_harness_mut() = start_on_harness;

        Ok(())
    }

    /// Interface method to set the magic value the fuzzer will wait for when
    /// `set_start_on_harness` has ben configured. This allows you to place multiple harnesses in
    /// a single binary and selectively enable one of them.
    pub fn set_start_magic_number(&mut self, magic_number: i64) {
        debug!(
            self.as_conf_object(),
            "set_start_magic_number({magic_number})"
        );

        *self.configuration_mut().magic_start_mut() = magic_number;
    }

    /// Interface method to enable or disable the fuzzer to stop automatically when it
    /// reaches the default stop condition for the architecture of the processor that is
    /// running when the default stop condition occurs. Note that this method will not
    /// resume or run the simulation, the SIMICS script containing this call should
    /// resume execution afterward.
    pub fn set_stop_on_harness(&mut self, stop_on_harness: bool) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "set_stop_on_harness({stop_on_harness})"
        );

        // self.set_stop_on_harness(stop_on_harness)?;
        *self.configuration_mut().stop_on_harness_mut() = stop_on_harness;

        Ok(())
    }

    /// Interface method to set the magic value the fuzzer will wait for when
    /// `set_start_on_harness` has ben configured. This allows you to place multiple harnesses in
    /// a single binary and selectively enable one of them.
    pub fn set_stop_magic_number(&mut self, magic_number: i64) {
        debug!(
            self.as_conf_object(),
            "set_stop_magic_number({magic_number})"
        );

        *self.configuration_mut().magic_stop_mut() = magic_number;
    }

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving the
    /// testcase and size address and resuming execution of the simulation. This method does
    /// not need to be called if `set_start_on_harness` is enabled.
    ///
    /// # Arguments
    ///
    /// * `cpu` - The CPU whose memory space should be written
    /// * `testcase_address` - The address to write test cases to
    /// * `size_address` - The address to write the size of each test case to (optional,
    /// `max_size` must be given if not provided).
    ///
    /// If your target cannot take advantage of the written-back size pointer, use
    /// `start_with_max_size` instead.
    pub fn start(
        &mut self,
        cpu: *mut ConfObject,
        testcase_address: GenericAddress,
        size_address: GenericAddress,
        virt: bool,
    ) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "start({testcase_address:#x}, {size_address:#x})"
        );

        self.stop_simulation(StopReason::ManualStart(
            ManualStart::builder()
                .processor(cpu)
                .buffer(testcase_address)
                .size(ManualStartSize::SizeAddress(size_address))
                .virt(virt)
                .build(),
        ))?;

        Ok(())
    }

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving
    /// the testcase and maximum testcase size and resuming execution of the simulation.
    /// This method does not need to be called if `set_start_on_harness` is enabled.
    ///
    /// # Arguments
    ///
    /// * `cpu` - The CPU whose memory space should be written
    /// * `testcase_address` - The address to write test cases to
    /// * `maximum_size` - The maximum size of the test case. The actual size of each test case will
    ///   not be written back to the target software
    ///
    /// If your target does not have a buffer readily available to receive testcase data or
    /// you simply want to use it directly in some other way (e.g. by sending it to a network
    /// port), use `start_without_buffer`
    pub fn start_with_maximum_size(
        &mut self,
        cpu: *mut ConfObject,
        testcase_address: GenericAddress,
        maximum_size: u32,
        virt: bool,
    ) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "start_with_maximum_size({testcase_address:#x}, {maximum_size:#x})"
        );

        self.stop_simulation(StopReason::ManualStart(
            ManualStart::builder()
                .processor(cpu)
                .buffer(testcase_address)
                .size(ManualStartSize::MaximumSize(maximum_size as u64))
                .virt(virt)
                .build(),
        ))?;

        Ok(())
    }

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving
    /// the testcase and maximum testcase size and resuming execution of the simulation.
    /// This method does not need to be called if `set_start_on_harness` is enabled.
    ///
    /// # Arguments
    ///
    /// * `cpu` - The CPU to initially trace and post timeout events on. This should typically be
    ///   the CPU that is running the code receiving the input this function returns.
    ///
    /// # Return Value
    ///
    /// Returns an [`AttrValue`] list of integers. Integers are `u8` sized, in the range 0-255.
    pub fn start_without_buffer(&mut self, cpu: *mut ConfObject) -> Result<AttrValue> {
        if !self.have_initial_snapshot() {
            // Start the fuzzer thread early so we can get a testcase
            self.start_fuzzer_thread()?;
        }
        let testcase = self.get_testcase()?;
        *self.cmplog_enabled_mut() = testcase.cmplog_deref();
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

        self.stop_simulation(StopReason::ManualStart(
            ManualStart::builder().processor(cpu).build(),
        ))?;

        testcase.testcase_clone().try_into()
    }

    /// Interface method to manually signal to stop a testcase execution. When this
    /// method is called, the current testcase execution will be stopped as if it had
    /// finished executing normally, and the state will be restored to the state at the
    /// initial snapshot. This method is particularly useful in callbacks triggered on
    /// breakpoints or other complex conditions. This method does
    /// not need to be called if `set_stop_on_harness` is enabled.
    pub fn stop(&mut self) -> Result<()> {
        debug!(self.as_conf_object(), "stop");

        self.stop_simulation(StopReason::ManualStop(Stop::default()))?;

        Ok(())
    }

    /// Interface method to manually signal to stop execution with a solution condition.
    /// When this method is called, the current testcase execution will be stopped as if
    /// it had finished executing with an exception or timeout, and the state will be
    /// restored to the state at the initial snapshot.
    pub fn solution(&mut self, id: u64, message: *mut c_char) -> Result<()> {
        let message = unsafe { CStr::from_ptr(message) }.to_str()?;

        debug!(self.as_conf_object(), "solution({id:#x}, {message})");

        self.stop_simulation(StopReason::Solution(
            Solution::builder().kind(SolutionKind::Manual).build(),
        ))?;

        Ok(())
    }

    /// Interface method to set the fuzzer to use the experimental snapshots interface
    /// instead of the micro checkpoints interface for snapshot save and restore operations
    pub fn set_use_snapshots(&mut self, use_snapshots: bool) -> Result<()> {
        debug!(self.as_conf_object(), "use_snapshots({use_snapshots})");

        #[cfg(not(simics_experimental_api_snapshots))]
        {}

        if cfg!(simics_experimental_api_snapshots) {
            *self.configuration_mut().use_snapshots_mut() = use_snapshots;
        } else if !cfg!(simics_experimental_api_snapshots) && use_snapshots {
            let version = version()?;

            error!(
                self.as_conf_object(),
                "Not enabling snapshots, API is unsupported for target SIMICS version {version}",
            );
        } else {
            // NOTE: We don't report an error if snapshots are turned off when they are unsupported
        }

        Ok(())
    }

    /// Interface method to set the execution timeout in seconds
    pub fn set_timeout(&mut self, timeout: f64) {
        debug!(self.as_conf_object(), "set_timeout({timeout})");

        *self.configuration_mut().timeout_mut() = timeout;
    }

    /// Interface method to add an exception-type solution number to the set of
    /// exception-type solution numbers currently being monitored for. If any exception in
    /// the set of exceptions currently monitored occurs, the testcase will be saved and
    /// reported as a solution.
    ///
    /// For example on x86_64, `add_exception_solution(14)` would treat any page fault as
    /// a solution.
    pub fn add_exception_solution(&mut self, exception: i64) {
        debug!(self.as_conf_object(), "add_exception_solution({exception})");

        self.configuration_mut().exceptions_mut().insert(exception);
    }

    /// Interface method to remove an exception-type solution number from the set of
    /// exception-type solution numbers currently being monitored for. If any exception in
    /// the set of solutions currently monitored occurs, the testcase will be saved and
    /// reported as a solution.
    pub fn remove_exception_solution(&mut self, exception: i64) {
        debug!(
            self.as_conf_object(),
            "remove_exception_solution({exception})"
        );

        self.configuration_mut().exceptions_mut().remove(&exception);
    }

    /// Set whether all CPU exceptions are considered solutions. If set to true, any
    /// exception encountered during fuzzing will be saved as a solution. This is typically
    /// not desired.
    pub fn set_all_exceptions_are_solutions(&mut self, all_exceptions_are_solutions: bool) {
        debug!(
            self.as_conf_object(),
            "set_all_exceptions_are_solutions({all_exceptions_are_solutions})"
        );

        *self.configuration_mut().all_exceptions_are_solutions_mut() = all_exceptions_are_solutions;
    }

    /// Set a specific breakpoint number to be considered a solution. If a breakpoint with
    /// this ID is encountered during fuzzing, the input will be saved as a solution.
    pub fn add_breakpoint_solution(&mut self, breakpoint: BreakpointId) {
        debug!(
            self.as_conf_object(),
            "add_breakpoint_solution({breakpoint})"
        );

        self.configuration_mut()
            .breakpoints_mut()
            .insert(breakpoint);
    }

    /// Remove a specific breakpoint from consideration as a solution. If a breakpoint with
    /// this ID is encountered during fuzzing, the input will be saved as a solution.
    pub fn remove_breakpoint_solution(&mut self, breakpoint: BreakpointId) {
        debug!(
            self.as_conf_object(),
            "remove_breakpoint_solution({breakpoint})"
        );
        self.configuration_mut()
            .breakpoints_mut()
            .remove(&breakpoint);
    }

    /// Set whether all SIMICS breakpoints are considered solutions. If set to true, any
    /// breakpoint (read, write, or execute) encountered during fuzzing will be saved as
    /// a solution.
    pub fn set_all_breakpoints_are_solutions(&mut self, all_breakpoints_are_solutions: bool) {
        debug!(
            self.as_conf_object(),
            "set_all_breakpoints_are_solutions({all_breakpoints_are_solutions})"
        );

        *self.configuration_mut().all_breakpoints_are_solutions_mut() =
            all_breakpoints_are_solutions;
    }

    /// Set whether cmplog is enabled or disabled. Cmplog adds stages to trace and
    /// analyze comparison operands during target software execution and mutate test
    /// cases strategically using the logged operands. Execution speed is lower when
    /// running with cmplog enabled, but the efficiency gain from improved mutations
    /// typically makes up for the lost speed by many orders of magnitude. It is
    /// particularly well suited for software which performs magic value checks, large
    /// value and string comparisons, and sums.
    pub fn set_cmplog_enabled(&mut self, enabled: bool) {
        debug!(self.as_conf_object(), "set_cmplog_enabled({enabled})");

        *self.configuration_mut().cmplog_mut() = enabled;
    }

    /// Set the directory path where the input corpus should be taken from when the
    /// fuzzer first starts, and where new corpus items will be saved. This path may be
    /// a SIMICS relative path prefixed with "%simics%". It is an error to provide no
    /// corpus directory when `set_generate_random_corpus(True)` has not been called
    /// prior to fuzzer startup. It is also an error to provide an *empty* corpus
    /// directory without calling `set_generate_random_corpus(True)`.  If not provided,
    /// "%simics%/corpus" will be used by default.
    pub fn set_corpus_directory(&mut self, corpus_directory: *mut c_char) -> Result<()> {
        let corpus_directory_path = unsafe { CStr::from_ptr(corpus_directory) }.to_str()?;

        if let Ok(corpus_directory) = lookup_file(corpus_directory_path) {
            debug!(
                self.as_conf_object(),
                "set_corpus_directory({})",
                corpus_directory.display(),
            );

            *self.configuration_mut().corpus_directory_mut() = corpus_directory;
        } else {
            error!(self.as_conf_object(), "Corpus directory cannot be set. The requested directory {corpus_directory_path} does not exist.");
        }

        Ok(())
    }

    /// Set the directory path where solutions should be saved when the fuzzer finds them. This
    /// directory will contain the fuzzer inputs which triggered any solution condition that had
    /// been configured for the fuzzing campaign. These entries can be used to reproduce
    /// and traige defects using the `reproduce` method. If no solutions directory is provided,
    /// "%simics%/solutions" will be used by default.
    pub fn set_solutions_directory(&mut self, solutions_directory: *mut c_char) -> Result<()> {
        let solutions_directory_path = unsafe { CStr::from_ptr(solutions_directory) }.to_str()?;

        if let Ok(solutions_directory) = lookup_file(solutions_directory_path) {
            debug!(
                self.as_conf_object(),
                "set_solutions_directory({})",
                solutions_directory.display(),
            );

            *self.configuration_mut().solutions_directory_mut() = solutions_directory;
        } else {
            error!(self.as_conf_object(), "Solutions directory cannot be set. The requested directory {solutions_directory_path} does not exist.");
        }

        Ok(())
    }

    /// Set whether a random corpus should be generated in the event that a corpus directory is
    /// not provided, or an empty corpus directory is provided. This option defaults to false
    /// because the penalty for using a random corpus is extremely high and corpus entries should
    /// be customized for the target software wherever possible. By setting this option, you
    /// should be aware your fuzz campaign's efficiency will be lowered. This is, however, very
    /// useful for demonstration and test purposes.
    pub fn set_generate_random_corpus(&mut self, generate_random_corpus: bool) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "set_generate_random_corpus({generate_random_corpus})"
        );
        *self.configuration_mut().generate_random_corpus_mut() = generate_random_corpus;

        Ok(())
    }

    /// Set the number of iterations to run the fuzzer for. This is the number of actual testcases
    /// executed, and includes all stages (e.g. calibration). This should typically not be used
    /// to limit the time of a fuzzing campaign, and is only useful for demonstration purposes.
    pub fn set_iterations(&mut self, iterations: usize) -> Result<()> {
        debug!(self.as_conf_object(), "set_iterations({iterations})");
        *self.configuration_mut().iterations_mut() = Some(iterations);

        Ok(())
    }

    pub fn get_configuration(&mut self) -> Result<attr_value_t> {
        let value: AttrValueType = self.configuration_clone().try_into()?;
        Ok(AttrValue::try_from(value)?.into())
    }

    /// Tokenize an executable file and add extracted tokens to token mutations for the fuzzer
    pub fn tokenize_executable(&mut self, executable_file: *mut c_char) -> Result<()> {
        let simics_path = unsafe { CStr::from_ptr(executable_file) }.to_str()?;

        let executable_path = lookup_file(simics_path)?;

        debug!(
            self.as_conf_object(),
            "tokenize_executable({})",
            executable_path.display()
        );

        self.configuration_mut()
            .tokens_mut()
            .extend(tokenize_executable_file(executable_path)?);

        Ok(())
    }

    /// Tokenize a source file and add extracted tokens to token mutations for the fuzzer
    pub fn tokenize_src(&mut self, source_file: *mut c_char) -> Result<()> {
        let simics_path = unsafe { CStr::from_ptr(source_file) }.to_str()?;

        let source_path = lookup_file(simics_path)?;

        debug!(
            self.as_conf_object(),
            "tokenize_src({})",
            source_path.display()
        );

        self.configuration_mut().tokens_mut().extend(
            tokenize_src_file([source_path])?
                .iter()
                .map(|e| e.as_bytes().to_vec())
                .collect::<Vec<_>>(),
        );

        Ok(())
    }

    /// Add tokens from a file of the format below, containing tokens extracted from the fuzz
    /// target:
    /// ```text,ignore
    /// x = "hello"
    /// y = "foo\x41bar"
    /// ```
    pub fn add_token_file(&mut self, token_file: *mut c_char) -> Result<()> {
        let simics_path = unsafe { CStr::from_ptr(token_file) }.to_str()?;

        let token_file = lookup_file(simics_path)?;

        debug!(
            self.as_conf_object(),
            "add_token_file({})",
            token_file.display()
        );

        if token_file.is_file() {
            self.configuration_mut().token_files_mut().push(token_file);
        }

        Ok(())
    }

    /// Add a processor to be traced. By default, only the processor the start event occurs on
    /// is used for tracing.
    pub fn add_trace_processor(&mut self, cpu: *mut ConfObject) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "add_trace_processor({:#x})", cpu as usize
        );

        self.add_processor(cpu, false)?;

        Ok(())
    }

    /// Set an architecture hint to be used for a particular processor. This allows overriding
    /// the detected or reported architecture for the processor object. This is particularly
    /// useful for x86 processors which report as x86-64 processors, or when fuzzing x86 code
    /// running on an x86-64 processor in a backward compatibility mode.
    pub fn add_architecture_hint(&mut self, cpu: *mut ConfObject, hint: *mut c_char) -> Result<()> {
        let hint = unsafe { CStr::from_ptr(hint) }.to_str()?;
        let processor_number = get_processor_number(cpu)?;
        debug!(
            self.as_conf_object(),
            "add_architecture_hint({processor_number}, {hint})"
        );
        self.configuration_mut()
            .architecture_hints_mut()
            .insert(processor_number, ArchitectureHint::from_str(hint)?);

        Ok(())
    }

    /// Reproduce a test case execution. This will set the fuzzer's next input through
    /// one execution using the provided file as input instead of taking input from the
    /// fuzzer. It will stop execution at the first stop, timeout, or other solution
    /// instead of continuing the fuzzing loop.
    ///
    /// This can be called during configuration *or* after stopping the fuzzer once a solution
    /// has been found.
    pub fn repro(&mut self, testcase_file: *mut c_char) -> Result<()> {
        let simics_path = unsafe { CStr::from_ptr(testcase_file) }.to_str()?;

        let testcase_file = lookup_file(simics_path)?;

        debug!(self.as_conf_object(), "repro({})", testcase_file.display());

        let contents = read(&testcase_file).map_err(|e| {
            anyhow!(
                "Failed to read repro testcase file {}: {}",
                testcase_file.display(),
                e
            )
        })?;

        *self.repro_testcase_mut() = Some(contents);

        if self.iterations_deref() > 0 {
            // We've done an iteration already, so we need to reset and run
            self.restore_initial_snapshot()?;
            self.get_and_write_testcase()?;
            self.post_timeout_event()?;

            run_alone(|| {
                continue_simulation(0)?;
                Ok(())
            })?;
        }

        Ok(())
    }
}
