// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Context, Result};
use simics::api::{get_attribute, get_object};
use simics::FromAttrValueList;
use std::ffi::{c_char, CStr};

/// Convert a C string argument received through a Simics interface method, rejecting NULL.
///
/// # Safety
///
/// `ptr` must be NULL or point to a NUL-terminated string valid for the duration of this call.
pub(crate) unsafe fn c_str_arg(ptr: *const c_char, name: &str) -> Result<String> {
    ensure!(!ptr.is_null(), "{name} must not be NULL");
    Ok(CStr::from_ptr(ptr)
        .to_str()
        .with_context(|| format!("{name} must be valid UTF-8"))?
        .to_owned())
}

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
