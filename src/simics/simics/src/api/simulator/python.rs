// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::api::sys::{SIM_run_python, SIM_source_python, VT_call_python_module_function};
use crate::api::AttrValue;
use crate::error::Result;
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::path::Path;

#[simics_exception]
pub fn call_python_module_function<S>(
    module: S,
    function: S,
    args: *mut AttrValue,
) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe {
        VT_call_python_module_function(
            raw_cstr(module.as_ref())?,
            raw_cstr(function.as_ref())?,
            args,
        )
    })
}

#[simics_exception]
pub fn source_python<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe { SIM_source_python(raw_cstr(file.as_ref().to_string_lossy())?) };
    Ok(())
}

#[simics_exception]
pub fn run_python<S>(line: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_run_python(raw_cstr(line)?) };
    Ok(())
}
