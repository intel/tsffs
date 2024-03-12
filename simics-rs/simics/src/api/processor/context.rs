// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Context handling

use crate::{
    simics_exception, sys::SIM_register_context_handler, ConfClass, ContextHandlerInterface,
};

#[simics_exception]
/// Register `cls` as a class for context handler objects
///
/// # Context
///
/// Unknown
pub fn register_context_handler(cls: *mut ConfClass, iface: *const ContextHandlerInterface) -> i32 {
    unsafe { SIM_register_context_handler(cls, iface as *const _) }
}
