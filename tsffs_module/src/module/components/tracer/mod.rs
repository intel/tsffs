// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(non_snake_case)]

use std::{collections::HashMap, ffi::c_void, fmt::Debug, num::Wrapping, ptr::null_mut};

use crate::{
    config::{InputConfig, OutputConfig, TraceMode},
    module::Module,
    processor::Processor,
    traits::{Interface, State},
};
use anyhow::{anyhow, bail, Result};

use c2rust_bitfields_derive::BitfieldStruct;
use ffi_macro::{callback_wrappers, params};
use libafl::prelude::{
    CmpMap, CmpValues, AFL_CMP_MAP_H, AFL_CMP_MAP_RTN_H, AFL_CMP_MAP_W, AFL_CMP_TYPE_INS,
};
use libafl_bolts::{bolts_prelude::OwnedMutSlice, prelude::OwnedRefMut, AsMutSlice, AsSlice};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::info;

use simics_api::{
    attr_object_or_nil_from_ptr, get_processor_number, AttrValue, CachedInstructionHandle,
    ConfObject, InstructionHandle,
};

/// The AFL++ `cmp_operands` struct
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AFLppCmpOperandsWritable {
    pub v0: u64,
    pub v1: u64,
    pub v0_128: u64,
    pub v1_128: u64,
}

impl AFLppCmpOperandsWritable {
    #[must_use]
    /// 64bit first cmp operand
    pub fn v0(&self) -> u64 {
        self.v0
    }

    #[must_use]
    /// 64bit second cmp operand
    pub fn v1(&self) -> u64 {
        self.v1
    }

    #[must_use]
    /// 128bit first cmp operand
    pub fn v0_128(&self) -> u64 {
        self.v0_128
    }

    #[must_use]
    /// 128bit second cmp operand
    pub fn v1_128(&self) -> u64 {
        self.v1_128
    }
}

/// The AFL++ `cmpfn_operands` struct
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AFLppCmpFnOperandsWritable {
    pub v0: [u8; 31],
    pub v0_len: u8,
    pub v1: [u8; 31],
    pub v1_len: u8,
}

impl AFLppCmpFnOperandsWritable {
    #[must_use]
    /// first rtn operand
    pub fn v0(&self) -> &[u8; 31] {
        &self.v0
    }

    #[must_use]
    /// second rtn operand
    pub fn v0_len(&self) -> u8 {
        self.v0_len
    }

    #[must_use]
    /// first rtn operand len
    pub fn v1(&self) -> &[u8; 31] {
        &self.v1
    }

    #[must_use]
    /// second rtn operand len
    pub fn v1_len(&self) -> u8 {
        self.v1_len
    }
}

#[derive(Debug, Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct AFLppCmpHeaderWritable {
    #[bitfield(name = "hits", ty = "u32", bits = "0..=23")]
    #[bitfield(name = "id", ty = "u32", bits = "24..=47")]
    #[bitfield(name = "shape", ty = "u32", bits = "48..=52")]
    #[bitfield(name = "_type", ty = "u32", bits = "53..=54")]
    #[bitfield(name = "attribute", ty = "u32", bits = "55..=58")]
    #[bitfield(name = "overflow", ty = "u32", bits = "59..=59")]
    #[bitfield(name = "reserved", ty = "u32", bits = "60..=63")]
    data: [u8; 8],
}

/// A proxy union to avoid casting operands as in AFL++
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub union AFLppCmpValsWritable {
    operands: [[AFLppCmpOperandsWritable; AFL_CMP_MAP_H]; AFL_CMP_MAP_W],
    fn_operands: [[AFLppCmpFnOperandsWritable; AFL_CMP_MAP_RTN_H]; AFL_CMP_MAP_W],
}

impl Debug for AFLppCmpValsWritable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AFLppCmpVals").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AFLppCmpMapWritable {
    headers: [AFLppCmpHeaderWritable; AFL_CMP_MAP_W],
    vals: AFLppCmpValsWritable,
}

impl Serialize for AFLppCmpMapWritable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let slice = unsafe {
            core::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                core::mem::size_of::<Self>(),
            )
        };
        serializer.serialize_bytes(slice)
    }
}

impl<'de> Deserialize<'de> for AFLppCmpMapWritable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        let map: Self = unsafe { core::ptr::read(bytes.as_ptr() as *const _) };
        Ok(map)
    }
}

impl CmpMap for AFLppCmpMapWritable {
    fn len(&self) -> usize {
        AFL_CMP_MAP_W
    }

    fn executions_for(&self, idx: usize) -> usize {
        self.headers[idx].hits() as usize
    }

    fn usable_executions_for(&self, idx: usize) -> usize {
        if self.headers[idx]._type() == AFL_CMP_TYPE_INS {
            if self.executions_for(idx) < AFL_CMP_MAP_H {
                self.executions_for(idx)
            } else {
                AFL_CMP_MAP_H
            }
        } else if self.executions_for(idx) < AFL_CMP_MAP_RTN_H {
            self.executions_for(idx)
        } else {
            AFL_CMP_MAP_RTN_H
        }
    }

    fn values_of(&self, idx: usize, execution: usize) -> Option<CmpValues> {
        if self.headers[idx]._type() == AFL_CMP_TYPE_INS {
            unsafe {
                match self.headers[idx].shape() {
                    0 => Some(CmpValues::U8((
                        self.vals.operands[idx][execution].v0 as u8,
                        self.vals.operands[idx][execution].v1 as u8,
                    ))),
                    1 => Some(CmpValues::U16((
                        self.vals.operands[idx][execution].v0 as u16,
                        self.vals.operands[idx][execution].v1 as u16,
                    ))),
                    3 => Some(CmpValues::U32((
                        self.vals.operands[idx][execution].v0 as u32,
                        self.vals.operands[idx][execution].v1 as u32,
                    ))),
                    7 => Some(CmpValues::U64((
                        self.vals.operands[idx][execution].v0,
                        self.vals.operands[idx][execution].v1,
                    ))),
                    // TODO handle 128 bits cmps
                    // other => panic!("Invalid CmpLog shape {}", other),
                    _ => None,
                }
            }
        } else {
            unsafe {
                let v0_len = self.vals.fn_operands[idx][execution].v0_len & (0x80 - 1);
                let v1_len = self.vals.fn_operands[idx][execution].v1_len & (0x80 - 1);
                Some(CmpValues::Bytes((
                    self.vals.fn_operands[idx][execution].v0[..(v0_len as usize)].to_vec(),
                    self.vals.fn_operands[idx][execution].v1[..(v1_len as usize)].to_vec(),
                )))
            }
        }
    }

    fn reset(&mut self) -> Result<(), libafl::Error> {
        // For performance, we reset just the headers
        self.headers = unsafe { core::mem::zeroed() };
        // self.vals.operands = unsafe { core::mem::zeroed() };
        Ok(())
    }
}

pub struct Tracer {
    coverage: OwnedMutSlice<'static, u8>,
    cmp: OwnedRefMut<'static, AFLppCmpMapWritable>,
    coverage_prev_loc: u64,
    processors: HashMap<i32, Processor>,
    mode: TraceMode,
}

impl From<*mut std::ffi::c_void> for &mut Tracer {
    /// Convert from a *mut Module pointer to a mutable reference to tracer
    fn from(value: *mut std::ffi::c_void) -> &'static mut Tracer {
        let module_ptr: *mut Module = value as *mut Module;
        let module = unsafe { &mut *module_ptr };
        &mut module.tracer
    }
}

impl Tracer {
    /// Try to instantiate a new AFL Coverage Tracer
    pub fn try_new() -> Result<Self> {
        Ok(Self {
            // Initialize with a dummy coverage map
            coverage: OwnedMutSlice::from(Vec::new()),
            cmp: OwnedRefMut::Owned(unsafe { Box::from_raw(null_mut()) }),
            coverage_prev_loc: 0,
            processors: HashMap::new(),
            mode: TraceMode::Once,
        })
    }

    fn log_pc(&mut self, pc: u64) -> Result<()> {
        let afl_idx = (pc ^ self.coverage_prev_loc) % self.coverage.as_slice().len() as u64;
        let mut cur_byte: Wrapping<u8> = Wrapping(self.coverage.as_slice()[afl_idx as usize]);
        cur_byte += 1;
        self.coverage.as_mut_slice()[afl_idx as usize] = cur_byte.0;
        self.coverage_prev_loc = (pc >> 1) % self.coverage.as_slice().len() as u64;

        Ok(())
    }

    fn log_cmp(&mut self, pc: u64, cmp: CmpValues) -> Result<()> {
        // Consistently hash pc to the same header index
        let shape = cmp_shape(&cmp)?;
        let operands = cmp
            .to_u64_tuple()
            .ok_or_else(|| anyhow!("Conversion to tuple of non-integral operands not supported"))?;
        let pc_index = hash_index(pc, self.cmp.as_ref().len() as u64);

        let hits = self.cmp.as_ref().headers[pc_index as usize].hits();
        self.cmp.as_mut().headers[pc_index as usize].set_hits(hits + 1);
        self.cmp.as_mut().headers[pc_index as usize].set_shape(shape);
        self.cmp.as_mut().headers[pc_index as usize].set__type(AFL_CMP_TYPE_INS);

        unsafe {
            self.cmp.as_mut().vals.operands[pc_index as usize][hits as usize % AFL_CMP_MAP_H] =
                AFLppCmpOperandsWritable {
                    v0: operands.0,
                    v1: operands.1,
                    v0_128: 0,
                    v1_128: 0,
                };
        }

        Ok(())
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

#[cfg(test)]
mod test_tracer_math {
    use crate::module::components::tracer::{byte_width, hash_index};

    #[test]
    fn test_cmp_hash() {
        const LEN: usize = 65536;
        // Sanity check that we'll get the right number of bytes needed to represent the map width
        assert_eq!(byte_width(LEN as u64), 2);
    }

    #[test]
    fn test_hash_into() {
        const LEN: usize = 65536;
        let x = hash_index(0x180001b0, LEN as u64);
        let y = hash_index(0x180001b1, LEN as u64);
        assert_ne!(x, y);
    }
}

impl State for Tracer {
    fn on_initialize(
        &mut self,
        _module: *mut ConfObject,
        input_config: &mut InputConfig,
        output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        self.mode = input_config.trace_mode;
        // TODO: Maybe actually fix this lifetime stuff but it is actually unsafe to share this
        // coverage map so maybe there is no unsafe solution here
        self.coverage = unsafe {
            OwnedMutSlice::from_raw_parts_mut(
                input_config.coverage_map.0,
                input_config.coverage_map.1,
            )
        };
        self.cmp = unsafe {
            OwnedRefMut::Owned(Box::from_raw(
                input_config.cmp_map as *mut AFLppCmpMapWritable,
            ))
        };
        self.coverage_prev_loc = thread_rng().gen_range(0..self.coverage.as_slice().len()) as u64;
        info!("Initialized Tracer");
        Ok(output_config)
    }

    fn pre_first_run(&mut self, module: *mut ConfObject) -> Result<()> {
        for (_processor_number, processor) in self.processors.iter_mut() {
            match self.mode {
                TraceMode::Once => {
                    processor.register_cached_instruction_cb(
                        tracer_callbacks::on_cached_instruction,
                        Some(module as *mut c_void),
                    )?;
                }
                TraceMode::HitCount => {
                    processor.register_instruction_before_cb(
                        tracer_callbacks::on_instruction_before,
                        Some(module as *mut c_void),
                    )?;
                }
            }
        }
        Ok(())
    }

    // Uncomment to check map hash
    // fn on_stopped(&mut self, module: *mut ConfObject, reason: StopReason) -> Result<()> {
    //     let buf = self.coverage_writer.read_all()?;

    //     info!("Hash of AFL Map: {:#x}", hash(&buf));

    //     Ok(())
    // }
}

impl Interface for Tracer {
    fn on_add_processor(&mut self, processor_attr: *mut AttrValue) -> Result<()> {
        let processor_obj: *mut ConfObject = attr_object_or_nil_from_ptr(processor_attr)?;
        let processor_number = get_processor_number(processor_obj);
        let processor = Processor::try_new(processor_number, processor_obj)?
            .try_with_cpu_instrumentation_subscribe(processor_attr)?
            .try_with_processor_info_v2(processor_attr)?
            .try_with_cpu_instruction_query(processor_attr)?
            .try_with_int_register(processor_attr)?;

        self.processors.insert(processor_number, processor);

        info!("Tracer added processor #{}", processor_number);

        Ok(())
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl Tracer {
    #[params(..., !slf: *mut std::ffi::c_void)]
    pub fn on_instruction_before(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut InstructionHandle,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu);

        if let Some(processor) = self.processors.get_mut(&processor_number) {
            if let Ok(r) = processor.trace(handle) {
                // trace!("Traced execution was control flow: {:#x}", pc);
                if let Some(pc) = r.edge {
                    self.log_pc(pc)?;
                }
                if let Some((pc, cmp)) = r.cmp {
                    self.log_cmp(pc, cmp)?;
                }
            }
        }

        Ok(())
    }

    #[params(..., !slf: *mut std::ffi::c_void)]
    pub fn on_cached_instruction(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        _cached_instruction_data: *mut CachedInstructionHandle,
        handle: *mut InstructionHandle,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu);

        if let Some(processor) = self.processors.get_mut(&processor_number) {
            if let Ok(r) = processor.trace(handle) {
                // trace!("Traced execution was control flow: {:#x}", pc);
                if let Some(pc) = r.edge {
                    self.log_pc(pc)?;
                }

                if let Some((pc, cmp)) = r.cmp {
                    self.log_cmp(pc, cmp)?;
                }
            }
        }

        Ok(())
    }
}
