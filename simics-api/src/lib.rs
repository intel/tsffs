// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS API
//!
//! High Level safe bindings to the SIMICS API
//!
//! This crate provides higher-level bindings to the SIMICS API for writing safe code for the
//! SIMICS platform. This crate should be used instead of `simics-api-sys` for most purposes.
#![allow(clippy::useless_conversion)]
#![deny(clippy::unwrap_used)]

#[cfg(not(any(
    feature = "6.0.163",
    feature = "6.0.164",
    feature = "6.0.165",
    feature = "6.0.166",
    feature = "6.0.167",
    feature = "6.0.168",
    feature = "6.0.169",
    feature = "6.0.170"
)))]
compile_error!("Must enable a feature to specify a SIMICS API version");

#[cfg(any(
    feature = "6.0.163",
    feature = "6.0.164",
    feature = "6.0.165",
    feature = "6.0.166",
    feature = "6.0.167",
    feature = "6.0.168",
))]
pub mod safe;

#[cfg(any(
    feature = "6.0.163",
    feature = "6.0.164",
    feature = "6.0.165",
    feature = "6.0.166",
    feature = "6.0.167",
    feature = "6.0.168",
))]
pub use safe::*;

pub use simics_api_sys as sys;
