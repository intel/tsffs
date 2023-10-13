// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use simics_api_sys::{
    sim_exception, SIM_clear_exception, SIM_get_pending_exception, SIM_last_error,
};
use std::ffi::CStr;

pub type SimException = sim_exception;

/// Get the last SIMICS error as a string
pub fn last_error() -> Error {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}

pub fn clear_exception() -> SimException {
    unsafe { SIM_clear_exception() }
}

pub fn get_pending_exception() -> SimException {
    unsafe { SIM_get_pending_exception() }
}
