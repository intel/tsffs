// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]

pub use api::alloc::SimicsAlloc;
pub use error::{Error, Result};

extern crate num_traits;

pub mod api;
pub mod error;
pub mod log;

#[forbid(unsafe_code)]
pub mod util;

#[global_allocator]
/// All crates using the SIMICS API must also use the SIMICS allocator as their
/// global allocator, hence we set it here
static GLOBAL: SimicsAlloc = SimicsAlloc;
