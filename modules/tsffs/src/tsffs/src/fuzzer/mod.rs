// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use getters::Getters;
use typed_builder::TypedBuilder;

use crate::{state::StopReason, traits::Component, Tsffs};

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Fuzzer<'a>
where
    'a: 'static,
{
    parent: &'a mut Tsffs,
}

impl<'a> Component for Fuzzer<'a> {
    fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
        Ok(())
    }
}
