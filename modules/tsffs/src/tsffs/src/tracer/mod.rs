// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Error, Result};
use ffi_macro::ffi;
use getters::Getters;
use simics::api::{
    sys::{cached_instruction_handle_t, instruction_handle_t},
    AttrValue, AttrValueType, ConfObject,
};
use simics_macro::TryIntoAttrValueTypeDict;
use std::{collections::HashMap, ffi::c_void, fmt::Display, str::FromStr};
use typed_builder::TypedBuilder;

use crate::{state::StopReason, traits::Component, Tsffs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum CoverageMode {
    HitCount,
    Once,
}

impl CoverageMode {
    const AS_STRING: &'static [(&'static str, Self)] =
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

#[derive(TypedBuilder, Getters, Clone, Debug, TryIntoAttrValueTypeDict)]
#[getters(mutable)]
pub struct TracerConfiguration {
    #[builder(default)]
    coverage_mode: CoverageMode,
    #[builder(default = true)]
    cmplog: bool,
}

impl Default for TracerConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Tracer<'a>
where
    'a: 'static,
{
    parent: &'a mut Tsffs,
    #[builder(default)]
    configuration: TracerConfiguration,
}

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl<'a> Tracer<'a>
where
    'a: 'static,
{
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

impl<'a> Component for Tracer<'a> {
    fn on_simulation_stopped(&mut self, _reason: &StopReason) -> Result<()> {
        Ok(())
    }
}
