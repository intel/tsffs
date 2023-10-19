// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::api::sys::{
    attr_value_t, SIM_call_python_function, SIM_run_python, SIM_source_python,
    VT_call_python_module_function,
};
use crate::error::Result;
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::path::Path;

#[simics_exception]
/// Source a python file of SIMICS python code
pub fn source_python<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe { SIM_source_python(raw_cstr(file.as_ref().to_string_lossy())?) };
    Ok(())
}

#[simics_exception]
/// Run (by eval-ing) python code in the simulator context
pub fn run_python<S>(line: S) -> Result<attr_value_t>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_run_python(raw_cstr(line)?) })
}

#[simics_exception]
/// Call a python function with a set of arguments
pub fn call_python_function<S>(function: S, args: *mut attr_value_t) -> Result<attr_value_t>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_call_python_function(raw_cstr(function)?, args) })
}

#[simics_exception]
/// Call a function in a python module
pub fn call_python_module_function<S>(
    module: S,
    function: S,
    args: *mut attr_value_t,
) -> Result<attr_value_t>
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
