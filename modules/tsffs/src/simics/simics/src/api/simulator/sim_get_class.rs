// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{sys::SIM_get_class, ConfClass},
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

#[simics_exception]
/// Get a class by name
///
/// # Performance
///
/// * `SIM_get_class` - Performs a hashtable lookup of `name`. Loads the module containing
///   the class named `name` if it is not loaded. This can be expensive once, but is cheap
///   every time thereafter.
pub fn get_class<S>(name: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_class(raw_cstr(name)?) })
}
