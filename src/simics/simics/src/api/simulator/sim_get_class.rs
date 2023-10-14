// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{sys::SIM_get_class, ConfClass},
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

#[simics_exception]
pub fn get_class<S>(name: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_class(raw_cstr(name)?) })
}
