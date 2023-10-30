// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Definitions for tracking the state of the fuzzer

use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Definition of all the reasons the simulator could be stopped by the fuzzer. In general,
/// callbacks in the fuzzer, for example [`Driver::on_magic_instruction`] may be called
/// asynchronously and stop the simulation.
pub enum StopReason {
    MagicStart,
    MagicStop,
    Start,
    Stop,
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
