// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Host machine profiling APIs

use crate::{
    api::sys::{profile_area_t, SIM_add_profiling_area, SIM_remove_profiling_area},
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

pub type ProfileArea = profile_area_t;

#[simics_exception]
/// Add an address space area for profiling
pub fn add_profiling_area<S>(name: S, start: usize, end: usize) -> Result<*mut ProfileArea>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_add_profiling_area(raw_cstr(name)?, start, end) })
}

#[simics_exception]
/// Remove an area set for profiling
pub fn remove_profiling_area(handle: *mut ProfileArea) {
    unsafe { SIM_remove_profiling_area(handle) }
}
