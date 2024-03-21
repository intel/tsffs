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

use crate::{magic::MagicNumber, ManualStartInfo};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum SolutionKind {
    Timeout,
    Exception,
    Breakpoint,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Definition of all the reasons the simulator could be stopped by the fuzzer. In general,
/// callbacks in the fuzzer, for example [`Driver::on_magic_instruction`] may be called
/// asynchronously and stop the simulation.
pub(crate) enum StopReason {
    Magic {
        magic_number: MagicNumber,
    },
    ManualStart {
        #[serde(skip, default = "null_mut")]
        processor: *mut ConfObject,
        info: ManualStartInfo,
    },
    ManualStartWithoutBuffer {
        #[serde(skip, default = "null_mut")]
        processor: *mut ConfObject,
    },
    ManualStop,
    Solution {
        kind: SolutionKind,
    },
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
