use std::{
    ffi::{c_void, CStr},
    ptr::addr_of_mut,
};

use anyhow::{anyhow, bail, Result};

use confuse_simics_api::{
    conf_object_t, cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, instruction_handle_t, int_register_interface_t,
    mm_free, processor_info_v2_interface_t, SIM_attr_free, SIM_make_attr_data,
};

use log::error;

use crate::nonnull;

const BRANCH_INSTR_MNEM: &[&str] = &[
    "call", "ret", "jmp", "ja", "jae", "jb", "jbe", "jc", "jcxz", "jecxz", "jrcxz", "je", "jg",
    "jge", "jl", "jle", "jna", "jnae", "jnb", "jnbe", "jnc", "jne", "jng", "jnge", "jnl", "jnle",
    "jno", "jnp", "jns", "jnz", "jo", "jp", "jpe", "jpo", "js", "jz", "ljmp", "retf", "retfq",
    "syscall", "sysret",
];

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

    // Called in cached instruction callback to check if the current instruction is a branch
    pub fn is_branch(
        &self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<Option<u64>> {
        let instruction_bytes = match unsafe { *self.cpu_instrumentation_query }
            .get_instruction_bytes
        {
            Some(get_instruction_bytes) => unsafe { get_instruction_bytes(cpu, instruction_query) },
            _ => bail!("No function get_instruction_bytes in interface"),
        };

        let mut instruction_bytes_object = unsafe {
            SIM_make_attr_data(
                instruction_bytes.size,
                instruction_bytes.data as *const c_void,
            )
        };

        let disassembled_instruction = match unsafe { *self.processor_info_v2 }.disassemble {
            Some(disassemble) => unsafe { disassemble(cpu, 0, instruction_bytes_object, 0) },
            _ => bail!("No function disassemble in interface"),
        };

        let instruction_string =
            unsafe { CStr::from_ptr(disassembled_instruction.string) }.to_string_lossy();

        let instruction_is_branch = BRANCH_INSTR_MNEM
            .iter()
            .filter(|p| instruction_string.starts_with(*p))
            .next()
            .is_some();

        unsafe {
            SIM_attr_free(addr_of_mut!(instruction_bytes_object));
            mm_free(disassembled_instruction.string as *mut c_void);
        }

        if instruction_is_branch {
            let pc = match unsafe { *self.processor_info_v2 }.get_program_counter {
                Some(get_program_counter) => unsafe { get_program_counter(cpu) },
                _ => bail!("No function disassemble in interface"),
            };
            Ok(Some(pc))
        } else {
            Ok(None)
        }
    }
}
