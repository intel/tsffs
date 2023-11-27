// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::{sys::SIM_get_debugger, ConfObject};
use simics_macro::simics_exception;

#[simics_exception]
pub fn get_debugger() -> *mut ConfObject {
    unsafe { SIM_get_debugger() }
}
