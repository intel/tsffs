// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]

extern crate num_traits;
#[macro_use]
extern crate num_derive;

pub mod api;
pub mod link;
pub mod manifest;
pub mod module;
pub mod package;
pub mod project;
pub mod simics;
pub mod traits;
pub mod util;
