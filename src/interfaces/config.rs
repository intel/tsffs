// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{arch::ArchitectureHint, Tsffs};
use simics::{debug, get_processor_number, interface, AsConfObject, ConfObject, Result};
use std::{
    ffi::{c_char, CStr},
    str::FromStr,
};

#[interface(name = "config")]
impl Tsffs {
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
        self.architecture_hints
            .insert(processor_number, ArchitectureHint::from_str(hint)?);

        Ok(())
    }
}
