// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use getters::Getters;
use simics::api::BreakpointId;
use std::collections::HashSet;
use typed_builder::TypedBuilder;

/// The timeout runs in virtual time, so a typical 5 second timeout is acceptable
pub const TIMEOUT_DEFAULT: f64 = 5.0;

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
/// Configuration of the fuzzer of each condition that can be treated as a fault
pub struct SolutionConfiguration {
    #[builder(default = false)]
    /// Whether any breakpoint that occurs during fuzzing is treated as a fault
    all_breakpoints_are_solutions: bool,
    #[builder(default = false)]
    /// Whether any CPU exception that occurs during fuzzing is treated as a solution
    all_exceptions_are_solutions: bool,
    #[builder(default)]
    /// The set of specific exception numbers that are treated as a solution
    exceptions: HashSet<i64>,
    #[builder(default)]
    /// The set of breakpoints to treat as solutions
    breakpoints: HashSet<BreakpointId>,
    #[builder(default = TIMEOUT_DEFAULT)]
    /// The amount of time in seconds before a testcase execution is considered "timed
    /// out" and will be treated as a solution
    timeout: f64,
}

impl Default for SolutionConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Detector {
    config: SolutionConfiguration,
}

impl Default for Detector {
    fn default() -> Self {
        Self::builder()
            .config(SolutionConfiguration::default())
            .build()
    }
}