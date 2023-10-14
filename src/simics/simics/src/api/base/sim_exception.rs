// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::sys::{
    sim_exception, SIM_clear_exception, SIM_get_pending_exception, SIM_last_error,
};
use std::ffi::CStr;

pub type SimException = sim_exception;

/// Get the last SIMICS error message as a string
pub fn last_error() -> String {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}

/// Clear a SIMICS exception, if there is one, and return it. Returns
/// [`SimException::Sim_No_Exception`] if none exists
pub fn clear_exception() -> SimException {
    unsafe { SIM_clear_exception() }
}

/// Return a pending simics exception. Returns [`SimException::Sim_No_Excception`] if none exists
pub fn get_pending_exception() -> SimException {
    unsafe { SIM_get_pending_exception() }
}
