// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS
//!
//! The SIMICS crate provides:
//!
//! * High level API bindings to the SIMICS API
//! * Re-exports the low level API bindings to the SIMICS API
//! * Utility and logging functionality relevant to the SIMICS API
//! * A global allocator using the SIMICS allocation functionality for consistent memory
//!   management when running code embedded in the SIMICS simulator

#![deny(clippy::unwrap_used)]

pub use api::alloc::SimicsAlloc;
pub use error::{Error, Result};

extern crate num_traits;

pub mod api;
pub mod error;
pub mod ispm;
pub mod log;

#[forbid(unsafe_code)]
pub mod util;

#[global_allocator]
/// All crates using the SIMICS API must also use the SIMICS allocator as their
/// global allocator, hence we set it here
static GLOBAL: SimicsAlloc = SimicsAlloc;
