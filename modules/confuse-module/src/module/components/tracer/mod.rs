//! Tracer object

use crate::module::{
    component::{Component, ComponentInterface},
    config::{InitializeConfig, InitializedConfig},
    controller::TRACER,
    cpu::Cpu,
    map_type::MapType,
    stop_reason::StopReason,
};
use anyhow::{ensure, Result};
use confuse_simics_api::{attr_value_t, conf_object_t, instruction_handle_t};
use crc32fast::hash;
use ipc_shm::{IpcShm, IpcShmWriter};
use log::{debug, info, trace};
use rand::{thread_rng, Rng};
use std::{cell::RefCell, num::Wrapping, sync::MutexGuard};

pub struct AFLCoverageTracer {
    afl_coverage_map: IpcShm,
    afl_coverage_map_writer: IpcShmWriter,
    afl_prev_loc: u64,
    cpus: Vec<RefCell<Cpu>>,
}

impl AFLCoverageTracer {
    /// Retrieve the global controller object
    pub fn get<'a>() -> Result<MutexGuard<'a, Self>> {
        let tracer = TRACER.lock().expect("Could not lock controller");
        Ok(tracer)
    }
}

impl AFLCoverageTracer {
    // 256kb
    pub const AFL_COVERAGE_MAP_SIZE: usize = 0x40000;

    pub fn try_new() -> Result<Self> {
        let mut afl_coverage_map =
            IpcShm::try_new("afl_coverage_map", AFLCoverageTracer::AFL_COVERAGE_MAP_SIZE)?;
        let afl_coverage_map_writer = afl_coverage_map.writer()?;
        let afl_prev_loc = thread_rng().gen_range(0..afl_coverage_map.len()) as u64;

        Ok(Self {
            afl_coverage_map,
            afl_coverage_map_writer,
            afl_prev_loc,
            cpus: vec![],
        })
    }

    fn log_pc(&mut self, pc: u64) -> Result<()> {
        let afl_idx = (pc ^ self.afl_prev_loc) % self.afl_coverage_map_writer.len() as u64;
        let mut cur_byte: Wrapping<u8> =
            Wrapping(self.afl_coverage_map_writer.read_byte(afl_idx as usize)?);
        cur_byte += 1;
        self.afl_coverage_map_writer
            .write_byte(cur_byte.0, afl_idx as usize)?;
        self.afl_prev_loc = (pc >> 1) % self.afl_coverage_map_writer.len() as u64;
        Ok(())
    }

    pub unsafe fn on_instruction(
        &mut self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<()> {
        let mut pcs = Vec::new();
        for processor in &self.cpus {
            if let Ok(Some(pc)) =
                unsafe { processor.borrow_mut().is_branch(cpu, instruction_query) }
            {
                pcs.push(pc);
            }
        }

        for pc in pcs {
            self.log_pc(pc)?;
        }

        Ok(())
    }
}

impl Component for AFLCoverageTracer {
    fn on_initialize(
        &mut self,
        _initialize_config: &InitializeConfig,
        initialized_config: InitializedConfig,
    ) -> Result<InitializedConfig> {
        Ok(initialized_config.with_map(MapType::Coverage(self.afl_coverage_map.try_clone()?)))
    }

    fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn on_reset(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self, _reason: Option<StopReason>) -> Result<()> {
        let map = self.afl_coverage_map_writer.read_all()?;
        let map_csum = hash(&map);
        trace!("Map checksum: {:#x}", map_csum);
        Ok(())
    }

    fn pre_first_run(&mut self) -> Result<()> {
        Ok(())
    }
}

impl ComponentInterface for AFLCoverageTracer {
    unsafe fn on_add_processor(
        &mut self,
        _obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        info!("Adding processor to context");
        let cpu = RefCell::new(Cpu::try_new(processor)?);

        cpu.borrow()
            .register_cached_instruction_cb(callbacks::cached_instruction_cb)?;

        ensure!(
            self.cpus.is_empty(),
            "A CPU has already been added! This module only supports 1 vCPU at this time."
        );

        self.cpus.push(cpu);

        Ok(())
    }

    unsafe fn on_add_fault(&mut self, obj: *mut conf_object_t, fault: i64) -> Result<()> {
        Ok(())
    }
}

mod callbacks {
    use std::ffi::c_void;

    use confuse_simics_api::{cached_instruction_handle_t, conf_object_t, instruction_handle_t};

    use super::AFLCoverageTracer;

    #[no_mangle]
    pub extern "C" fn cached_instruction_cb(
        _obj: *mut conf_object_t,
        cpu: *mut conf_object_t,
        _cached_instruction: *mut cached_instruction_handle_t,
        instruction_query: *mut instruction_handle_t,
        _user_data: *mut c_void,
    ) {
        let mut tracer = AFLCoverageTracer::get().expect("Could not get tracer");
        unsafe {
            tracer
                .on_instruction(cpu, instruction_query)
                .expect("Failed to handle cached instruction callback")
        };
    }
}
