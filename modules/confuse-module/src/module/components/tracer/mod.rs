//! Tracer object

use crate::module::{
    component::Component,
    config::{InitializeConfig, InitializedConfig},
    controller::TRACER,
    map_type::MapType,
    stop_reason::StopReason,
};
use anyhow::{ensure, Result};
use confuse_simics_api::{
    attr_value_t, conf_object, conf_object_t, cpu_cached_instruction_interface_t,
    cpu_instruction_query_interface_t, cpu_instrumentation_subscribe_interface_t,
    instruction_handle_t, int_register_interface_t, processor_info_v2_interface_t,
    SIM_attr_object_or_nil, SIM_c_get_interface, CPU_CACHED_INSTRUCTION_INTERFACE,
    CPU_INSTRUCTION_QUERY_INTERFACE, CPU_INSTRUMENTATION_SUBSCRIBE_INTERFACE,
    INT_REGISTER_INTERFACE, PROCESSOR_INFO_V2_INTERFACE,
};
use crc32fast::hash;
use ipc_shm::{IpcShm, IpcShmWriter};
use log::{debug, info};
use rand::{thread_rng, Rng};
use std::{
    cell::RefCell,
    num::Wrapping,
    os::raw::c_char,
    ptr::null_mut,
    sync::{Arc, MutexGuard},
};

use self::cpu::Cpu;

mod cpu;

pub struct AFLCoverageTracer {
    afl_coverage_map: IpcShm,
    afl_coverage_map_writer: IpcShmWriter,
    afl_prev_loc: u64,
    cpus: Vec<Box<RefCell<Cpu>>>,
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

    pub fn on_instruction(
        &mut self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<()> {
        let mut pcs = Vec::new();
        for processor in &self.cpus {
            if let Ok(Some(pc)) = processor.borrow_mut().is_branch(cpu, instruction_query) {
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
        let map = self.afl_coverage_map_writer.read_all()?;
        let map_csum = hash(&map);
        debug!("Map checksum: {}", map_csum);
        Ok(())
    }

    fn on_stop(&mut self, reason: &Option<StopReason>) -> Result<()> {
        Ok(())
    }

    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        info!("Adding processor to context");
        let cpu: *mut conf_object =
            unsafe { SIM_attr_object_or_nil(*processor) }.expect("Attribute object expected");

        info!("Got CPU");

        let cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t = unsafe {
            SIM_c_get_interface(
                cpu,
                CPU_INSTRUMENTATION_SUBSCRIBE_INTERFACE.as_ptr() as *const i8,
            ) as *mut cpu_instrumentation_subscribe_interface_t
        };

        info!("Subscribed to CPU instrumentation");

        let cpu_instruction_query: *mut cpu_instruction_query_interface_t = unsafe {
            SIM_c_get_interface(cpu, CPU_INSTRUCTION_QUERY_INTERFACE.as_ptr() as *const i8)
                as *mut cpu_instruction_query_interface_t
        };

        info!("Got CPU query interface");

        let cpu_cached_instruction: *mut cpu_cached_instruction_interface_t = unsafe {
            SIM_c_get_interface(cpu, CPU_CACHED_INSTRUCTION_INTERFACE.as_ptr() as *const i8)
                as *mut cpu_cached_instruction_interface_t
        };

        info!("Subscribed to cached instructions");

        let processor_info_v2: *mut processor_info_v2_interface_t = unsafe {
            SIM_c_get_interface(cpu, PROCESSOR_INFO_V2_INTERFACE.as_ptr() as *const i8)
                as *mut processor_info_v2_interface_t
        };

        info!("Subscribed to processor info");

        let int_register: *mut int_register_interface_t = unsafe {
            SIM_c_get_interface(cpu, INT_REGISTER_INTERFACE.as_ptr() as *const i8)
                as *mut int_register_interface_t
        };

        info!("Subscribed to internal register queries");

        if let Some(register) =
            unsafe { *cpu_instrumentation_subscribe }.register_cached_instruction_cb
        {
            unsafe {
                register(
                    cpu,
                    null_mut(),
                    Some(simics::cached_instruction_cb),
                    null_mut(),
                )
            };
        }

        let cpu = Box::new(RefCell::new(Cpu::try_new(
            cpu,
            cpu_instrumentation_subscribe,
            cpu_instruction_query,
            cpu_cached_instruction,
            processor_info_v2,
            int_register,
        )?));

        ensure!(
            self.cpus.is_empty(),
            "A CPU has already been added! This module only supports 1 vCPU at this time."
        );

        self.cpus.push(cpu);

        Ok(())
    }
}

mod simics {
    use std::ffi::c_void;

    use confuse_simics_api::{cached_instruction_handle_t, conf_object_t, instruction_handle_t};

    #[no_mangle]
    pub extern "C" fn cached_instruction_cb(
        _obj: *mut conf_object_t,
        cpu: *mut conf_object_t,
        _cached_instruction: *mut cached_instruction_handle_t,
        instruction_query: *mut instruction_handle_t,
        _user_data: *mut c_void,
    ) {
    }
}
