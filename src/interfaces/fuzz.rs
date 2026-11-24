// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state::{SolutionKind, StopReason},
    util::c_str_arg,
    ManualStartAddress, ManualStartInfo, ManualStartSize, Tsffs,
};
use anyhow::{anyhow, ensure, Result};
use libafl::inputs::HasBytesVec;
use simics::{debug, interface, lookup_file, AsConfObject, AttrValue, ConfObject, GenericAddress};
use std::{
    collections::VecDeque,
    ffi::c_char,
    fs::{read, read_dir},
    io,
};

extern crate ffi2 as ffi;

#[interface(name = "fuzz")]
impl Tsffs {
    /// Reproduce test case executions. Instead of taking input from the fuzzer, TSFFS runs
    /// the provided file, or every file of the provided directory, one after the other,
    /// each from the initial snapshot. Execution stops at the stop, timeout, or other
    /// solution of the last test case instead of continuing the fuzzing loop. How each
    /// test case ended is logged.
    ///
    /// The test cases of a directory are the regular files directly inside it, sorted by
    /// name. Hidden files are skipped: the fuzzer keeps lock and metadata files next to the
    /// corpus entries.
    ///
    /// This can be called during configuration *or* after stopping the fuzzer once a solution
    /// has been found. It can be called again after the reproduced executions have stopped.
    pub fn repro(&mut self, testcase_file: *mut c_char) -> Result<()> {
        // SAFETY: Simics passes NULL or a NUL-terminated string valid for this call
        let simics_path = unsafe { c_str_arg(testcase_file, "testcase_file") }?;

        let testcase_file = lookup_file(&simics_path)?;

        debug!(self.as_conf_object(), "repro({})", testcase_file.display());

        let mut files = if testcase_file.is_dir() {
            let mut files = read_dir(&testcase_file)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<io::Result<Vec<_>>>()?;

            files.retain(|file| {
                file.is_file()
                    && !file
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            });

            files
        } else {
            vec![testcase_file.clone()]
        };

        files.sort();

        let inputs = files
            .into_iter()
            .map(|file| {
                let contents = read(&file).map_err(|e| {
                    anyhow!(
                        "Failed to read repro testcase file {}: {}",
                        file.display(),
                        e
                    )
                })?;

                Ok((file, contents))
            })
            .collect::<Result<VecDeque<_>>>()?;

        ensure!(
            !inputs.is_empty(),
            "No testcase to reproduce in {}",
            testcase_file.display()
        );

        self.replay = Some(inputs);

        if self.have_initial_snapshot() {
            // The harness start was already reached, while fuzzing or in an earlier repro
            // run: run the first testcase from the snapshot, as the harness start would.
            // What the previous execution reported does not describe this testcase
            self.last_exception_processor_number = None;
            self.last_stop_reason = None;
            // Restoring the snapshot also restores this object's attributes: keep the
            // `repro_auto_continue` the user has set by now.
            let repro_auto_continue = self.repro_auto_continue;
            self.start_next_execution(
                true,
                "No start buffer, the harness start hands over the testcase.",
            )?;
            self.repro_auto_continue = repro_auto_continue;
            self.execution_trace.0.clear();
            self.resume_after_start(true)?;
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
    ///
    /// TSFFS has no buffer to write the next testcase to, so this method is called again
    /// each time the restored snapshot reaches the start, and returns that execution's
    /// testcase: the next one from the fuzzer, or in repro mode the one being reproduced.
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
    /// finished executing normally, and the state will be restored according to
    /// `snapshot_restore_interval` (restoring every iteration by default). This method is
    /// particularly useful in callbacks triggered on breakpoints or other complex
    /// conditions. This method does
    /// not need to be called if `set_stop_on_harness` is enabled.
    pub fn stop(&mut self) -> Result<()> {
        debug!(self.as_conf_object(), "stop");

        self.stop_simulation(StopReason::ManualStop)?;

        Ok(())
    }

    /// Interface method to manually signal to stop execution with a solution condition.
    /// When this method is called, the current testcase execution will be stopped as if
    /// it had finished executing with an exception or timeout, and, if an initial
    /// snapshot exists, it will be restored before the next iteration resumes.
    pub fn solution(&mut self, id: u64, message: *mut c_char) -> Result<()> {
        // SAFETY: Simics passes NULL or a NUL-terminated string valid for this call
        let message = unsafe { c_str_arg(message, "message") }?;

        debug!(self.as_conf_object(), "solution({id:#x}, {message})");

        self.stop_simulation(StopReason::Solution {
            kind: SolutionKind::Manual,
        })?;

        Ok(())
    }
}
