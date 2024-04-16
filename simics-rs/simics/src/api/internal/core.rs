// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Not officially exported CORE APIs

#[cfg(simics_version_6)]
use crate::simics_exception;

#[cfg(simics_version_6)]
extern "C" {
    /// Discard recorded future events and forget them
    pub fn CORE_discard_future();
}

#[cfg(simics_version_6)]
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
