//! Implements generic processor operations on the simulated CPU or CPUs
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use simics_api::{
    attr_string, get_attribute, read_byte, write_byte, AttrValue, ConfObject, CpuCachedInstruction,
    CpuInstructionQuery, CpuInstrumentationSubscribe, Cycle, InstructionHandle, IntRegister,
    ProcessorInfoV2,
};
use std::{collections::HashMap, ffi::c_void, os::unix::process};

mod disassembler;

use disassembler::x86_64::Disassembler as X86_64Disassembler;

use crate::traits::TracerDisassembler;

pub struct Processor {
    number: i32,
    cpu: *mut ConfObject,
    arch: String,
    disassembler: Box<dyn TracerDisassembler>,
    cpu_instrumentation_subscribe: Option<CpuInstrumentationSubscribe>,
    cpu_instruction_query: Option<CpuInstructionQuery>,
    cpu_cached_instruction: Option<CpuCachedInstruction>,
    processor_info_v2: Option<ProcessorInfoV2>,
    int_register: Option<IntRegister>,
    cycle: Option<Cycle>,
    reg_numbers: HashMap<String, i32>,
}

impl Processor {
    pub fn number(&self) -> i32 {
        self.number
    }

    pub fn cpu(&self) -> *mut ConfObject {
        self.cpu
    }

    pub fn arch(&self) -> String {
        self.arch.clone()
    }
}

impl Processor {
    pub fn try_new(number: i32, cpu: *mut ConfObject) -> Result<Self> {
        let arch = attr_string(get_attribute(cpu, "architecture")?)?;

        let disassembler = match arch.as_str() {
            "x86-64" => Box::new(X86_64Disassembler::new()),
            _ => {
                bail!("Unsupported architecture {}", arch)
            }
        };

        Ok(Self {
            number,
            cpu,
            arch,
            disassembler,
            cpu_instrumentation_subscribe: None,
            cpu_instruction_query: None,
            cpu_cached_instruction: None,
            processor_info_v2: None,
            int_register: None,
            cycle: None,
            reg_numbers: HashMap::new(),
        })
    }

    pub fn try_with_cpu_instrumentation_subscribe(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_instrumentation_subscribe =
            Some(CpuInstrumentationSubscribe::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_instruction_query(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_instruction_query = Some(CpuInstructionQuery::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_cached_instruction(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_cached_instruction = Some(CpuCachedInstruction::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_processor_info_v2(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.processor_info_v2 = Some(ProcessorInfoV2::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_int_register(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.int_register = Some(IntRegister::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cycle(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.cycle = Some(Cycle::try_new(processor_attr)?);
        Ok(self)
    }
}

impl Processor {
    pub fn register_instruction_before_cb(
        &mut self,
        cpu: *mut ConfObject,
        cb: unsafe extern "C" fn(
            *mut ConfObject,
            *mut ConfObject,
            *mut InstructionHandle,
            *mut c_void,
        ),
    ) -> Result<()> {
        if let Some(cpu_instrumentation_subscribe) = self.cpu_instrumentation_subscribe.as_mut() {
            cpu_instrumentation_subscribe.register_instruction_before_cb(cpu, cb)?;
        }

        Ok(())
    }

    pub fn trace(
        &mut self,
        cpu: *mut ConfObject,
        instruction_query: *mut InstructionHandle,
    ) -> Result<Option<u64>> {
        if let Some(cpu_instruction_query) = self.cpu_instruction_query.as_mut() {
            let bytes = cpu_instruction_query.get_instruction_bytes(cpu, instruction_query)?;
            self.disassembler.disassemble(bytes)?;

            if self.disassembler.last_was_call()?
                || self.disassembler.last_was_control_flow()?
                || self.disassembler.last_was_ret()?
            {
                if let Some(processor_info_v2) = self.processor_info_v2.as_mut() {
                    Ok(processor_info_v2.get_program_counter(cpu).ok())
                } else {
                    bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
                }
            } else {
                Ok(None)
            }
        } else {
            bail!("No CpuInstructionQuery interface registered in processor. Try building with `try_with_cpu_instruction_query`");
        }
    }

    pub fn get_reg_value<S: AsRef<str>>(&mut self, reg: S) -> Result<u64> {
        let int_register = if let Some(int_register) = self.int_register.as_ref() {
            int_register
        } else {
            bail!("No IntRegister interface registered in processor. Try building with `try_with_int_register`");
        };

        let reg_number = if let Some(reg_number) = self.reg_numbers.get(reg.as_ref()) {
            *reg_number
        } else {
            let reg_name = reg.as_ref().to_string();
            let reg_number = int_register.get_number(self.cpu, reg)?;
            self.reg_numbers.insert(reg_name, reg_number);
            reg_number
        };

        int_register.read(self.cpu, reg_number)
    }

    pub fn write_bytes(&self, logical_address_start: u64, bytes: &[u8]) -> Result<()> {
        let processor_info_v2 = if let Some(processor_info_v2) = self.processor_info_v2.as_ref() {
            processor_info_v2
        } else {
            bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
        };

        let physical_memory = processor_info_v2.get_physical_memory(self.cpu)?;

        for (i, byte) in bytes.iter().enumerate() {
            let logical_address = logical_address_start + i as u64;
            let physical_address =
                processor_info_v2.logical_to_physical(self.cpu, logical_address)?;
            write_byte(physical_memory, physical_address.address, *byte);
            // let written = read_byte(physical_memory, physical_address);
            // ensure!(written == *byte, "Did not read back same written byte");
        }

        Ok(())
    }
}
