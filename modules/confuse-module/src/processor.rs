use anyhow::{anyhow, Result};

use confuse_simics_api::{
    conf_object_t,
    cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, int_register_interface_t, processor_info_v2_interface_t,
};





use log::{error};



use crate::nonnull;





pub struct Processor {
    cpu: *mut conf_object_t,
    cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
    cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
    cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
    processor_info_v2: *mut processor_info_v2_interface_t,
    int_register: *mut int_register_interface_t,
}

impl Processor {
    pub fn try_new(
        cpu: *mut conf_object_t,
        // For information on these interfaces, see the "Model-to-simulator interfaces" part of the
        // documentation
        cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
        cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
        cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
        processor_info_v2: *mut processor_info_v2_interface_t,
        int_register: *mut int_register_interface_t,
    ) -> Result<Self> {
        Ok(Self {
            cpu: nonnull!(cpu)?,
            cpu_instrumentation_subscribe: nonnull!(cpu_instrumentation_subscribe)?,
            cpu_instrumentation_query: nonnull!(cpu_instrumentation_query)?,
            cpu_cached_instruction: nonnull!(cpu_cached_instruction)?,
            processor_info_v2: nonnull!(processor_info_v2)?,
            int_register: nonnull!(int_register)?,
        })
    }

    pub fn get_cpu(&self) -> *mut conf_object_t {
        self.cpu
    }

    pub fn get_int_register(&self) -> *mut int_register_interface_t {
        self.int_register
    }
}
