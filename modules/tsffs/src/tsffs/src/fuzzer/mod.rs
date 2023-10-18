// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use getters::Getters;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Fuzzer {}

impl Default for Fuzzer {
    fn default() -> Self {
        Self::builder().build()
    }
}
