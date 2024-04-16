// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Logging APIs

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::sys::{
    VT_log_critical, VT_log_error, VT_log_info, VT_log_spec_violation, VT_log_unimplemented,
};
use crate::{
    simics_exception,
    sys::{SIM_log_level, SIM_log_register_groups, SIM_set_log_level},
    ConfClass, ConfObject, Error, Result,
};
use std::{ffi::CString, ptr::null};

/// The default log group
pub const LOG_GROUP: i32 = 0;

#[repr(i32)]
/// A log level as defined by SIMICS
pub enum LogLevel {
    /// Error log level
    Error = 0,
    /// Warning log level
    Warn = 1,
    /// Informational log level
    Info = 2,
    /// Debug log level
    Debug = 3,
    /// Trace log level
    Trace = 4,
}

/// Sanitize a string for logging (i.e. as if with printf)
fn sanitize<S>(s: S) -> String
where
    S: AsRef<str>,
{
    s.as_ref().replace('%', "%%")
}

#[simics_exception]
/// Log an info-level message through the SIMICS logging functions
///
/// # Arguments
///
/// * `level` - The level to emit this log message at
/// * `device` - The device to emit this log message through
/// * `msg` - The message to log
///
/// # Notes
///
/// The macros [`simics::error`], [`simics::warn`], [`simics::info`], [`simics::debug`],
/// and [`simics::trace`] are more flexible and user friendly. They should be used instead.
///
/// # Context
///
/// All Contexts
pub fn log_info<S>(level: LogLevel, device: *mut ConfObject, msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    let msg_cstring = CString::new(msg.as_ref())?;

    unsafe {
        VT_log_info(level as i32, device, LOG_GROUP as u64, msg_cstring.as_ptr());
    }

    Ok(())
}

#[simics_exception]
/// Log an info-level message through the SIMICS logging functions
///
/// # Arguments
///
/// * `device` - The device to emit this log message through
/// * `msg` - The message to log
///
/// # Notes
///
/// The macros [`simics::error`], [`simics::warn`], [`simics::info`], [`simics::debug`],
/// and [`simics::trace`] are more flexible and user friendly. They should be used instead.
///
/// # Context
///
/// All Contexts
pub fn log_error<S>(device: *mut ConfObject, msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    let msg_cstring = CString::new(sanitize(msg.as_ref()))?;

    unsafe {
        VT_log_error(device, LOG_GROUP as u64, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an info-level message through the SIMICS logging functions
///
/// # Arguments
///
/// * `device` - The device to emit this log message through
/// * `msg` - The message to log
///
/// # Notes
///
/// This function causes a frontend exception. Only use it if the error is truly critical.
///
/// # Context
///
/// All Contexts
pub fn log_critical<S>(device: *mut ConfObject, msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    let msg_cstring = CString::new(sanitize(msg.as_ref()))?;

    unsafe {
        VT_log_critical(device, LOG_GROUP as u64, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
///
/// # Context
///
/// All Contexts
pub fn log_spec_violation(level: LogLevel, device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(sanitize(msg))?;

    unsafe {
        VT_log_spec_violation(level as i32, device, LOG_GROUP as u64, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
///
/// # Context
///
/// All Contexts
pub fn log_unimplemented(level: LogLevel, device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(sanitize(msg))?;

    unsafe {
        VT_log_unimplemented(level as i32, device, LOG_GROUP as u64, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Get the current log level of an object
///
/// # Arguments
///
/// * `obj` - The object to get the log level for
///
/// # Context
///
/// Cell Context
pub fn log_level(obj: *mut ConfObject) -> u32 {
    unsafe { SIM_log_level(obj as *const ConfObject) }
}

#[simics_exception]
/// Set the SIMICS log level for an object
///
/// # Arguments
///
/// * `obj` - The object to set the log level for
/// * `level` - The level to set the log level to
///
/// # Context
///
/// Cell Context
pub fn set_log_level(obj: *mut ConfObject, level: LogLevel) {
    unsafe { SIM_set_log_level(obj, level as u32) };
}

#[simics_exception]
/// Register one or more groups for the class
///
/// # Context
///
/// Global Context
pub fn log_register_groups<S>(cls: *mut ConfClass, names: &[S]) -> Result<()>
where
    S: AsRef<str>,
{
    let name_cstrs = names
        .iter()
        .map(|n| CString::new(n.as_ref()).map_err(Error::from))
        .collect::<Result<Vec<CString>>>()?;
    let mut name_ptrs = name_cstrs.iter().map(|n| n.as_ptr()).collect::<Vec<_>>();
    name_ptrs.push(null());
    unsafe { SIM_log_register_groups(cls, name_ptrs.as_ptr()) };

    Ok(())
}
