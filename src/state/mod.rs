// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Definitions for tracking the state of the fuzzer

use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use simics::api::ConfObject;
use std::{
    fmt::{Display, Formatter},
    ptr::null_mut,
    str::FromStr,
};
use typed_builder::TypedBuilder;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum ManualStartSize {
    MaximumSize(u64),
    SizeAddress(u64),
    NoSize,
}

#[derive(TypedBuilder, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ManualStart {
    #[builder(default = null_mut())]
    #[serde(skip, default = "null_mut")]
    pub processor: *mut ConfObject,
    #[builder(default, setter(into, strip_option))]
    pub buffer: Option<u64>,
    #[builder(default = ManualStartSize::NoSize)]
    pub size: ManualStartSize,
    #[builder(default)]
    pub virt: bool,
}

#[derive(TypedBuilder, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct MagicStart {
    #[builder(default = null_mut())]
    #[serde(skip, default = "null_mut")]
    pub processor: *mut ConfObject,
}

#[derive(TypedBuilder, Serialize, Deserialize, Debug, Clone, Default)]
pub(crate) struct Stop {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum SolutionKind {
    Timeout,
    Exception,
    Breakpoint,
    Manual,
}

#[derive(TypedBuilder, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Solution {
    pub kind: SolutionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Definition of all the reasons the simulator could be stopped by the fuzzer. In general,
/// callbacks in the fuzzer, for example [`Driver::on_magic_instruction`] may be called
/// asynchronously and stop the simulation.
pub(crate) enum StopReason {
    MagicStart(MagicStart),
    MagicStop(Stop),
    ManualStart(ManualStart),
    ManualStop(Stop),
    Solution(Solution),
}

impl Display for StopReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_string(self).unwrap_or_default())
    }
}

impl FromStr for StopReason {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        from_str(s).map_err(|e| anyhow!("Failed to deserialize from string: {e}"))
    }
}
