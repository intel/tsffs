// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use crate::api::sys::{SIM_run_python, SIM_source_python, VT_call_python_module_function};
use crate::api::{clear_exception, last_error, AttrValue, SimException};
use anyhow::{anyhow, Result};
use raw_cstr::raw_cstr;

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
            args.into(),
        )
    })
}

pub fn source_python<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe { SIM_source_python(raw_cstr(file.as_ref().to_string_lossy())?) };

    match clear_exception() {
        SimException::SimExc_No_Exception => Ok(()),
        _ => Err(anyhow!("Error running python script: {}", last_error())),
    }
}

pub fn run_python<S>(line: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_run_python(raw_cstr(line)?) };

    match clear_exception() {
        SimException::SimExc_No_Exception => Ok(()),
        _ => Err(anyhow!("Error running python script: {}", last_error())),
    }
}
