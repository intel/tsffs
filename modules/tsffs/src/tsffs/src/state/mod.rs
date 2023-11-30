// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Definitions for tracking the state of the fuzzer

use anyhow::{anyhow, Error, Result};
use getters::Getters;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use simics::api::ConfObject;
use std::{ptr::null_mut, str::FromStr};
use typed_builder::TypedBuilder;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ManualStartSize {
    MaximumSize(u64),
    SizeAddress(u64),
    NoSize,
}

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Debug, Clone)]
#[getters(mutable)]
pub struct ManualStart {
    #[builder(default = null_mut())]
    #[serde(skip, default = "null_mut")]
    processor: *mut ConfObject,
    #[builder(default, setter(into, strip_option))]
    buffer: Option<u64>,
    #[builder(default = ManualStartSize::NoSize)]
    size: ManualStartSize,
    #[builder(default)]
    virt: bool,
}

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Debug, Clone)]
#[getters(mutable)]
pub struct MagicStart {
    #[builder(default = null_mut())]
    #[serde(skip, default = "null_mut")]
    processor: *mut ConfObject,
}

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Debug, Clone, Default)]
#[getters(mutable)]
pub struct Stop {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SolutionKind {
    Timeout,
    Exception,
    Breakpoint,
    Manual,
}

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Debug, Clone)]
#[getters(mutable)]
pub struct Solution {
    kind: SolutionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Definition of all the reasons the simulator could be stopped by the fuzzer. In general,
/// callbacks in the fuzzer, for example [`Driver::on_magic_instruction`] may be called
/// asynchronously and stop the simulation.
pub enum StopReason {
    MagicStart(MagicStart),
    MagicStop(Stop),
    ManualStart(ManualStart),
    ManualStop(Stop),
    Solution(Solution),
}

impl ToString for StopReason {
    fn to_string(&self) -> String {
        to_string(self).expect("Failed to serialize to string")
    }
}

impl FromStr for StopReason {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        from_str(s).map_err(|e| anyhow!("Failed to deserialize from string: {e}"))
    }
}
