// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Error, Result};
use ffi_macro::ffi;
use getters::Getters;
use simics::api::{
    sys::{cached_instruction_handle_t, instruction_handle_t},
    ConfObject,
};
use std::{collections::HashMap, str::FromStr};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageMode {
    HitCount,
    Once,
}

impl CoverageMode {
    const AS_STRING: &[(&'static str, Self)] =
        &[("hit-count", Self::HitCount), ("once", Self::Once)];
}

impl Default for CoverageMode {
    fn default() -> Self {
        Self::HitCount
    }
}

impl FromStr for CoverageMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let as_string = Self::AS_STRING.iter().cloned().collect::<HashMap<_, _>>();

        as_string.get(s).cloned().ok_or_else(|| {
            anyhow!(
                "Invalid coverage mode {}. Expected one of {}",
                s,
                Self::AS_STRING
                    .iter()
                    .map(|i| i.0)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct TracingConfiguration {
    #[builder(default)]
    coverage_mode: CoverageMode,
    #[builder(default = true)]
    cmplog: bool,
}

impl Default for TracingConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Tracer {
    configuration: TracingConfiguration,
}

impl Default for Tracer {
    fn default() -> Self {
        Self::builder()
            .configuration(TracingConfiguration::default())
            .build()
    }
}

// #[ffi(expect, use_all)]
// impl Tracer {
//     #[ffi(arg(self), arg(rest))]
//     pub fn on_instruction_before(
//         &mut self,
//         obj: *mut ConfObject,
//         cpu: *mut ConfObject,
//         handle: *mut instruction_handle_t,
//     ) -> Result<()> {
//         Ok(())
//     }
//
//     #[ffi(arg(self), arg(rest))]
//     pub fn on_cached_instruction_before(
//         &mut self,
//         obj: *mut ConfObject,
//         cpu: *mut ConfObject,
//         cached_instruction_data: *mut cached_instruction_handle_t,
//         handle: *mut instruction_handle_t,
//     ) -> Result<()> {
//         Ok(())
//     }
// }
