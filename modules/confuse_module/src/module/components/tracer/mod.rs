use std::{collections::HashMap, ffi::c_void, num::Wrapping};

use crate::{
    config::{InputConfig, OutputConfig, TraceMode},
    module::Confuse,
    processor::Processor,
    traits::{ConfuseInterface, ConfuseState},
};
use anyhow::Result;

use libafl::prelude::{AsMutSlice, AsSlice, OwnedMutSlice};
use raffl_macro::{callback_wrappers, params};
use rand::{thread_rng, Rng};
use tracing::info;

use simics_api::{
    attr_object_or_nil_from_ptr, get_processor_number, AttrValue, CachedInstructionHandle,
    ConfObject, InstructionHandle,
};

pub struct Tracer<'a> {
    coverage: OwnedMutSlice<'a, u8>,
    coverage_prev_loc: u64,
    processors: HashMap<i32, Processor>,
    mode: TraceMode,
}

impl<'a, 'b> From<*mut std::ffi::c_void> for &'a mut Tracer<'b> {
    /// Convert from a *mut Confuse pointer to a mutable reference to tracer
    fn from(value: *mut std::ffi::c_void) -> &'a mut Tracer<'b> {
        let confuse_ptr: *mut Confuse = value as *mut Confuse;
        let confuse = unsafe { &mut *confuse_ptr };
        &mut confuse.tracer
    }
}

impl<'a> Tracer<'a> {
    pub const COVERAGE_MAP_SIZE: usize = 0x10000;

    /// Try to instantiate a new AFL Coverage Tracer
    pub fn try_new() -> Result<Self> {
        Ok(Self {
            // Initialize with a dummy coverage map
            coverage: OwnedMutSlice::from(Vec::new()),
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
}

impl<'a> ConfuseState for Tracer<'a> {
    fn on_initialize(
        &mut self,
        _confuse: *mut ConfObject,
        input_config: &mut InputConfig,
        output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        self.mode = input_config.trace_mode.clone();
        // TODO: Maybe actually fix this lifetime stuff but it is actually unsafe to share this
        // coverage map so maybe there is no unsafe solution here
        self.coverage = unsafe {
            OwnedMutSlice::from_raw_parts_mut(
                input_config.coverage_map.0,
                input_config.coverage_map.1,
            )
        };
        self.coverage_prev_loc = thread_rng().gen_range(0..self.coverage.as_slice().len()) as u64;
        Ok(output_config)
    }

    fn pre_first_run(&mut self, confuse: *mut ConfObject) -> Result<()> {
        for (_processor_number, processor) in self.processors.iter_mut() {
            match self.mode {
                TraceMode::Once => {
                    processor.register_cached_instruction_cb(
                        tracer_callbacks::on_cached_instruction,
                        Some(confuse as *mut c_void),
                    )?;
                }
                TraceMode::HitCount => {
                    processor.register_instruction_before_cb(
                        tracer_callbacks::on_instruction_before,
                        Some(confuse as *mut c_void),
                    )?;
                }
            }
        }
        Ok(())
    }

    // Uncomment to check map hash
    // fn on_stopped(&mut self, confuse: *mut ConfObject, reason: StopReason) -> Result<()> {
    //     let buf = self.coverage_writer.read_all()?;

    //     info!("Hash of AFL Map: {:#x}", hash(&buf));

    //     Ok(())
    // }
}

impl<'a> ConfuseInterface for Tracer<'a> {
    fn on_add_processor(&mut self, processor_attr: *mut AttrValue) -> Result<()> {
        let processor_obj: *mut ConfObject = attr_object_or_nil_from_ptr(processor_attr)?;
        let processor_number = get_processor_number(processor_obj);
        let processor = Processor::try_new(processor_number, processor_obj)?
            .try_with_cpu_instrumentation_subscribe(processor_attr)?
            .try_with_processor_info_v2(processor_attr)?
            .try_with_cpu_instruction_query(processor_attr)?;

        self.processors.insert(processor_number, processor);

        info!("Tracer added processor #{}", processor_number);

        Ok(())
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl<'a> Tracer<'a> {
    #[params(..., !slf: *mut std::ffi::c_void)]
    pub fn on_instruction_before(
        &mut self,
        _obj: *mut ConfObject,
        cpu: *mut ConfObject,
        handle: *mut InstructionHandle,
    ) -> Result<()> {
        let processor_number = get_processor_number(cpu);

        if let Some(processor) = self.processors.get_mut(&processor_number) {
            if let Ok(Some(pc)) = processor.trace(handle) {
                // trace!("Traced execution was control flow: {:#x}", pc);
                self.log_pc(pc)?;
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
            if let Ok(Some(pc)) = processor.trace(handle) {
                // trace!("Traced execution was control flow: {:#x}", pc);
                self.log_pc(pc)?;
            }
        }

        Ok(())
    }
}
