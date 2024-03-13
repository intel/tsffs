// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Debugger control

use crate::{simics_exception, sys::SIM_get_debugger, ConfObject};

#[simics_exception]
/// Return the current debugger if one is active
pub fn get_debugger() -> *mut ConfObject {
    unsafe { SIM_get_debugger() }
}
