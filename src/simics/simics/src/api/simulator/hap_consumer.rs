// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unused_unit)]

use crate::api::{last_error, AttrValue, ConfObject, GenericTransaction};
use anyhow::{bail, ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{hap_handle_t, SIM_get_all_hap_types, SIM_hap_add_callback};
use simics_macro::{simics_exception, simics_hap_codegen};
use std::{
    ffi::{c_char, c_void},
    mem::transmute,
    ptr::null_mut,
};

pub use self::haps::*;

pub type HapHandle = hap_handle_t;

#[simics_exception]
pub fn get_all_hap_types() -> AttrValue {
    unsafe { SIM_get_all_hap_types() }
}

#[simics_hap_codegen(source = "bindings.rs")]
pub mod haps {}
