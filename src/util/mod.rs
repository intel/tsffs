// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use simics::api::{get_attribute, get_object};
use simics::FromAttrValueList;

#[derive(Debug, Clone, FromAttrValueList)]
pub(crate) struct MicroCheckpointInfo {
    #[allow(unused)]
    pub name: String,
    #[allow(unused)]
    pub pages: i64,
    #[allow(unused)]
    pub zero: i64,
}

#[allow(unused)]
pub(crate) struct Utils;

#[allow(unused)]
impl Utils {
    /// Get the list of saved micro checkpoints
    pub fn get_micro_checkpoints() -> Result<Vec<MicroCheckpointInfo>> {
        let checkpoints: Vec<MicroCheckpointInfo> =
            get_attribute(get_object("sim.rexec")?, "state_info")?.try_into()?;

        Ok(checkpoints)
    }
}
