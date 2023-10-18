// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use simics_macro::simics_exception;

use crate::api::{
    sys::{SIM_flush_all_caches, SIM_flush_cell_caches},
    ConfObject,
};

#[simics_exception]
/// Flush all global and local caches
pub fn flush_all_caches() {
    unsafe { SIM_flush_all_caches() }
}

#[simics_exception]
/// Flush caches for an object's cell
pub fn flush_cell_caches(obj: *mut ConfObject) {
    unsafe { SIM_flush_cell_caches(obj) }
}
