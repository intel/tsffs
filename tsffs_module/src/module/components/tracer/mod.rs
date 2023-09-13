// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(non_snake_case)]

use std::{collections::HashMap, ffi::c_void, num::Wrapping, ptr::null_mut};

use crate::{
    config::{InputConfig, OutputConfig, TraceMode},
    module::Module,
    processor::{CmpType, Processor},
    traits::{Interface, State},
};
use anyhow::{anyhow, bail, Result};

use ffi_macro::{callback_wrappers, params};
use libafl::prelude::{
    AFLppCmpMap, AFLppCmpOperands, CmpMap, CmpValues, AFL_CMP_MAP_H, AFL_CMP_TYPE_INS,
};
use libafl_bolts::{bolts_prelude::OwnedMutSlice, prelude::OwnedRefMut, AsMutSlice, AsSlice};
use rand::{thread_rng, Rng};
use tracing::{info, trace};

use simics_api::{
    attr_object_or_nil_from_ptr, get_processor_number, AttrValue, CachedInstructionHandle,
    ConfObject, InstructionHandle,
};

pub struct Tracer {
    coverage: OwnedMutSlice<'static, u8>,
    cmp: OwnedRefMut<'static, AFLppCmpMap>,
    coverage_prev_loc: u64,
    processors: HashMap<i32, Processor>,
    mode: TraceMode,
    cmplog: bool,
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
            cmplog: false,
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

    fn log_cmp(&mut self, pc: u64, types: Vec<CmpType>, cmp: CmpValues) -> Result<()> {
        // Consistently hash pc to the same header index
        let shape = cmp_shape(&cmp)?;
        let operands = cmp
            .to_u64_tuple()
            .ok_or_else(|| anyhow!("Conversion to tuple of non-integral operands not supported"))?;
        let pc_index = hash_index(pc, self.cmp.as_ref().len() as u64);

        let hits = self.cmp.as_mut().headers_mut()[pc_index as usize].hits();
        self.cmp.as_mut().headers_mut()[pc_index as usize].set_hits(hits + 1);
        self.cmp.as_mut().headers_mut()[pc_index as usize].set_shape(shape);
        self.cmp.as_mut().headers_mut()[pc_index as usize].set__type(AFL_CMP_TYPE_INS);
        let attribute = types
            .iter()
            .map(|t| *t as u32)
            .reduce(|acc, t| acc | t)
            .ok_or_else(|| anyhow!("Could not reduce types"))?;
        self.cmp.as_mut().headers_mut()[pc_index as usize].set_attribute(attribute);
        // NOTE: overflow isn't used by aflppredqueen

        trace!("Logging cmp with types {:?} and values {:?}", types, cmp);

        unsafe {
            self.cmp.as_mut().values_mut().operands_mut()[pc_index as usize]
                [hits as usize % AFL_CMP_MAP_H] = AFLppCmpOperands::new(operands.0, operands.1);
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
        // Sanity check that we'll get the right number of bytes needed to represent the map width
        assert_eq!(byte_width(65535), 2);
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
        self.cmp = unsafe { OwnedRefMut::Owned(Box::from_raw(input_config.cmp_map)) };
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

    fn on_run(
        &mut self,
        _module: *mut ConfObject,
        run_config: &crate::config::RunConfig,
    ) -> Result<()> {
        self.cmplog = run_config.cmplog;

        trace!("Running with cmplog mode: {}", self.cmplog);

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
            }
        }

        if self.cmplog {
            if let Some(processor) = self.processors.get_mut(&processor_number) {
                if let Ok(r) = processor.trace_cmp(handle) {
                    if let Some((pc, types, cmp)) = r.cmp {
                        self.log_cmp(pc, types, cmp)?;
                    }
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
            }
        }

        if self.cmplog {
            if let Some(processor) = self.processors.get_mut(&processor_number) {
                if let Ok(r) = processor.trace_cmp(handle) {
                    if let Some((pc, types, cmp)) = r.cmp {
                        self.log_cmp(pc, types, cmp)?;
                    }
                }
            }
        }

        Ok(())
    }
}
