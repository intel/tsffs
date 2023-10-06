// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::{last_error, ConfObject};
use anyhow::{bail, Result};
use simics_api_sys::SIM_object_clock;

/// Get the clock of an object that implements the required clock interface
pub fn object_clock(obj: *mut ConfObject) -> Result<*mut ConfObject> {
    let clock = unsafe { SIM_object_clock(obj as *const ConfObject) };

    if clock.is_null() {
        bail!("Unable to get object clock: {}", last_error());
    } else {
        Ok(clock.into())
    }
}
