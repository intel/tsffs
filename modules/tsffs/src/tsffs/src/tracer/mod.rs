// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Error, Result};
use ffi_macro::ffi;
use getters2::Getters;
use libafl::prelude::CmpValues;
use libafl_bolts::{AsMutSlice, AsSlice};
use libafl_targets::{AFLppCmpLogOperands, AFLPP_CMPLOG_MAP_H, AFL_CMP_TYPE_INS};
use simics::{
    api::{
        get_processor_number, sys::instruction_handle_t, AsConfObject, AttrValue, AttrValueType,
        ConfObject,
    },
    trace,
};
use std::{collections::HashMap, ffi::c_void, fmt::Display, num::Wrapping, str::FromStr};
use typed_builder::TypedBuilder;

use crate::{arch::ArchitectureOperations, Tsffs};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CmpExpr {
    Deref((Box<CmpExpr>, Option<u8>)),
    Reg((String, u8)),
    Mul((Box<CmpExpr>, Box<CmpExpr>)),
    Add((Box<CmpExpr>, Box<CmpExpr>)),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Addr(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CmpType {
    Equal = 1,
    Greater = 2,
    Lesser = 4,
    Fp = 8,
    FpMod = 16,
    IntMod = 32,
    Transform = 64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CmpValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Expr(Box<CmpExpr>),
}

impl TryFrom<&CmpExpr> for CmpValue {
    type Error = Error;
    fn try_from(value: &CmpExpr) -> Result<Self> {
        Ok(match value {
            CmpExpr::U8(u) => CmpValue::U8(*u),
            CmpExpr::I8(i) => CmpValue::I8(*i),
            CmpExpr::U16(u) => CmpValue::U16(*u),
            CmpExpr::I16(i) => CmpValue::I16(*i),
            CmpExpr::U32(u) => CmpValue::U32(*u),
            CmpExpr::I32(i) => CmpValue::I32(*i),
            CmpExpr::U64(u) => CmpValue::U64(*u),
            CmpExpr::I64(i) => CmpValue::I64(*i),
            _ => bail!("Can't convert directly from non-integral expr"),
        })
    }
}

fn cmp_shape(cmp: &CmpValues) -> Result<u32> {
    match cmp {
        CmpValues::U8(_) => Ok(0),
        CmpValues::U16(_) => Ok(1),
        CmpValues::U32(_) => Ok(3),
        CmpValues::U64(_) => Ok(7),
        _ => bail!("Shape not implemented for non-integral types"),
    }
}

fn byte_width(value: u64) -> usize {
    if value < 0x10000 {
        if value < 0x100 {
            1
        } else {
            2
        }
    } else if value < 0x100000000 {
        4
    } else {
        8
    }
}

/// Hash a value into an index into an array lf length `len`
fn hash_index(value: u64, len: u64) -> u64 {
    let value_bytes = value.to_le_bytes();
    let hash_width = byte_width(len - 1);
    let hash_iters = value_bytes.len() / hash_width;
    let mut buffer = [0u8; 8];

    for i in 0..hash_iters {
        if i == 0 {
            buffer[0..hash_width]
                .clone_from_slice(&value_bytes[i * hash_width..(i + 1) * hash_width])
        } else {
            (0..hash_width).for_each(|j| {
                buffer[j] ^= value_bytes[i * hash_width..(i + 1) * hash_width][j];
            });
        }
    }

    u64::from_le_bytes(buffer)
}

#[derive(TypedBuilder, Debug, Clone, Getters, PartialEq, Eq)]
pub struct TraceEntry {
    #[builder(default, setter(into, strip_option))]
    #[getters(deref)]
    /// The target of an edge in the trace
    edge: Option<u64>,
    #[builder(default, setter(into, strip_option))]
    cmp: Option<(u64, Vec<CmpType>, CmpValues)>,
}

impl Default for TraceEntry {
    fn default() -> Self {
        Self::builder().build()
    }
}

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

impl Tsffs {
    fn log_pc(&mut self, pc: u64) -> Result<()> {
        let afl_idx =
            (pc ^ self.coverage_prev_loc) % self.coverage_map_ref().as_slice().len() as u64;
        let mut cur_byte: Wrapping<u8> =
            Wrapping(self.coverage_map_ref().as_slice()[afl_idx as usize]);
        cur_byte += 1;
        self.coverage_map_mut().as_mut_slice()[afl_idx as usize] = cur_byte.0;
        self.coverage_prev_loc = (pc >> 1) % self.coverage_map_ref().as_slice().len() as u64;

        Ok(())
    }

    fn log_cmp(&mut self, pc: u64, types: Vec<CmpType>, cmp: CmpValues) -> Result<()> {
        // Consistently hash pc to the same header index
        let shape = cmp_shape(&cmp)?;
        let operands = cmp
            .to_u64_tuple()
            .ok_or_else(|| anyhow!("Conversion to tuple of non-integral operands not supported"))?;
        let pc_index = hash_index(pc, self.aflpp_cmp_map_ref().headers().len() as u64);

        let hits = self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize].hits();

        if hits == 0 {
            trace!(
                self.as_conf_object(),
                "Logging first hit of comparison with types {types:?} and values {cmp:?} (assume == if no types)"
            );
        }

        self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize].set_hits(hits + 1);
        self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize].set_shape(shape);
        self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize].set__type(AFL_CMP_TYPE_INS);

        if let Some(attribute) = types.iter().map(|t| *t as u32).reduce(|acc, t| acc | t) {
            self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize].set_attribute(attribute);
            // NOTE: overflow isn't used by aflppredqueen
        } else {
            // Naively use EQ if we don't have a value
            self.aflpp_cmp_map_mut().headers_mut()[pc_index as usize]
                .set_attribute(CmpType::Equal as u32);
        }

        self.aflpp_cmp_map_mut().values_mut().operands_mut()[pc_index as usize]
            [hits as usize % AFLPP_CMPLOG_MAP_H] = AFLppCmpLogOperands::new(operands.0, operands.1);

        Ok(())
    }
}

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl Tsffs {
    #[ffi(arg(rest), arg(self))]
    /// Callback after each instruction executed
    ///
    /// # Arguments
    ///
    /// * `obj`
    /// * `cpu` - The processor the instruction is being executed by
    /// * `handle` - An opaque handle to the instruction being executed
    pub fn on_instruction_after(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu)?;

        if self.coverage_enabled_deref() {
            if let Some(arch) = self.processors_mut().get_mut(&processor_number) {
                match arch.trace_pc(handle) {
                    Ok(r) => {
                        if let Some(pc) = r.edge_deref() {
                            self.log_pc(pc)?;
                        }
                    }
                    Err(_) => {
                        // This is not really an error, but we may want to know  about it
                        // sometimes when debugging
                        // trace!(self.as_conf_object(), "Error tracing for PC: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    #[ffi(arg(rest), arg(self))]
    /// Callback after each instruction executed
    ///
    /// # Arguments
    ///
    /// * `obj`
    /// * `cpu` - The processor the instruction is being executed by
    /// * `handle` - An opaque handle to the instruction being executed
    pub fn on_instruction_before(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu)?;

        if self.configuration_ref().cmplog_deref() && self.cmplog_enabled_deref() {
            if let Some(arch) = self.processors_mut().get_mut(&processor_number) {
                match arch.trace_cmp(handle) {
                    Ok(r) => {
                        if let Some((pc, types, cmp)) = r.cmp_ref() {
                            self.log_cmp(*pc, types.clone(), cmp.clone())?;
                        }
                    }
                    Err(_) => {
                        // This is not really an error, but we may want to know  about it
                        // sometimes when debugging
                        // trace!(self.as_conf_object(), "Error tracing for CMP: {e}");
                    }
                }
            }
        }

        Ok(())
    }
}
