// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Not officially exported CORE APIs

use simics_macro::simics_exception;

extern "C" {
    /// Discard recorded future events and forget them
    pub fn CORE_discard_future();
}

#[simics_exception]
/// Discard future events that are scheduled
///
/// This will clear recorded events and logs
///
/// # Context
///
/// Global Context
pub fn discard_future() {
    unsafe { CORE_discard_future() };
}
