// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use ffi_macro::ffi;
use getters::Getters;
use simics::{
    api::{
        sys::{cached_instruction_handle_t, instruction_handle_t},
        AsAttrValue, AsAttrValueType, AttrValue, AttrValueType, ConfObject, FromAttrValue,
    },
    Error, Result,
};
use simics_macro::AsAttrValueType;
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

impl AsAttrValue for CoverageMode {
    fn as_attr_value(&self) -> Result<AttrValue> {
        let to_string = Self::AS_STRING
            .iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>();
        Ok(to_string
            .get(self)
            .ok_or_else(|| Error::from(anyhow!("No matching coverage mode {self:?}")))
            .and_then(|s| s.to_string().as_attr_value().map_err(Error::from))?)
    }
}

impl FromAttrValue for CoverageMode {
    fn from_attr_value(value: AttrValue) -> simics::Result<Self>
    where
        Self: Sized,
    {
        let s: String = String::from_attr_value(value)?;
        Self::from_str(&s)
    }
}

impl AsAttrValueType for CoverageMode {
    fn as_attr_value_type(self) -> AttrValueType {
        AttrValueType::String(self.to_string())
    }
}

#[derive(TypedBuilder, Getters, Clone, Debug, AsAttrValueType)]
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
        obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        Ok(())
    }

    #[ffi(arg(self), arg(rest))]
    pub fn on_cached_instruction_before(
        &mut self,
        obj: *mut ConfObject,
        cpu: *mut ConfObject,
        cached_instruction_data: *mut cached_instruction_handle_t,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        Ok(())
    }
}
