// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]

pub use error::{Error, Result};

extern crate num_traits;

pub mod api;
pub mod error;
pub mod log;

#[forbid(unsafe_code)]
pub mod util;
