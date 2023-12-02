// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture specific data and definitions

use self::{
    risc_v::RISCVArchitectureOperations, x86::X86ArchitectureOperations,
    x86_64::X86_64ArchitectureOperations,
};
use crate::{tracer::TraceEntry, traits::TracerDisassembler, StartBuffer, StartSize, CLASS_NAME};
use anyhow::{anyhow, bail, Error, Result};
use raw_cstr::AsRawCstr;
use simics::{
    api::{
        get_object, read_phys_memory, sys::instruction_handle_t, write_byte, Access, AttrValueType,
        ConfObject, CpuInstructionQueryInterface, CpuInstrumentationSubscribeInterface,
        CycleInterface, GenericAddress, IntRegisterInterface, ProcessorInfoV2Interface,
    },
    trace,
};
use std::{fmt::Debug, str::FromStr};

pub mod arc;
pub mod arm;
pub mod arm_thumb2;
pub mod armv8;
pub mod risc_v;
pub mod x86;
pub mod x86_64;

#[derive(Debug, Clone)]
pub enum ArchitectureHint {
    X86_64,
    I386,
    RISCV,
}

impl FromStr for ArchitectureHint {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "x86-64" => Self::X86_64,
            "i386" | "i486" | "i586" | "i686" | "ia-32" | "x86" => Self::I386,
            "riscv" | "risc-v" | "riscv32" | "riscv64" => Self::RISCV,
            _ => bail!("Unknown hint: {}", s),
        })
    }
}

impl From<ArchitectureHint> for AttrValueType {
    fn from(val: ArchitectureHint) -> Self {
        match val {
            ArchitectureHint::X86_64 => "x86-64",
            ArchitectureHint::I386 => "i386",
            ArchitectureHint::RISCV => "risc-v",
        }
        .into()
    }
}

impl ArchitectureHint {
    pub fn architecture(&self, cpu: *mut ConfObject) -> Result<Architecture> {
        Ok(match self {
            ArchitectureHint::X86_64 => {
                Architecture::X86_64(X86_64ArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::I386 => {
                Architecture::I386(X86ArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::RISCV => {
                Architecture::RISCV(RISCVArchitectureOperations::new_unchecked(cpu)?)
            }
        })
    }
}

pub enum Architecture {
    X86_64(X86_64ArchitectureOperations),
    I386(X86ArchitectureOperations),
    RISCV(RISCVArchitectureOperations),
}

impl Debug for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Architecture::X86_64(_) => "x86-64",
                Architecture::I386(_) => "i386",
                Architecture::RISCV(_) => "risc-v",
            }
        )
    }
}
/// Each architecture must provide a struct that performs architecture-specific operations
pub trait ArchitectureOperations {
    const DEFAULT_TESTCASE_AREA_REGISTER_NAME: &'static str;
    const DEFAULT_TESTCASE_SIZE_REGISTER_NAME: &'static str;

    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized;
    fn new_unchecked(_: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        bail!("Invalid CPU");
    }
    fn cpu(&self) -> *mut ConfObject;
    fn disassembler(&mut self) -> &mut dyn TracerDisassembler;
    fn int_register(&mut self) -> &mut IntRegisterInterface;
    fn processor_info_v2(&mut self) -> &mut ProcessorInfoV2Interface;
    fn cpu_instruction_query(&mut self) -> &mut CpuInstructionQueryInterface;
    fn cpu_instrumentation_subscribe(&mut self) -> &mut CpuInstrumentationSubscribeInterface;
    fn cycle(&mut self) -> &mut CycleInterface;

    /// Returns the address and whether the address is virtual for the testcase buffer used by
    /// the magic start functionality
    fn get_magic_start_buffer(&mut self) -> Result<StartBuffer> {
        let number = self
            .int_register()
            .get_number(Self::DEFAULT_TESTCASE_AREA_REGISTER_NAME.as_raw_cstr()?)?;

        trace!(
            get_object(CLASS_NAME)?,
            "Got number {} for register {}",
            number,
            Self::DEFAULT_TESTCASE_AREA_REGISTER_NAME
        );

        let logical_address = self.int_register().read(number)?;
        trace!(
            get_object(CLASS_NAME)?,
            "Got logical address {:#x} from register",
            logical_address
        );

        let physical_address_block = self
            .processor_info_v2()
            // NOTE: Do we need to support segmented memory via logical_to_physical?
            .logical_to_physical(logical_address, Access::Sim_Access_Read)?;

        // NOTE: -1 signals no valid mapping, but this is equivalent to u64::MAX
        if physical_address_block.valid == 0 {
            bail!("Invalid linear address found in magic start buffer register {number}: {logical_address:#x}");
        } else {
            trace!(
                get_object(CLASS_NAME)?,
                "Got physical address {:#x} from logical address",
                physical_address_block.address
            );
            Ok(StartBuffer::builder()
                .physical_address(physical_address_block.address)
                .virt(physical_address_block.address != logical_address)
                .build())
        }
    }
    /// Returns the memory pointed to by the magic start functionality containing the maximum
    /// size of an input testcase
    fn get_magic_start_size(&mut self) -> Result<StartSize> {
        let number = self
            .int_register()
            .get_number(Self::DEFAULT_TESTCASE_SIZE_REGISTER_NAME.as_raw_cstr()?)?;
        let logical_address = self.int_register().read(number)?;
        let physical_address_block = self
            .processor_info_v2()
            // NOTE: Do we need to support segmented memory via logical_to_physical?
            .logical_to_physical(logical_address, Access::Sim_Access_Read)?;

        // NOTE: -1 signals no valid mapping, but this is equivalent to u64::MAX
        if physical_address_block.valid == 0 {
            bail!("Invalid linear address found in magic start buffer register {number}: {logical_address:#x}");
        }

        let size_size = self.processor_info_v2().get_logical_address_width()? / u8::BITS as i32;
        let size = read_phys_memory(self.cpu(), physical_address_block.address, size_size)?;

        Ok(StartSize::builder()
            .physical_address((
                physical_address_block.address,
                physical_address_block.address != logical_address,
            ))
            .initial_size(size)
            .build())
    }

    fn get_manual_start_buffer(
        &mut self,
        buffer_address: GenericAddress,
        virt: bool,
    ) -> Result<StartBuffer> {
        let physical_address = if virt {
            let physical_address_block = self
                .processor_info_v2()
                // NOTE: Do we need to support segmented memory via logical_to_physical?
                .logical_to_physical(buffer_address, Access::Sim_Access_Read)?;

            if physical_address_block.valid == 0 {
                bail!(
                    "Invalid linear address for given buffer address {:#x}",
                    buffer_address
                );
            }

            physical_address_block.address
        } else {
            buffer_address
        };

        Ok(StartBuffer::builder()
            .physical_address(physical_address)
            .virt(physical_address != buffer_address)
            .build())
    }

    /// Returns the initial start size for non-magic instructions by reading it from a given
    /// (possibly virtual) address
    fn get_manual_start_size(
        &mut self,
        size_address: GenericAddress,
        virt: bool,
    ) -> Result<StartSize> {
        let physical_address = if virt {
            let physical_address_block = self
                .processor_info_v2()
                // NOTE: Do we need to support segmented memory via logical_to_physical?
                .logical_to_physical(size_address, Access::Sim_Access_Read)?;

            if physical_address_block.valid == 0 {
                bail!("Invalid linear address given for start buffer : {size_address:#x}");
            }

            physical_address_block.address
        } else {
            size_address
        };

        let size_size = self.processor_info_v2().get_logical_address_width()? / u8::BITS as i32;
        let size = read_phys_memory(self.cpu(), physical_address, size_size)?;

        Ok(StartSize::builder()
            .physical_address((physical_address, physical_address != size_address))
            .initial_size(size)
            .build())
    }

    /// Writes the buffer with a testcase of a certain size
    fn write_start(
        &mut self,
        testcase: &[u8],
        buffer: &StartBuffer,
        size: &StartSize,
    ) -> Result<()> {
        let mut testcase = testcase.to_vec();
        // NOTE: We have to handle both riscv64 and riscv32 here
        let addr_size =
            self.processor_info_v2().get_logical_address_width()? as usize / u8::BITS as usize;
        let initial_size =
            size.initial_size()
                .ok_or_else(|| anyhow!("Expected initial size for start"))? as usize;

        let physical_memory = self.processor_info_v2().get_physical_memory()?;

        trace!(
            get_object(CLASS_NAME)?,
            "Truncating testcase to {initial_size} bytes (from {} bytes)",
            testcase.len()
        );

        testcase.truncate(initial_size);

        testcase.iter().enumerate().try_for_each(|(i, c)| {
            let physical_address = buffer.physical_address + (i as u64);
            write_byte(physical_memory, physical_address, *c)
        })?;

        if let Some((address, _)) = size.physical_address {
            testcase
                .len()
                .to_le_bytes()
                .iter()
                .take(addr_size)
                .enumerate()
                .try_for_each(|(i, c)| {
                    let physical_address = address + (i as u64);
                    write_byte(physical_memory, physical_address, *c)
                })?;
        } else {
            trace!(
                get_object(CLASS_NAME)?,
                "Not writing testcase size, no physical address saved for size"
            );
        }

        Ok(())
    }

    fn trace_pc(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry>;
    fn trace_cmp(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry>;
}

impl ArchitectureOperations for Architecture {
    const DEFAULT_TESTCASE_AREA_REGISTER_NAME: &'static str = "";
    const DEFAULT_TESTCASE_SIZE_REGISTER_NAME: &'static str = "";

    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        if let Ok(x86_64) = X86_64ArchitectureOperations::new(cpu) {
            Ok(Self::X86_64(x86_64))
        } else if let Ok(x86) = X86ArchitectureOperations::new(cpu) {
            Ok(Self::I386(x86))
        } else if let Ok(riscv) = RISCVArchitectureOperations::new(cpu) {
            Ok(Self::RISCV(riscv))
        } else {
            bail!("Unsupported architecture");
        }
    }

    fn cpu(&self) -> *mut ConfObject {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu(),
            Architecture::I386(i386) => i386.cpu(),
            Architecture::RISCV(riscv) => riscv.cpu(),
        }
    }

    fn disassembler(&mut self) -> &mut dyn TracerDisassembler {
        match self {
            Architecture::X86_64(x86_64) => x86_64.disassembler(),
            Architecture::I386(i386) => i386.disassembler(),
            Architecture::RISCV(riscv) => riscv.disassembler(),
        }
    }

    fn int_register(&mut self) -> &mut IntRegisterInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.int_register(),
            Architecture::I386(i386) => i386.int_register(),
            Architecture::RISCV(riscv) => riscv.int_register(),
        }
    }

    fn processor_info_v2(&mut self) -> &mut ProcessorInfoV2Interface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.processor_info_v2(),
            Architecture::I386(i386) => i386.processor_info_v2(),
            Architecture::RISCV(riscv) => riscv.processor_info_v2(),
        }
    }

    fn cpu_instruction_query(&mut self) -> &mut CpuInstructionQueryInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu_instruction_query(),
            Architecture::I386(i386) => i386.cpu_instruction_query(),
            Architecture::RISCV(riscv) => riscv.cpu_instruction_query(),
        }
    }

    fn cpu_instrumentation_subscribe(&mut self) -> &mut CpuInstrumentationSubscribeInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu_instrumentation_subscribe(),
            Architecture::I386(i386) => i386.cpu_instrumentation_subscribe(),
            Architecture::RISCV(riscv) => riscv.cpu_instrumentation_subscribe(),
        }
    }

    fn cycle(&mut self) -> &mut CycleInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cycle(),
            Architecture::I386(i386) => i386.cycle(),
            Architecture::RISCV(riscv) => riscv.cycle(),
        }
    }

    fn get_magic_start_buffer(&mut self) -> Result<StartBuffer> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer(),
            Architecture::I386(i386) => i386.get_magic_start_buffer(),
            Architecture::RISCV(riscv) => riscv.get_magic_start_buffer(),
        }
    }

    fn get_magic_start_size(&mut self) -> Result<StartSize> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_size(),
            Architecture::I386(i386) => i386.get_magic_start_size(),
            Architecture::RISCV(riscv) => riscv.get_magic_start_size(),
        }
    }

    fn get_manual_start_buffer(
        &mut self,
        buffer_address: GenericAddress,
        virt: bool,
    ) -> Result<StartBuffer> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_manual_start_buffer(buffer_address, virt),
            Architecture::I386(i386) => i386.get_manual_start_buffer(buffer_address, virt),
            Architecture::RISCV(riscv) => riscv.get_manual_start_buffer(buffer_address, virt),
        }
    }

    fn get_manual_start_size(
        &mut self,
        size_address: GenericAddress,
        virt: bool,
    ) -> Result<StartSize> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_manual_start_size(size_address, virt),
            Architecture::I386(i386) => i386.get_manual_start_size(size_address, virt),
            Architecture::RISCV(riscv) => riscv.get_manual_start_size(size_address, virt),
        }
    }

    fn write_start(
        &mut self,
        testcase: &[u8],
        buffer: &StartBuffer,
        size: &StartSize,
    ) -> Result<()> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.write_start(testcase, buffer, size),
            Architecture::I386(i386) => i386.write_start(testcase, buffer, size),
            Architecture::RISCV(riscv) => riscv.write_start(testcase, buffer, size),
        }
    }

    fn trace_pc(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.trace_pc(instruction_query),
            Architecture::I386(i386) => i386.trace_pc(instruction_query),
            Architecture::RISCV(riscv) => riscv.trace_pc(instruction_query),
        }
    }

    fn trace_cmp(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.trace_cmp(instruction_query),
            Architecture::I386(i386) => i386.trace_cmp(instruction_query),
            Architecture::RISCV(riscv) => riscv.trace_cmp(instruction_query),
        }
    }
}
