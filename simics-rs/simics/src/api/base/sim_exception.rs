// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS exception handling APIs. These should typically not be used directly, most SIMICS
//! api functions provided by this crate use the `#[simics_exception]` attribute to automatically
//! convert thrown exceptions into a [`Result`]. This allows more idiomatic error handling
//! via `Result`s.

use crate::sys::{sim_exception, SIM_clear_exception, SIM_get_pending_exception, SIM_last_error};
use std::ffi::CStr;

/// Alias for `sim_exception`
pub type SimException = sim_exception;

/// Returns the error message associated with the most recently raised frontend
/// exception, even if that exception has been cleared.
///
/// The returned string is only valid until the next use of the Simics API in the same
/// thread.
///
/// # Context
///
/// Cell Context
pub fn last_error() -> String {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}

/// Clears the currently pending frontend exception and returns the value of it.
///
/// # Return Value
///
/// Returns the exception that was pending before the call, or SimExc_No_Exception.
///
/// # Context
///
/// Cell Context
pub fn clear_exception() -> SimException {
    unsafe { SIM_clear_exception() }
}

/// This function returns the exception type of the current pending exception, or
/// SimExc_No_Exception if none available.
///
/// # Return Value
///
/// The pending exception. This value is [`SimException::SimExc_No_Exception`] if there was
/// none.
///
/// # Context
///
/// Cell Context
pub fn get_pending_exception() -> SimException {
    unsafe { SIM_get_pending_exception() }
}
