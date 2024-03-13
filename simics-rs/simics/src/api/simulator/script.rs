// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Script running and management

use crate::{
    simics_exception,
    sys::{
        SIM_get_batch_mode, SIM_load_target, SIM_run_command, SIM_run_command_file,
        SIM_run_command_file_params,
    },
    AttrValue, Error, Result,
};
use raw_cstr::raw_cstr;
use std::any::type_name;

#[simics_exception]
/// Run a SIMICS CLI command
pub fn run_command<S>(line: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_run_command(raw_cstr(line)?) }.into())
}

#[simics_exception]
/// Run a SIMICS CLI command file
pub fn run_command_file<S>(file: S, local: bool) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_run_command_file(raw_cstr(file)?, local) };
    Ok(())
}

#[simics_exception]
/// Run a SIMICS cli command file with a list of parameters. Parameters are key-value pairs.
pub fn run_command_file_params<S, I, T>(file: S, local: bool, params: I) -> Result<()>
where
    S: AsRef<str>,
    I: IntoIterator<Item = (S, T)>,
    T: TryInto<AttrValue>,
{
    let params: Vec<AttrValue> = params
        .into_iter()
        .map(|a| {
            a.1.try_into()
                .map_err(|_| Error::ToAttrValueConversionError {
                    ty: type_name::<T>().to_string(),
                })
                .and_then(|v| {
                    [a.0.as_ref().into(), v]
                        .into_iter()
                        .collect::<Vec<_>>()
                        .try_into()
                        .map_err(|_| Error::ToAttrValueConversionError {
                            ty: type_name::<T>().to_string(),
                        })
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let params: AttrValue = params.try_into()?;
    let params = params.into();
    unsafe { SIM_run_command_file_params(raw_cstr(file)?, local, params) };
    Ok(())
}

#[simics_exception]
/// Load a target
pub fn load_target<S>(target: S, ns: S, presets: AttrValue, cmdline_args: AttrValue) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe {
        SIM_load_target(
            raw_cstr(target)?,
            raw_cstr(ns)?,
            presets.into(),
            cmdline_args.into(),
        )
    };
    Ok(())
}

#[simics_exception]
/// Check whether running in batch mode
pub fn get_batch_mode() -> bool {
    unsafe { SIM_get_batch_mode() }
}
