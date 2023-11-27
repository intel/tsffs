// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::too_many_arguments,
    clippy::unit_arg,
    clippy::should_implement_trait
)]

//! High level bindings for API-provided SIMICS interfaces

pub use self::interfaces::*;

include!(concat!(env!("OUT_DIR"), "/interfaces.rs"));
