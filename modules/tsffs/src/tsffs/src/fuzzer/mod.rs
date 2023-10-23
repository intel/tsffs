// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use getters::Getters;
use typed_builder::TypedBuilder;

use crate::Tsffs;

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Fuzzer<'a>
where
    'a: 'static,
{
    parent: &'a mut Tsffs,
}
