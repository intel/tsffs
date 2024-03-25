// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state::{SolutionKind, StopReason},
    ManualStartAddress, ManualStartInfo, ManualStartSize, Tsffs,
};
use anyhow::{anyhow, Result};
use libafl::inputs::HasBytesVec;
use simics::{
    continue_simulation, debug, interface, lookup_file, run_alone, AsConfObject, AttrValue,
    ConfObject, GenericAddress,
};
use std::{
    ffi::{c_char, CStr},
    fs::read,
};

#[interface(name = "fuzz")]
impl Tsffs {
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

        self.repro_testcase = Some(contents);

        if self.iterations > 0 {
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
    pub fn start_with_buffer_ptr_size_ptr(
        &mut self,
        cpu: *mut ConfObject,
        buffer_address: GenericAddress,
        size_address: GenericAddress,
        virt: bool,
    ) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "start({buffer_address:#x}, {size_address:#x})"
        );

        self.stop_simulation(StopReason::ManualStart {
            processor: cpu,
            info: ManualStartInfo {
                address: if virt {
                    ManualStartAddress::Virtual(buffer_address)
                } else {
                    ManualStartAddress::Physical(buffer_address)
                },
                size: ManualStartSize::SizePtr {
                    address: if virt {
                        ManualStartAddress::Virtual(size_address)
                    } else {
                        ManualStartAddress::Physical(size_address)
                    },
                },
            },
        })?;

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
    pub fn start_with_buffer_ptr_size_value(
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

        self.stop_simulation(StopReason::ManualStart {
            processor: cpu,
            info: ManualStartInfo {
                address: if virt {
                    ManualStartAddress::Virtual(testcase_address)
                } else {
                    ManualStartAddress::Physical(testcase_address)
                },
                size: ManualStartSize::MaxSize(maximum_size.try_into()?),
            },
        })?;

        Ok(())
    }

    /// Interface method to manually start the fuzzing loop by taking a snapshot, saving
    /// the testcase, size address, and maximum testcase size and resuming execution of the
    /// simulation. This method does not need to be called if `set_start_on_harness` is enabled.
    ///
    /// # Arguments
    ///
    /// * `cpu` - The CPU whose memory space should be written
    /// * `testcase_address` - The address to write test cases to
    /// * `size_address` - The address to write the size of each test case to
    /// * `maximum_size` - The maximum size of the test case. The actual size of each test case will
    ///   be written back to the target software at the provided size address.
    ///
    /// If your target cannot take advantage of the written-back size pointer, use
    /// `start_with_max_size` instead.
    pub fn start_with_buffer_ptr_size_ptr_value(
        &mut self,
        cpu: *mut ConfObject,
        buffer_address: GenericAddress,
        size_address: GenericAddress,
        maximum_size: u32,
        virt: bool,
    ) -> Result<()> {
        debug!(
            self.as_conf_object(),
            "start({buffer_address:#x}, {size_address:#x}, {maximum_size:#x})"
        );

        self.stop_simulation(StopReason::ManualStart {
            processor: cpu,
            info: ManualStartInfo {
                address: if virt {
                    ManualStartAddress::Virtual(buffer_address)
                } else {
                    ManualStartAddress::Physical(buffer_address)
                },
                size: ManualStartSize::SizePtrAndMaxSize {
                    address: if virt {
                        ManualStartAddress::Virtual(size_address)
                    } else {
                        ManualStartAddress::Physical(size_address)
                    },
                    maximum_size: maximum_size.try_into()?,
                },
            },
        })?;

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

        self.stop_simulation(StopReason::ManualStartWithoutBuffer { processor: cpu })?;

        Ok(testcase.testcase.bytes().to_vec().try_into()?)
    }

    /// Interface method to manually signal to stop a testcase execution. When this
    /// method is called, the current testcase execution will be stopped as if it had
    /// finished executing normally, and the state will be restored to the state at the
    /// initial snapshot. This method is particularly useful in callbacks triggered on
    /// breakpoints or other complex conditions. This method does
    /// not need to be called if `set_stop_on_harness` is enabled.
    pub fn stop(&mut self) -> Result<()> {
        debug!(self.as_conf_object(), "stop");

        self.stop_simulation(StopReason::ManualStop)?;

        Ok(())
    }

    /// Interface method to manually signal to stop execution with a solution condition.
    /// When this method is called, the current testcase execution will be stopped as if
    /// it had finished executing with an exception or timeout, and the state will be
    /// restored to the state at the initial snapshot.
    pub fn solution(&mut self, id: u64, message: *mut c_char) -> Result<()> {
        let message = unsafe { CStr::from_ptr(message) }.to_str()?;

        debug!(self.as_conf_object(), "solution({id:#x}, {message})");

        self.stop_simulation(StopReason::Solution {
            kind: SolutionKind::Manual,
        })?;

        Ok(())
    }
}
