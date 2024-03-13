// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! HAP APIs and HAP definitions

#![allow(clippy::unused_unit)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    simics_exception,
    sys::{
        hap_handle_t, hap_type_t, SIM_get_all_hap_types, SIM_hap_add_type, SIM_hap_get_name,
        SIM_hap_get_number, SIM_hap_is_active, SIM_hap_is_active_obj, SIM_hap_is_active_obj_idx,
        SIM_hap_occurred_always, SIM_hap_remove_type,
    },
    AttrValue, ConfObject, Result,
};
use raw_cstr::raw_cstr;
use std::{ffi::CStr, ptr::null_mut};

/// Alias for `hap_handle_t`
pub type HapHandle = hap_handle_t;
/// Alias for `hap_type_t`
pub type HapType = hap_type_t;

#[simics_exception]
/// Return an attribute list of all hap types.
pub fn get_all_hap_types() -> AttrValue {
    unsafe { SIM_get_all_hap_types() }.into()
}

#[simics_exception]
/// Add a new hap type
pub fn hap_add_type<S>(hap: S, params: S, param_desc: S, index: S, desc: S) -> Result<HapType>
where
    S: AsRef<str>,
{
    Ok(unsafe {
        SIM_hap_add_type(
            raw_cstr(hap)?,
            raw_cstr(params)?,
            raw_cstr(param_desc)?,
            raw_cstr(index)?,
            raw_cstr(desc)?,
            0,
        )
    })
}

#[simics_exception]
/// Get the name from a hap number
pub fn hap_get_name(hap: HapType) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_hap_get_name(hap)) }
        .to_str()?
        .to_string())
}

#[simics_exception]
/// Get the number from a hap name
pub fn hap_get_number<S>(hap: S) -> Result<HapType>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_hap_get_number(raw_cstr(hap)?) })
}

#[simics_exception]
/// Check if a hap is active (i.e. it has 1 or more registered handlers)
pub fn hap_is_active(hap: HapType) -> bool {
    unsafe { SIM_hap_is_active(hap) }
}

#[simics_exception]
/// Check if a hap is active for a given object
pub fn hap_is_active_obj(hap: HapType, obj: *mut ConfObject) -> bool {
    unsafe { SIM_hap_is_active_obj(hap, obj) }
}

#[simics_exception]
/// check if a hap is active for a given object and index
pub fn hap_is_active_obj_idx(hap: HapType, obj: *mut ConfObject, index: i64) -> bool {
    unsafe { SIM_hap_is_active_obj_idx(hap, obj, index) }
}

#[simics_exception]
/// Trigger a hap occurrence
pub fn hap_occurred_always(
    hap: HapType,
    obj: Option<*mut ConfObject>,
    value: i64,
    list: &mut AttrValue,
) -> i32 {
    unsafe { SIM_hap_occurred_always(hap, obj.unwrap_or(null_mut()), value, list.as_mut_ptr()) }
}

#[simics_exception]
/// Remove a custom hap type registered with [`hap_add_type`]
pub fn hap_remove_type<S>(hap: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_hap_remove_type(raw_cstr(hap)?) };
    Ok(())
}

// NOTE: recommended to only use the always version

include!(concat!(env!("OUT_DIR"), "/haps.rs"));
// Re-export all HAPs
pub use self::haps::*;
