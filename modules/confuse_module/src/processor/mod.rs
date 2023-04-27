//! Implements generic processor operations on the simulated CPU or CPUs
use std::ffi::c_void;

use anyhow::{bail, Result};
use simics_api::{
    attr_string, get_attribute, ConfObject, CpuCachedInstruction, CpuInstructionQuery,
    CpuInstrumentationSubscribe, Cycle, InstructionHandle, IntRegister, OwnedMutAttrValuePtr,
    OwnedMutConfObjectPtr, OwnedMutInstructionHandlePtr, ProcessorInfoV2,
};

mod disassembler;

use disassembler::x86_64::Disassembler as X86_64Disassembler;

use crate::traits::TracerDisassembler;

pub struct Processor {
    number: i32,
    cpu: OwnedMutConfObjectPtr,
    arch: String,
    disassembler: Box<dyn TracerDisassembler>,
    cpu_instrumentation_subscribe: Option<CpuInstrumentationSubscribe>,
    cpu_instruction_query: Option<CpuInstructionQuery>,
    cpu_cached_instruction: Option<CpuCachedInstruction>,
    processor_info_v2: Option<ProcessorInfoV2>,
    int_register: Option<IntRegister>,
    cycle: Option<Cycle>,
}

impl Processor {
    pub fn try_new(number: i32, cpu: &OwnedMutConfObjectPtr) -> Result<Self> {
        let cpu = cpu.clone();
        let arch = attr_string(get_attribute(cpu.clone(), "architecture")?)?;

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
        })
    }

    pub fn try_with_cpu_instrumentation_subscribe(
        mut self,
        processor_attr: OwnedMutAttrValuePtr,
    ) -> Result<Self> {
        self.cpu_instrumentation_subscribe =
            Some(CpuInstrumentationSubscribe::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_instruction_query(
        mut self,
        processor_attr: OwnedMutAttrValuePtr,
    ) -> Result<Self> {
        self.cpu_instruction_query = Some(CpuInstructionQuery::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_cached_instruction(
        mut self,
        processor_attr: OwnedMutAttrValuePtr,
    ) -> Result<Self> {
        self.cpu_cached_instruction = Some(CpuCachedInstruction::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_processor_info_v2(
        mut self,
        processor_attr: OwnedMutAttrValuePtr,
    ) -> Result<Self> {
        self.processor_info_v2 = Some(ProcessorInfoV2::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_int_register(mut self, processor_attr: OwnedMutAttrValuePtr) -> Result<Self> {
        self.int_register = Some(IntRegister::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cycle(mut self, processor_attr: OwnedMutAttrValuePtr) -> Result<Self> {
        self.cycle = Some(Cycle::try_new(processor_attr)?);
        Ok(self)
    }
}

impl Processor {
    pub fn arch(&self) -> String {
        self.arch.clone()
    }
}

impl Processor {
    pub fn register_instruction_before_cb(
        &mut self,
        cpu: OwnedMutConfObjectPtr,
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
        cpu: OwnedMutConfObjectPtr,
        instruction_query: OwnedMutInstructionHandlePtr,
    ) -> Result<Option<u64>> {
        if let Some(cpu_instruction_query) = self.cpu_instruction_query.as_mut() {
            let bytes =
                cpu_instruction_query.get_instruction_bytes(cpu.clone(), instruction_query)?;
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
}
