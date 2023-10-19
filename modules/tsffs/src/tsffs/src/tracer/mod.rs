// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use ffi_macro::ffi;
use getters::Getters;
use simics::{
    api::{
        sys::{cached_instruction_handle_t, instruction_handle_t},
        AttrValue, AttrValueType, ConfObject,
    },
    Error, Result,
};
use simics_macro::TryIntoAttrValueType;
use std::{collections::HashMap, ffi::c_void, fmt::Display, str::FromStr};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
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

    fn from_str(s: &str) -> Result<Self> {
        let as_string = Self::AS_STRING.iter().cloned().collect::<HashMap<_, _>>();

        Ok(as_string.get(s).cloned().ok_or_else(|| {
            anyhow!(
                "Invalid coverage mode {}. Expected one of {}",
                s,
                Self::AS_STRING
                    .iter()
                    .map(|i| i.0)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?)
    }
}

impl Display for CoverageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let to_string = Self::AS_STRING
            .iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>();
        if let Some(name) = to_string.get(self) {
            write!(f, "{}", name)
        } else {
            panic!("Invalid state for enum");
        }
    }
}

impl TryFrom<AttrValue> for CoverageMode {
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        String::try_from(value)?.parse()
    }
}

impl From<CoverageMode> for AttrValueType {
    fn from(value: CoverageMode) -> Self {
        value.to_string().into()
    }
}

#[derive(TypedBuilder, Getters, Clone, Debug, TryIntoAttrValueType)]
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

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl Tracer {
    #[ffi(arg(rest), arg(self))]
    pub fn on_instruction_before(
        &mut self,
        _obj: *mut ConfObject,
        _cpu: *mut ConfObject,
        _handle: *mut instruction_handle_t,
    ) -> Result<()> {
        Ok(())
    }

    #[ffi(arg(self), arg(rest))]
    pub fn on_cached_instruction_before(
        &mut self,
        _obj: *mut ConfObject,
        _cpu: *mut ConfObject,
        _cached_instruction_data: *mut cached_instruction_handle_t,
        _handle: *mut instruction_handle_t,
    ) -> Result<()> {
        Ok(())
    }
}
