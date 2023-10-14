// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::sys::{
        SIM_copyright, SIM_register_copyright, SIM_version, SIM_version_base, SIM_version_major,
        SIM_vmxmon_version,
    },
    Result,
};
use raw_cstr::raw_cstr;
use std::ffi::CStr;

/// Get the current SIMICS version
pub fn version() -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_version()) }
        .to_str()?
        .to_string())
}

/// Get the current SIMICS version base
pub fn version_base() -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_version_base()) }
        .to_str()?
        .to_string())
}

/// Get the current SIMICS major version
pub fn version_major() -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_version_major()) }
        .to_str()?
        .to_string())
}

/// Get the current SIMICS vmxmon version
pub fn vmxmon_version() -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_vmxmon_version()) }
        .to_str()?
        .to_string())
}

/// Get the current copyright string
pub fn copyright() -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_copyright()) }
        .to_str()?
        .to_string())
}

/// Set the current copyright string
pub fn register_copyright<S>(str: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_register_copyright(raw_cstr(str)?) };
    Ok(())
}
