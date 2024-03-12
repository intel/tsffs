// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Interfaces defined by the base package

#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::too_many_arguments,
    clippy::unit_arg,
    clippy::should_implement_trait
)]

pub use self::interfaces::*;

include!(concat!(env!("OUT_DIR"), "/interfaces.rs"));
