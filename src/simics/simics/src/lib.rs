// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]

extern crate num_traits;

pub mod api;

#[forbid(unsafe_code)]
pub mod simics;

#[forbid(unsafe_code)]
pub mod util;
