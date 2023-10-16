// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS logging APIs

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    api::{
        sys::{
            SIM_log_critical, SIM_log_error, SIM_log_info, SIM_log_level, SIM_log_register_groups,
            SIM_log_spec_violation, SIM_log_unimplemented, SIM_set_log_level,
        },
        ConfObject,
    },
    Error, Result,
};
use simics_macro::simics_exception;
use std::{ffi::CString, ptr::null};

use super::ConfClass;

const LOG_GROUP: i32 = 0;

#[repr(i32)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

#[simics_exception]
/// Log an info-level message through the SIMICS logging functions
pub fn log_info<S>(level: LogLevel, device: *mut ConfObject, msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    let msg_cstring = CString::new(msg.as_ref())?;

    unsafe {
        SIM_log_info(level as i32, device, LOG_GROUP, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
pub fn log_error(device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_error(device, LOG_GROUP, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
pub fn log_critical(device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_critical(device, LOG_GROUP, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
pub fn log_spec_violation(level: LogLevel, device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_spec_violation(level as i32, device, LOG_GROUP, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Log an error-level message through the SIMICS logging functions
pub fn log_unimplemented(level: LogLevel, device: *mut ConfObject, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_unimplemented(level as i32, device, LOG_GROUP, msg_cstring.as_ptr());
    };

    Ok(())
}

#[simics_exception]
/// Get the current log level of an object
pub fn log_level(obj: *mut ConfObject) -> u32 {
    unsafe { SIM_log_level(obj as *const ConfObject) }
}

#[simics_exception]
/// Set the SIMICS log level for an object
pub fn set_log_level(obj: *mut ConfObject, level: LogLevel) {
    unsafe { SIM_set_log_level(obj, level as u32) };
}

#[simics_exception]
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
