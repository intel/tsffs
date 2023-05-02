use std::{collections::HashMap, ffi::c_void, num::Wrapping};

use crate::{
    config::{InputConfig, OutputConfig, TraceMode},
    maps::MapType,
    module::Confuse,
    processor::Processor,
    traits::{ConfuseInterface, ConfuseState},
};
use anyhow::Result;

use ipc_shm::{IpcShm, IpcShmWriter};
use log::{info};
use raffl_macro::{callback_wrappers, params};
use rand::{thread_rng, Rng};

use simics_api::{
    attr_object_or_nil_from_ptr, get_processor_number, AttrValue,
    CachedInstructionHandle, ConfObject, InstructionHandle,
};

pub struct Tracer {
    coverage: IpcShm,
    coverage_writer: IpcShmWriter,
    coverage_prev_loc: u64,
    processors: HashMap<i32, Processor>,
    mode: TraceMode,
}
impl<'a> From<*mut std::ffi::c_void> for &'a mut Tracer {
    /// Convert from a *mut Confuse pointer to a mutable reference to tracer
    fn from(value: *mut std::ffi::c_void) -> &'a mut Tracer {
        let confuse_ptr: *mut Confuse = value as *mut Confuse;
        let confuse = unsafe { &mut *confuse_ptr };
        &mut confuse.tracer
    }
}

impl Tracer {
    pub const COVERAGE_MAP_SIZE: usize = 0x10000;

    /// Try to instantiate a new AFL Coverage Tracer
    pub fn try_new() -> Result<Self> {
        let mut coverage = IpcShm::try_new("afl_coverage_map", Tracer::COVERAGE_MAP_SIZE)?;
        let coverage_writer = coverage.writer()?;
        let coverage_prev_loc = thread_rng().gen_range(0..coverage.len()) as u64;

        Ok(Self {
            coverage,
            coverage_writer,
            coverage_prev_loc,
            processors: HashMap::new(),
            mode: TraceMode::Once,
        })
    }

    fn log_pc(&mut self, pc: u64) -> Result<()> {
        let afl_idx = (pc ^ self.coverage_prev_loc) % self.coverage.len() as u64;
        let mut cur_byte: Wrapping<u8> =
            Wrapping(self.coverage_writer.read_byte(afl_idx as usize)?);
        cur_byte += 1;
        self.coverage_writer
            .write_byte(cur_byte.0, afl_idx as usize)?;
        self.coverage_prev_loc = (pc >> 1) % self.coverage_writer.len() as u64;

        Ok(())
    }
}

impl ConfuseState for Tracer {
    fn on_initialize(
        &mut self,
        _confuse: *mut ConfObject,
        input_config: &InputConfig,
        output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        self.mode = input_config.trace_mode.clone();
        Ok(output_config.with_map(MapType::Coverage(self.coverage.try_clone()?)))
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

impl ConfuseInterface for Tracer {
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
