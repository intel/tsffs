// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! # SIMICS API SYS
//!
//! Low level bindings to the SIMICS API
//!
//! This crate provides raw bindings to the SIMICS api built directly from the header files of the
//! SIMICS base package using `bindgen`. In general, you should prefer to use the `simics-api`
//! crate over this one, as it provides higher level safe bindings to the SIMICS API.
#![deny(clippy::unwrap_used)]

mod bindings;

pub use bindings::*;

#[cfg(feature = "auto")]
include!(concat!(env!("OUT_DIR"), "/version-auto.rs"));

#[cfg(feature = "6.0.163")]
pub const SIMICS_VERSION: &str = "6.0.163";
#[cfg(feature = "6.0.164")]
pub const SIMICS_VERSION: &str = "6.0.164";
#[cfg(feature = "6.0.165")]
pub const SIMICS_VERSION: &str = "6.0.165";
#[cfg(feature = "6.0.166")]
pub const SIMICS_VERSION: &str = "6.0.166";
#[cfg(feature = "6.0.167")]
pub const SIMICS_VERSION: &str = "6.0.167";
#[cfg(feature = "6.0.168")]
pub const SIMICS_VERSION: &str = "6.0.168";
#[cfg(feature = "6.0.169")]
pub const SIMICS_VERSION: &str = "6.0.169";
#[cfg(feature = "6.0.170")]
pub const SIMICS_VERSION: &str = "6.0.170";
#[cfg(feature = "6.0.171")]
pub const SIMICS_VERSION: &str = "6.0.171";
#[cfg(feature = "6.0.172")]
pub const SIMICS_VERSION: &str = "6.0.172";
#[cfg(feature = "6.0.173")]
pub const SIMICS_VERSION: &str = "6.0.173";
#[cfg(feature = "6.0.174")]
pub const SIMICS_VERSION: &str = "6.0.174";
