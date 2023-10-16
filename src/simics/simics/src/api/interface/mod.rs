// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]

//! High level bindings for API-provided SIMICS interfaces

use simics_macro::simics_interface_codegen;

pub use self::interfaces::*;

#[simics_interface_codegen(source = "bindings.rs")]
/// Automatically generated bindings for interfaces provided by the SIMICS API. See the
/// simics-macro crate for details
pub mod interfaces {}
