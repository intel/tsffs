// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use simics::api::{get_attribute, get_object};
use simics_macro::TryFromAttrValueTypeList;

#[derive(Debug, Clone, TryFromAttrValueTypeList)]
pub struct MicroCheckpointInfo {
    pub name: String,
    #[allow(unused)]
    pub pages: u64,
    #[allow(unused)]
    pub zero: u64,
}

pub struct Utils {}

impl Utils {
    /// Get the list of saved micro checkpoints
    pub fn get_micro_checkpoints() -> Result<Vec<MicroCheckpointInfo>> {
        Ok(get_attribute(get_object("sim.rexec")?, "state_info")?.try_into()?)
    }
}
