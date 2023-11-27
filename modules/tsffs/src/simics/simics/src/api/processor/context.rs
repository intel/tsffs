// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use simics_macro::simics_exception;

use crate::api::{sys::SIM_register_context_handler, ConfClass, ContextHandlerInterface};

#[simics_exception]
/// Register `cls` as a class for context handler objects
///
/// # Context
///
/// Unknown
pub fn register_context_handler(cls: *mut ConfClass, iface: *const ContextHandlerInterface) -> i32 {
    unsafe { SIM_register_context_handler(cls, iface as *const _) }
}
