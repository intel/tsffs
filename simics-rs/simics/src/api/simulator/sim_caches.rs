// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simulator cache control

use crate::{
    simics_exception,
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
