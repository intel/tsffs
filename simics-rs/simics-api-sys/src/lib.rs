// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! # SIMICS API SYS
//!
//! Low level bindings to the SIMICS API
//!
//! This crate provides raw bindings to the SIMICS api built directly from the header files of the
//! SIMICS base package using `bindgen`. In general, you should prefer to use the `simics-api`
//! crate over this one, as it provides higher level safe bindings to the SIMICS API.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(rustdoc::broken_intra_doc_links, rustdoc::bare_urls)]

mod bindings;

pub use bindings::*;

include!(concat!(env!("OUT_DIR"), "/version-auto.rs"));
