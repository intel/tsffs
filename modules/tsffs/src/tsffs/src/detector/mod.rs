// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use ffi_macro::ffi;
use getters::Getters;
use simics::api::{BreakpointId, ConfObject, GenericTransaction};
use simics_macro::{TryFromAttrValueTypeDict, TryIntoAttrValueTypeDict};
use std::{collections::BTreeSet, ffi::c_void};
use typed_builder::TypedBuilder;

use crate::{state::StopReason, traits::Component, Tsffs};

/// The timeout runs in virtual time, so a typical 5 second timeout is acceptable
pub const TIMEOUT_DEFAULT: f64 = 5.0;

#[derive(
    TypedBuilder, Getters, Debug, Clone, TryIntoAttrValueTypeDict, TryFromAttrValueTypeDict,
)]
#[getters(mutable)]
/// Configuration of the fuzzer of each condition that can be treated as a fault
pub struct DetectorConfiguration {
    #[builder(default = false)]
    /// Whether any breakpoint that occurs during fuzzing is treated as a fault
    all_breakpoints_are_solutions: bool,
    #[builder(default = false)]
    /// Whether any CPU exception that occurs during fuzzing is treated as a solution
    all_exceptions_are_solutions: bool,
    #[builder(default)]
    /// The set of specific exception numbers that are treated as a solution
    exceptions: BTreeSet<i64>,
    #[builder(default)]
    /// The set of breakpoints to treat as solutions
    breakpoints: BTreeSet<BreakpointId>,
    #[builder(default = TIMEOUT_DEFAULT)]
    /// The amount of time in seconds before a testcase execution is considered "timed
    /// out" and will be treated as a solution
    timeout: f64,
}

impl Default for DetectorConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Detector<'a>
where
    'a: 'static,
{
    parent: &'a Tsffs,
    #[builder(default)]
    configuration: DetectorConfiguration,
}

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl<'a> Detector<'a>
where
    'a: 'static,
{
    // NOTE: Core_External_Interrupt also exists, but is only for SPARC, so we do not support it
    #[ffi(arg(self), arg(rest))]
    pub fn on_core_exception(&mut self, _obj: *mut ConfObject, _exception: i64) -> Result<()> {
        Ok(())
    }

    #[ffi(arg(self), arg(rest))]
    pub fn on_core_breakpoint(
        &mut self,
        _obj: *mut ConfObject,
        _breakpoint: i64,
        _transaction: *mut GenericTransaction,
    ) -> Result<()> {
        Ok(())
    }
}

impl<'a> Component for Detector<'a> {
    fn on_simulation_stopped(&mut self, _reason: &StopReason) -> Result<()> {
        Ok(())
    }
}
