// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Error, Result};
use cpp_demangle::{DemangleOptions, Symbol};
use ffi2::ffi;
use lcov2::Records;
use libafl::prelude::CmpValues;
use libafl_bolts::{AsMutSlice, AsSlice};
use libafl_targets::{AFLppCmpLogOperands, AFL_CMP_TYPE_INS, CMPLOG_MAP_H};
use rustc_demangle::try_demangle;
use serde::{Deserialize, Serialize};
use simics::{
    api::{
        get_processor_number,
        sys::{cached_instruction_handle_t, instruction_handle_t},
        AsConfObject, AttrValue, AttrValueType, ConfObject,
    },
    error, get_interface, trace, ProcessorInfoV2Interface,
};
use std::{
    collections::HashMap, ffi::c_void, fmt::Display, hash::Hash, num::Wrapping,
    slice::from_raw_parts, str::FromStr,
};
use typed_builder::TypedBuilder;

use crate::{arch::ArchitectureOperations, os::windows::debug_info::SymbolInfo, Tsffs};

pub(crate) struct DecodedComparison {
    pub expressions: Vec<CmpExpr>,
    pub types: Vec<CmpType>,
}

/// Classification of an instruction when it enters the Simics instruction cache.
pub(crate) struct DecodedInstruction {
    pub branch: bool,
    pub comparison: Option<DecodedComparison>,
}

#[cfg(test)]
mod cached_feedback_tests {
    use super::*;
    use std::ptr::null_mut;

    #[test]
    fn repeated_branches_preserve_hit_counts_chaining_and_reporting() -> Result<()> {
        let mut tsffs = Tsffs {
            coverage_reporting: true,
            ..Default::default()
        };
        tsffs
            .coverage_map
            .set(vec![0u8; 16].into())
            .map_err(|_| anyhow!("Coverage map already set"))?;
        for _ in 0..257 {
            tsffs.record_branch(4)?;
        }
        let map = tsffs
            .coverage_map
            .get()
            .ok_or_else(|| anyhow!("Missing coverage map"))?
            .as_slice();
        assert_eq!(map[4], 1);
        assert_eq!(map[6], 0);
        assert_eq!(tsffs.coverage_prev_loc, 2);
        assert_eq!(tsffs.edges_seen_since_last.get(&4), Some(&4));

        tsffs.coverage_prev_loc = 0;
        tsffs.record_branch(4)?;
        assert_eq!(
            tsffs
                .coverage_map
                .get()
                .ok_or_else(|| anyhow!("Missing coverage map"))?
                .as_slice()[4],
            2
        );
        Ok(())
    }

    #[test]
    fn inactive_comparison_callbacks_do_not_read_cpu_state() -> Result<()> {
        let comparison = DecodedComparison {
            expressions: vec![],
            types: vec![],
        };
        for (coverage_enabled, cmplog, cmplog_enabled) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let mut tsffs = Tsffs {
                coverage_enabled,
                cmplog,
                cmplog_enabled,
                ..Default::default()
            };
            tsffs.trace_cached_comparison(null_mut(), &comparison)?;
        }
        Ok(())
    }
}

struct CachedComparison {
    tsffs: *mut Tsffs,
    decoded: DecodedComparison,
}

unsafe extern "C" fn free_cached_comparison(
    _obj: *mut ConfObject,
    _cpu: *mut ConfObject,
    data: *mut c_void,
) {
    // Simics releases this allocation when it discards the associated cache entry.
    drop(Box::from_raw(data.cast::<CachedComparison>()));
}

unsafe extern "C" fn on_cached_comparison(
    _obj: *mut ConfObject,
    cpu: *mut ConfObject,
    _handle: *mut instruction_handle_t,
    data: *mut c_void,
) {
    let data = &*data.cast::<CachedComparison>();
    let tsffs = &mut *data.tsffs;
    // Never panic across the FFI boundary: losing one comparison only drops cmplog feedback.
    if let Err(error) = tsffs.trace_cached_comparison(cpu, &data.decoded) {
        tsffs.report_cached_feedback_error("trace a cached comparison", &error);
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExecutionTraceSymbol {
    /// The symbol name
    pub symbol: String,
    /// The demangled symbol name, if it demangles correctly
    pub symbol_demangled: Option<String>,
    /// The offset into the symbol
    pub offset: u64,
    /// The containing module's name (usually the path to the executable or dll)
    pub module: String,
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub(crate) struct ExecutionTrace(pub HashMap<i32, Vec<ExecutionTraceEntry>>);

impl Hash for ExecutionTrace {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (k, v) in self.0.iter() {
            for entry in v.iter() {
                k.hash(state);
                entry.hash(state);
            }
        }
    }
}

#[derive(
    Clone, TypedBuilder, Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub(crate) struct ExecutionTraceEntry {
    pc: u64,
    #[builder(default, setter(into, strip_option))]
    insn: Option<String>,
    #[builder(default, setter(into, strip_option))]
    insn_bytes: Option<Vec<u8>>,
    #[builder(default, setter(into))]
    symbol: Option<ExecutionTraceSymbol>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CmpExprShift {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CmpExpr {
    Deref((Box<CmpExpr>, Option<u8>)),
    Reg((String, u8)),
    Mul((Box<CmpExpr>, Box<CmpExpr>)),
    Add((Box<CmpExpr>, Box<CmpExpr>)),
    Sub((Box<CmpExpr>, Box<CmpExpr>)),
    Shift((Box<CmpExpr>, Box<CmpExpr>, CmpExprShift)),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(crate) enum CmpType {
    Equal = 1,
    Greater = 2,
    Lesser = 4,
    Fp = 8,
    FpMod = 16,
    IntMod = 32,
    Transform = 64,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CmpValue {
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

impl CmpValue {
    /// The value extended to 64 bits (sign-extended when signed) and its width in bytes
    fn extended(&self) -> Option<(u64, usize)> {
        Some(match self {
            CmpValue::U8(v) => (*v as u64, 1),
            CmpValue::I8(v) => (*v as u64, 1),
            CmpValue::U16(v) => (*v as u64, 2),
            CmpValue::I16(v) => (*v as u64, 2),
            CmpValue::U32(v) => (*v as u64, 4),
            CmpValue::I32(v) => (*v as u64, 4),
            CmpValue::U64(v) => (*v, 8),
            CmpValue::I64(v) => (*v as u64, 8),
            CmpValue::Expr(_) => return None,
        })
    }
}

/// Pair the operands of a compare at the width of the left operand. The right operand is
/// extended then truncated to that width, as the CPU does with an immediate: `cmp rax, 5`
/// encodes a sign-extended 8-bit immediate.
pub(crate) fn pair_cmp_values(left: &CmpValue, right: &CmpValue) -> Option<CmpValues> {
    let (left, width) = left.extended()?;
    let (right, _) = right.extended()?;
    Some(match width {
        1 => CmpValues::U8((left as u8, right as u8)),
        2 => CmpValues::U16((left as u16, right as u16)),
        4 => CmpValues::U32((left as u32, right as u32)),
        _ => CmpValues::U64((left, right)),
    })
}

#[cfg(test)]
mod pair_cmp_values_tests {
    use super::*;

    #[test]
    fn pairs_an_immediate_at_the_width_of_the_left_operand() {
        // cmp rax, 5 and cmp rax, -1: sign-extended 8-bit immediate
        assert_eq!(
            pair_cmp_values(&CmpValue::U64(7), &CmpValue::I8(5)),
            Some(CmpValues::U64((7, 5)))
        );
        assert_eq!(
            pair_cmp_values(&CmpValue::U64(7), &CmpValue::I8(-1)),
            Some(CmpValues::U64((7, u64::MAX)))
        );
        // cmp eax, 0x41424344 and cmp eax, -2
        assert_eq!(
            pair_cmp_values(&CmpValue::U32(7), &CmpValue::I32(0x41424344)),
            Some(CmpValues::U32((7, 0x41424344)))
        );
        assert_eq!(
            pair_cmp_values(&CmpValue::U32(7), &CmpValue::I8(-2)),
            Some(CmpValues::U32((7, 0xffff_fffe)))
        );
        // cmp ax, 0x4142 and cmp al, 0x80
        assert_eq!(
            pair_cmp_values(&CmpValue::U16(7), &CmpValue::I16(0x4142)),
            Some(CmpValues::U16((7, 0x4142)))
        );
        assert_eq!(
            pair_cmp_values(&CmpValue::U8(7), &CmpValue::I8(-128)),
            Some(CmpValues::U8((7, 0x80)))
        );
    }

    #[test]
    fn keeps_same_width_pairs_and_rejects_expressions() {
        assert_eq!(
            pair_cmp_values(&CmpValue::U64(1), &CmpValue::U64(u64::MAX)),
            Some(CmpValues::U64((1, u64::MAX)))
        );
        assert_eq!(
            pair_cmp_values(&CmpValue::I32(-1), &CmpValue::I32(2)),
            Some(CmpValues::U32((u32::MAX, 2)))
        );
        assert_eq!(
            pair_cmp_values(&CmpValue::Expr(Box::new(CmpExpr::U8(1))), &CmpValue::U8(1)),
            None
        );
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

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceEntry {
    #[builder(default, setter(into, strip_option))]
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
pub(crate) enum CoverageMode {
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
    fn record_branch(&mut self, pc: u64) -> Result<()> {
        if self.coverage_reporting && self.edges_seen.insert(pc) {
            let coverage_map = self.coverage_map.get_mut().ok_or_else(|| {
                anyhow!("Coverage map not initialized. This is a bug in the fuzzer or the target")
            })?;
            let afl_idx = (pc ^ self.coverage_prev_loc) % coverage_map.as_slice().len() as u64;
            self.edges_seen_since_last.insert(pc, afl_idx);
        }
        self.log_pc(pc)
    }

    fn attach_cached_callbacks(
        &mut self,
        cpu: *mut ConfObject,
        cached_handle: *mut cached_instruction_handle_t,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        let tsffs = self as *mut Self;
        let number = get_processor_number(cpu)?;
        if let Some(arch) = self.processors.get_mut(&number) {
            let Ok(decoded) = arch.decode_cached_instruction(handle) else {
                return Ok(());
            };
            let interface = arch
                .cached_instruction()
                .ok_or_else(|| anyhow!("Cached instruction interface not initialized"))?;
            if decoded.branch {
                interface.register_instruction_after_cb(
                    cached_handle,
                    Some(tsffs::on_cached_branch_after),
                    tsffs.cast(),
                    None,
                )?;
            }
            if let Some(decoded) = decoded.comparison {
                let data = Box::into_raw(Box::new(CachedComparison { tsffs, decoded }));
                if let Err(error) = interface.register_instruction_before_cb(
                    cached_handle,
                    Some(on_cached_comparison),
                    data.cast(),
                    Some(free_cached_comparison),
                ) {
                    unsafe { drop(Box::from_raw(data)) };
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    /// Report the first selective-instrumentation failure; later ones are silent to avoid
    /// flooding the log from a hot simulator callback.
    fn report_cached_feedback_error(&mut self, action: &str, error: &anyhow::Error) {
        if !self.cached_feedback_error_reported {
            self.cached_feedback_error_reported = true;
            error!(
                self.as_conf_object(),
                "Failed to {action}; coverage and cmplog feedback may be missing (further failures are not reported): {error:#}"
            );
        }
    }

    fn trace_cached_comparison(
        &mut self,
        cpu: *mut ConfObject,
        decoded: &DecodedComparison,
    ) -> Result<()> {
        if self.coverage_enabled && self.cmplog && self.cmplog_enabled {
            let number = get_processor_number(cpu)?;
            if let Some(arch) = self.processors.get_mut(&number) {
                if let Ok(entry) = arch.trace_decoded_cmp(decoded) {
                    if let Some((pc, types, cmp)) = entry.cmp {
                        self.log_cmp(pc, types, cmp)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn log_pc(&mut self, pc: u64) -> Result<()> {
        let coverage_map = self.coverage_map.get_mut().ok_or_else(|| {
            anyhow!("Coverage map not initialized. This is a bug in the fuzzer or the target")
        })?;
        let afl_idx = (pc ^ self.coverage_prev_loc) % coverage_map.as_slice().len() as u64;
        let mut cur_byte: Wrapping<u8> = Wrapping(coverage_map.as_slice()[afl_idx as usize]);
        cur_byte += 1;
        coverage_map.as_mut_slice()[afl_idx as usize] = cur_byte.0;
        self.coverage_prev_loc = (pc >> 1) % coverage_map.as_slice().len() as u64;

        Ok(())
    }

    fn log_cmp(&mut self, pc: u64, types: Vec<CmpType>, cmp: CmpValues) -> Result<()> {
        // Consistently hash pc to the same header index
        let aflpp_cmp_map = self.aflpp_cmp_map.get_mut().ok_or_else(|| {
            anyhow!("AFL++ cmp map not initialized. This is a bug in the fuzzer or the target")
        })?;
        let shape = cmp_shape(&cmp)?;
        let operands = cmp
            .to_u64_tuple()
            .ok_or_else(|| anyhow!("Conversion to tuple of non-integral operands not supported"))?;
        let pc_index = hash_index(pc, aflpp_cmp_map.headers().len() as u64);

        let hits = aflpp_cmp_map.headers_mut()[pc_index as usize].hits();

        aflpp_cmp_map.headers_mut()[pc_index as usize].set_hits(hits + 1);
        aflpp_cmp_map.headers_mut()[pc_index as usize].set_shape(shape);
        aflpp_cmp_map.headers_mut()[pc_index as usize].set__type(AFL_CMP_TYPE_INS);

        if let Some(attribute) = types.iter().map(|t| *t as u32).reduce(|acc, t| acc | t) {
            aflpp_cmp_map.headers_mut()[pc_index as usize].set_attribute(attribute);
            // NOTE: overflow isn't used by aflppredqueen
        } else {
            // Naively use EQ if we don't have a value
            aflpp_cmp_map.headers_mut()[pc_index as usize].set_attribute(CmpType::Equal as u32);
        }

        aflpp_cmp_map.values_mut().operands_mut()[pc_index as usize]
            [hits as usize % CMPLOG_MAP_H] = AFLppCmpLogOperands::new(operands.0, operands.1);

        if hits == 0 {
            trace!(
                self.as_conf_object(),
                "Logged first hit of comparison with types {types:?} and values {cmp:?} (assume == if no types)"
            );
        }

        Ok(())
    }
}

/// Record a hit of `pc` in the line and function coverage of the symbol containing it. Each
/// line is recorded against its own source file, because a function can contain lines inlined
/// from other files.
fn record_symbol_hit(coverage: &mut Records, symbol: &SymbolInfo, pc: u64) {
    let Some(function_start_line) = symbol.lines.first() else {
        return;
    };
    let offset = pc - symbol.base;
    let mut hit_function_start = false;

    symbol
        .lines
        .iter()
        .filter(|l| offset >= l.rva && offset < l.rva + l.size as u64)
        .for_each(|l| {
            let record = coverage.get_or_insert_mut(&l.file_path);
            (l.start_line..=l.end_line).for_each(|line| record.increment_line(line as usize));
            hit_function_start |= l.file_path == function_start_line.file_path
                && (l.start_line..=l.end_line).contains(&function_start_line.start_line);
        });

    if hit_function_start {
        // Increment function hit counter if we just hit the fn
        coverage
            .get_or_insert_mut(&function_start_line.file_path)
            .increment_function_data(&symbol.name);
    }
}

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl Tsffs {
    #[ffi(arg(rest), arg(self))]
    /// Classify a newly cached x86 instruction and attach only the required execution callbacks.
    pub fn on_cached_instruction(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        cached_handle: *mut cached_instruction_handle_t,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        // A failure only loses feedback for one cache entry; report it once instead of
        // panicking inside the simulator's cache-fill path.
        if let Err(error) = self.attach_cached_callbacks(cpu, cached_handle, handle) {
            self.report_cached_feedback_error("instrument a cached instruction", &error);
        }
        Ok(())
    }

    #[ffi(arg(rest), arg(self))]
    /// Record the destination after an instrumented branch executes, without decoding it.
    pub fn on_cached_branch_after(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        _handle: *mut instruction_handle_t,
    ) -> Result<()> {
        if self.coverage_enabled {
            let number = get_processor_number(cpu)?;
            if let Some(arch) = self.processors.get_mut(&number) {
                if let Ok(pc) = arch.processor_info_v2().get_program_counter() {
                    self.record_branch(pc)?;
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
    pub fn on_instruction_after(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut instruction_handle_t,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu)?;

        if self.coverage_enabled {
            if let Some(arch) = self.processors.get_mut(&processor_number) {
                match arch.trace_pc(handle) {
                    Ok(r) => {
                        if let Some(pc) = r.edge {
                            self.record_branch(pc)?;
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

        if self.coverage_enabled && self.cmplog && self.cmplog_enabled {
            if let Some(arch) = self.processors.get_mut(&processor_number) {
                match arch.trace_cmp(handle) {
                    Ok(r) => {
                        if let Some((pc, types, cmp)) = r.cmp {
                            self.log_cmp(pc, types.clone(), cmp.clone())?;
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

        let symcov = if self.coverage_enabled
            && self.windows
            && self.symbolic_coverage
            && self.saves_execution_traces()
        {
            // Get the current instruction address
            let mut processor_information_v2 = get_interface::<ProcessorInfoV2Interface>(cpu)?;
            let pc = processor_information_v2.get_program_counter()?;
            self.windows_os_info
                .symbol_lookup_trees
                .get(&processor_number)
                .and_then(|lookup_tree| {
                    lookup_tree
                        .query(pc..pc + 1)
                        .next()
                        .map(|symbol_for_query| {
                            let offset =
                                pc - symbol_for_query.value.base + symbol_for_query.value.rva;
                            let symbol_demangled = try_demangle(&symbol_for_query.value.name)
                                .map(|d| d.to_string())
                                .ok()
                                .or_else(|| {
                                    Symbol::new(&symbol_for_query.value.name)
                                        .ok()
                                        .and_then(|s| s.demangle(&DemangleOptions::new()).ok())
                                });

                            record_symbol_hit(&mut self.coverage, &symbol_for_query.value, pc);
                            ExecutionTraceSymbol {
                                symbol: symbol_for_query.value.name.clone(),
                                symbol_demangled,
                                offset,
                                module: symbol_for_query.value.module.clone(),
                            }
                        })
                })
        } else if self.coverage_enabled && self.symbolic_coverage {
            let mut processor_information_v2 = get_interface::<ProcessorInfoV2Interface>(cpu)?;
            let pc = processor_information_v2.get_program_counter()?;
            if let Some(lookup_tree) = self
                .windows_os_info
                .symbol_lookup_trees
                .get(&processor_number)
            {
                if let Some(symbol_for_query) = lookup_tree.query(pc..pc + 1).next() {
                    record_symbol_hit(&mut self.coverage, &symbol_for_query.value, pc);
                }
            }
            None
        } else {
            None
        };

        if self.coverage_enabled && self.saves_execution_traces() {
            if let Some(arch) = self.processors.get_mut(&processor_number) {
                self.execution_trace
                    .0
                    .entry(processor_number)
                    .or_default()
                    .push(if self.execution_trace_pc_only {
                        ExecutionTraceEntry::builder()
                            .pc(arch.processor_info_v2().get_program_counter()?)
                            .build()
                    } else {
                        let instruction_bytes =
                            arch.cpu_instruction_query().get_instruction_bytes(handle)?;
                        let instruction_bytes = unsafe {
                            from_raw_parts(instruction_bytes.data, instruction_bytes.size)
                        };

                        if let Ok(disassembly_string) =
                            arch.disassembler().disassemble_to_string(instruction_bytes)
                        {
                            ExecutionTraceEntry::builder()
                                .pc(arch.processor_info_v2().get_program_counter()?)
                                .insn(disassembly_string)
                                .insn_bytes(instruction_bytes.to_vec())
                                .symbol(symcov)
                                .build()
                        } else {
                            ExecutionTraceEntry::builder()
                                .pc(arch.processor_info_v2().get_program_counter()?)
                                .insn("(unknown)".to_string())
                                .insn_bytes(instruction_bytes.to_vec())
                                .symbol(symcov)
                                .build()
                        }
                    });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod symbol_coverage_tests {
    use super::*;
    use crate::os::windows::debug_info::LineInfo;
    use std::path::{Path, PathBuf};

    fn line(rva: u64, file_path: &str, line: u32) -> LineInfo {
        LineInfo {
            rva,
            size: 0x10,
            file_path: PathBuf::from(file_path),
            start_line: line,
            end_line: line,
        }
    }

    fn hit_count(coverage: &Records, file_path: &str, line: usize) -> Option<usize> {
        coverage
            .get(Path::new(file_path))
            .and_then(|record| record.lines.get(&line))
            .map(|entry| entry.hit_count)
    }

    #[test]
    fn inlined_lines_are_recorded_against_their_own_file() {
        const BASE: u64 = 0x40_0000;
        let symbol = SymbolInfo::new(
            0x1000,
            BASE,
            0x20,
            "function".to_string(),
            "module".to_string(),
            vec![line(0x1000, "main.c", 10), line(0x1010, "inline.h", 99)],
        );
        let mut coverage = Records::default();

        // Hit the code inlined from the header
        record_symbol_hit(&mut coverage, &symbol, BASE + 0x1010);

        assert_eq!(hit_count(&coverage, "inline.h", 99), Some(1));
        assert_eq!(hit_count(&coverage, "main.c", 99), None);
        assert!(coverage
            .get(Path::new("main.c"))
            .is_none_or(|record| record.function_data.is_empty()));

        // Hit the first line of the function
        record_symbol_hit(&mut coverage, &symbol, BASE + 0x1000);

        assert_eq!(hit_count(&coverage, "main.c", 10), Some(1));
        assert_eq!(hit_count(&coverage, "inline.h", 10), None);
        assert!(coverage
            .get(Path::new("main.c"))
            .is_some_and(|record| record.function_data.contains_key("function")));
    }
}
