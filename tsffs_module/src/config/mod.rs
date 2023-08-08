// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Configuration data for the module, passed to it when it starts up

use crate::faults::Fault;
use anyhow::{bail, Error, Result};
use derive_builder::Builder;
use libafl::prelude::AFLppCmpMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, str::FromStr};
use tracing::metadata::LevelFilter;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
/// Tracing mode for the branch tracer
pub enum TraceMode {
    /// Trace each instruction once, this does not necessarily enable hit counting,
    /// and will be less precise than `HitCount` but is significantly (2x+) faster
    Once,
    /// Trace each instruction every time it is executed
    HitCount,
}

impl Default for TraceMode {
    fn default() -> Self {
        Self::HitCount
    }
}

impl ToString for TraceMode {
    fn to_string(&self) -> String {
        match self {
            TraceMode::Once => "once",
            TraceMode::HitCount => "hit_count",
        }
        .to_string()
    }
}

impl FromStr for TraceMode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s.to_lowercase().as_str() {
            "once" => Self::Once,
            "hit_count" => Self::HitCount,
            "hitcount" => Self::HitCount,
            _ => bail!("No such trace mode {}", s),
        })
    }
}

#[derive(Builder, Debug, Clone)]
/// Contains parameters for the module to configure things like timeout duration, which faults
/// indicate a crash, etc. This is sent by the client in `ClientMessage::Initialize`
pub struct InputConfig {
    #[builder(setter(each(name = "fault")), default)]
    pub faults: HashSet<Fault>,
    pub timeout: f64,
    pub log_level: LevelFilter,
    pub trace_mode: TraceMode,
    pub coverage_map: (*mut u8, usize),
    pub cmp_map: *mut AFLppCmpMap,
    /// If repro is set, the simics thread will drop into the CLI on the first exception
    #[builder(default)]
    pub repro: bool,
}

unsafe impl Send for InputConfig {}
unsafe impl Sync for InputConfig {}

impl InputConfig {
    /// Add a fault to the set of faults considered crashes for a given fuzzing campaign
    pub fn with_fault(mut self, fault: Fault) -> Self {
        self.faults.insert(fault);
        self
    }

    /// Add one or more faults to the set of faults considered crashes for a given fuzzing
    /// campaign
    pub fn with_faults<I>(mut self, faults: I) -> Self
    where
        I: IntoIterator<Item = Fault>,
    {
        faults.into_iter().for_each(|i| {
            self.faults.insert(i);
        });
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout_seconds(mut self, seconds: f64) -> Self {
        self.timeout = seconds;
        self
    }

    pub fn with_timeout_milliseconds(mut self, milliseconds: f64) -> Self {
        self.timeout = milliseconds / 1000.0;
        self
    }

    pub fn with_timeout_microseconds(mut self, microseconds: f64) -> Self {
        self.timeout = microseconds / 1_000_000.0;
        self
    }

    /// Set the trace mode to either once or hitcount
    pub fn with_trace_mode(mut self, mode: TraceMode) -> Self {
        self.trace_mode = mode;
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
/// Contains the resulting configuration of the module after initialization with the provided
/// `InputConfig`. This is used to pass memory maps back to the client for things like
/// coverage and cmplog data, but can be extended.
pub struct OutputConfig {}

impl OutputConfig {}
