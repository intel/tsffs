// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{
        sys::{
            SIM_get_batch_mode, SIM_load_target, SIM_run_command, SIM_run_command_file,
            SIM_run_command_file_params,
        },
        AttrValue,
    },
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

#[simics_exception]
pub fn run_command<S>(line: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_run_command(raw_cstr(line)?) })
}

#[simics_exception]
pub fn run_command_file<S>(file: S, local: bool) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_run_command_file(raw_cstr(file)?, local) };
    Ok(())
}

#[simics_exception]
pub fn run_command_file_params<S>(file: S, local: bool, params: AttrValue) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_run_command_file_params(raw_cstr(file)?, local, params) };
    Ok(())
}

#[simics_exception]
pub fn load_target<S>(target: S, ns: S, presets: AttrValue, cmdline_args: AttrValue) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_load_target(raw_cstr(target)?, raw_cstr(ns)?, presets, cmdline_args) };
    Ok(())
}

#[simics_exception]
pub fn get_batch_mode() -> bool {
    unsafe { SIM_get_batch_mode() }
}
