// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture specific data and definitions

use self::{
    aarch64::AArch64ArchitectureOperations, arm::ARMArchitectureOperations,
    risc_v::RISCVArchitectureOperations, x86::X86ArchitectureOperations,
    x86_64::X86_64ArchitectureOperations,
};
use crate::{
    tracer::TraceEntry, traits::TracerDisassembler, ManualStartAddress, ManualStartInfo, StartInfo,
    StartPhysicalAddress, StartSize,
};
use anyhow::anyhow;
use anyhow::{bail, ensure, Error, Result};
use raw_cstr::AsRawCstr;
use simics::{
    api::{
        read_phys_memory, sys::instruction_handle_t, write_byte, Access, AttrValueType, ConfObject,
        CpuInstructionQueryInterface, CpuInstrumentationSubscribeInterface, CycleInterface,
        IntRegisterInterface, ProcessorInfoV2Interface,
    },
    read_byte,
};
use std::{fmt::Debug, str::FromStr};

pub mod aarch64;
pub mod arm;
pub mod risc_v;
pub mod x86;
pub mod x86_64;

#[derive(Debug, Clone)]
/// An architecture hint that can be parsed from a string
pub(crate) enum ArchitectureHint {
    /// The architecture is x86_64
    X86_64,
    /// The architecture is i386
    I386,
    /// The architecture is RISCV
    Riscv,
    /// The architecture is arm
    Arm,
    /// The architecture is aarch64
    Aarch64,
}

impl FromStr for ArchitectureHint {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "x86-64" => Self::X86_64,
            "i386" | "i486" | "i586" | "i686" | "ia-32" | "x86" => Self::I386,
            "riscv" | "risc-v" | "riscv32" | "riscv64" => Self::Riscv,
            "armv4" | "armv5" | "armv6" | "armv7" | "arm" | "arm32" => Self::Arm,
            "aarch64" | "armv8" | "arm64" => Self::Aarch64,
            _ => bail!("Unknown hint: {}", s),
        })
    }
}

impl From<ArchitectureHint> for AttrValueType {
    fn from(val: ArchitectureHint) -> Self {
        match val {
            ArchitectureHint::X86_64 => "x86-64",
            ArchitectureHint::I386 => "i386",
            ArchitectureHint::Riscv => "risc-v",
            ArchitectureHint::Arm => "arm",
            ArchitectureHint::Aarch64 => "aarch64",
        }
        .into()
    }
}

impl ArchitectureHint {
    /// Return the architecture for the given CPU object
    pub fn architecture(&self, cpu: *mut ConfObject) -> Result<Architecture> {
        Ok(match self {
            ArchitectureHint::X86_64 => {
                Architecture::X86_64(X86_64ArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::I386 => {
                Architecture::I386(X86ArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::Riscv => {
                Architecture::Riscv(RISCVArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::Arm => {
                Architecture::Arm(ARMArchitectureOperations::new_unchecked(cpu)?)
            }
            ArchitectureHint::Aarch64 => {
                Architecture::Aarch64(AArch64ArchitectureOperations::new_unchecked(cpu)?)
            }
        })
    }
}

pub(crate) enum Architecture {
    /// The x86_64 architecture
    X86_64(X86_64ArchitectureOperations),
    /// The i386 architecture
    I386(X86ArchitectureOperations),
    /// The RISC-V architecture
    Riscv(RISCVArchitectureOperations),
    /// The ARM architecture (v7 and below)
    Arm(ARMArchitectureOperations),
    /// The AARCH64 architecture (v8 and above)
    Aarch64(AArch64ArchitectureOperations),
}

impl Debug for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Architecture::X86_64(_) => "x86-64",
                Architecture::I386(_) => "i386",
                Architecture::Riscv(_) => "risc-v",
                Architecture::Arm(_) => "arm",
                Architecture::Aarch64(_) => "aarch64",
            }
        )
    }
}
/// Each architecture must provide a struct that performs architecture-specific operations
pub trait ArchitectureOperations {
    const INDEX_SELECTOR_REGISTER: &'static str;
    const ARGUMENT_REGISTER_0: &'static str;
    const ARGUMENT_REGISTER_1: &'static str;
    const ARGUMENT_REGISTER_2: &'static str;
    const POINTER_WIDTH_OVERRIDE: Option<i32> = None;

    /// Create a new instance of the architecture operations
    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized;

    /// Create a new instance of the architecture operations without checking the CPU
    fn new_unchecked(_: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        bail!("Invalid CPU");
    }

    /// Return the saved CPU object for this archtiecture
    fn cpu(&self) -> *mut ConfObject;

    /// Return a mutable reference to the disassembler for this architecture
    fn disassembler(&mut self) -> &mut dyn TracerDisassembler;

    /// Return a mutable reference to the interface for reading and writing registers
    fn int_register(&mut self) -> &mut IntRegisterInterface;

    /// Return a mutable reference to the interface for reading processor information
    fn processor_info_v2(&mut self) -> &mut ProcessorInfoV2Interface;

    /// Return a mutable reference to the interface for querying CPU instructions
    fn cpu_instruction_query(&mut self) -> &mut CpuInstructionQueryInterface;

    /// Return a mutable reference to the interface for subscribing to CPU instrumentation events
    fn cpu_instrumentation_subscribe(&mut self) -> &mut CpuInstrumentationSubscribeInterface;

    /// Return a mutable reference to the interface for querying CPU cycles and timing
    fn cycle(&mut self) -> &mut CycleInterface;

    /// Return the value of the magic index selector register, which is used to determine
    /// whether a magic instruction should be used or skipped.
    fn get_magic_index_selector(&mut self) -> Result<u64> {
        Ok(self
            .int_register()
            .get_number(Self::INDEX_SELECTOR_REGISTER.as_raw_cstr()?)
            .and_then(|n| self.int_register().read(n))?)
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_ptr: A pointer to a pointer-sized variable containing the size of the testcase
    fn get_magic_start_buffer_ptr_size_ptr(&mut self) -> Result<StartInfo> {
        let buffer_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_0.as_raw_cstr()?)?;
        let size_ptr_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_1.as_raw_cstr()?)?;
        let buffer_logical_address = self.int_register().read(buffer_register_number)?;
        let size_ptr_logical_address = self.int_register().read(size_ptr_register_number)?;
        let buffer_physical_address_block = self
            .processor_info_v2()
            .logical_to_physical(buffer_logical_address, Access::Sim_Access_Read)?;
        let size_ptr_physical_address_block = self
            .processor_info_v2()
            .logical_to_physical(size_ptr_logical_address, Access::Sim_Access_Read)?;

        ensure!(
            buffer_physical_address_block.valid != 0,
            "Invalid linear address found in magic start buffer register {buffer_register_number}: {buffer_logical_address:#x}"
        );
        ensure!(
            size_ptr_physical_address_block.valid != 0,
            "Invalid linear address found in magic start size register {size_ptr_register_number}: {size_ptr_logical_address:#x}"
        );

        let size_size = if let Some(width) = Self::POINTER_WIDTH_OVERRIDE {
            width
        } else {
            self.processor_info_v2().get_logical_address_width()? / u8::BITS as i32
        };

        let size = read_phys_memory(
            self.cpu(),
            size_ptr_physical_address_block.address,
            size_size,
        )?;

        let contents = (0..size)
            .map(|i| {
                read_byte(
                    self.processor_info_v2().get_physical_memory()?,
                    buffer_physical_address_block.address + i,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to read byte at {:#x}: {}",
                        buffer_physical_address_block.address + i,
                        e
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(StartInfo::builder()
            .address(
                if buffer_physical_address_block.address != buffer_logical_address {
                    StartPhysicalAddress::WasVirtual(buffer_physical_address_block.address)
                } else {
                    StartPhysicalAddress::WasPhysical(buffer_physical_address_block.address)
                },
            )
            .contents(contents)
            .size(StartSize::SizePtr {
                address: if size_ptr_physical_address_block.address != size_ptr_logical_address {
                    StartPhysicalAddress::WasVirtual(size_ptr_physical_address_block.address)
                } else {
                    StartPhysicalAddress::WasPhysical(size_ptr_physical_address_block.address)
                },
                maximum_size: size as usize,
            })
            .build())
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_val: The maximum size of the testcase
    fn get_magic_start_buffer_ptr_size_val(&mut self) -> Result<StartInfo> {
        let buffer_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_0.as_raw_cstr()?)?;
        let size_val_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_1.as_raw_cstr()?)?;
        let buffer_logical_address = self.int_register().read(buffer_register_number)?;
        let size_val = self.int_register().read(size_val_register_number)?;
        let buffer_physical_address_block = self
            .processor_info_v2()
            .logical_to_physical(buffer_logical_address, Access::Sim_Access_Read)?;

        ensure!(
            buffer_physical_address_block.valid != 0,
            "Invalid linear address found in magic start buffer register {buffer_register_number}: {buffer_logical_address:#x}"
        );

        let contents = (0..size_val)
            .map(|i| {
                read_byte(
                    self.processor_info_v2().get_physical_memory()?,
                    buffer_physical_address_block.address + i,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to read byte at {:#x}: {}",
                        buffer_physical_address_block.address + i,
                        e
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(StartInfo::builder()
            .address(
                if buffer_physical_address_block.address != buffer_logical_address {
                    StartPhysicalAddress::WasVirtual(buffer_physical_address_block.address)
                } else {
                    StartPhysicalAddress::WasPhysical(buffer_physical_address_block.address)
                },
            )
            .contents(contents)
            .size(StartSize::MaxSize(size_val as usize))
            .build())
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_ptr: A pointer to a pointer-sized variable to which the size is written
    /// - size_val: The maximum size of the testcase
    fn get_magic_start_buffer_ptr_size_ptr_val(&mut self) -> Result<StartInfo> {
        let buffer_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_0.as_raw_cstr()?)?;
        let size_ptr_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_1.as_raw_cstr()?)?;
        let size_val_register_number = self
            .int_register()
            .get_number(Self::ARGUMENT_REGISTER_2.as_raw_cstr()?)?;

        let buffer_logical_address = self.int_register().read(buffer_register_number)?;
        let size_ptr_logical_address = self.int_register().read(size_ptr_register_number)?;
        let size_val = self.int_register().read(size_val_register_number)?;

        let buffer_physical_address_block = self
            .processor_info_v2()
            .logical_to_physical(buffer_logical_address, Access::Sim_Access_Read)?;

        let size_ptr_physical_address_block = self
            .processor_info_v2()
            .logical_to_physical(size_ptr_logical_address, Access::Sim_Access_Read)?;

        ensure!(
            buffer_physical_address_block.valid != 0,
            "Invalid linear address found in magic start buffer register {buffer_register_number}: {buffer_logical_address:#x}"
        );
        ensure!(
            size_ptr_physical_address_block.valid != 0,
            "Invalid linear address found in magic start size register {size_ptr_register_number}: {size_ptr_logical_address:#x}"
        );

        let contents = (0..size_val)
            .map(|i| {
                read_byte(
                    self.processor_info_v2().get_physical_memory()?,
                    buffer_physical_address_block.address + i,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to read byte at {:#x}: {}",
                        buffer_physical_address_block.address + i,
                        e
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(StartInfo::builder()
            .address(
                if buffer_physical_address_block.address != buffer_logical_address {
                    StartPhysicalAddress::WasVirtual(buffer_physical_address_block.address)
                } else {
                    StartPhysicalAddress::WasPhysical(buffer_physical_address_block.address)
                },
            )
            .contents(contents)
            .size(StartSize::SizePtrAndMaxSize {
                address: if size_ptr_physical_address_block.address != size_ptr_logical_address {
                    StartPhysicalAddress::WasVirtual(size_ptr_physical_address_block.address)
                } else {
                    StartPhysicalAddress::WasPhysical(size_ptr_physical_address_block.address)
                },
                maximum_size: size_val as usize,
            })
            .build())
    }

    /// Returns the address and whether the address is virtual for the testcase buffer used by
    /// the manual start functionality
    fn get_manual_start_info(&mut self, info: &ManualStartInfo) -> Result<StartInfo> {
        let buffer_physical_address = if matches!(info.address, ManualStartAddress::Virtual(_)) {
            let physical_address_block = self
                .processor_info_v2()
                // NOTE: Do we need to support segmented memory via logical_to_physical?
                .logical_to_physical(
                    match info.address {
                        ManualStartAddress::Virtual(address) => address,
                        ManualStartAddress::Physical(address) => address,
                    },
                    Access::Sim_Access_Read,
                )?;

            if physical_address_block.valid == 0 {
                bail!(
                    "Invalid linear address for given buffer address {:?}",
                    info.address
                );
            }

            physical_address_block.address
        } else {
            info.address.address()
        };

        let address = StartPhysicalAddress::WasPhysical(buffer_physical_address);

        let size = match &info.size {
            crate::ManualStartSize::SizePtr { address } => {
                let address = match address {
                    ManualStartAddress::Virtual(v) => {
                        let physical_address = self
                            .processor_info_v2()
                            .logical_to_physical(*v, Access::Sim_Access_Read)?;

                        if physical_address.valid == 0 {
                            bail!("Invalid linear address given for start buffer : {v:#x}");
                        }

                        StartPhysicalAddress::WasVirtual(physical_address.address)
                    }
                    ManualStartAddress::Physical(p) => StartPhysicalAddress::WasPhysical(*p),
                };

                let size_size = if let Some(width) = Self::POINTER_WIDTH_OVERRIDE {
                    width
                } else {
                    self.processor_info_v2().get_logical_address_width()? / u8::BITS as i32
                };
                let maximum_size =
                    read_phys_memory(self.cpu(), address.physical_address(), size_size)?;
                StartSize::SizePtr {
                    address,
                    maximum_size: maximum_size as usize,
                }
            }
            crate::ManualStartSize::MaxSize(maximum_size) => StartSize::MaxSize(*maximum_size),
            crate::ManualStartSize::SizePtrAndMaxSize {
                address,
                maximum_size,
            } => {
                let address = match address {
                    ManualStartAddress::Virtual(v) => {
                        let physical_address = self
                            .processor_info_v2()
                            .logical_to_physical(*v, Access::Sim_Access_Read)?;

                        if physical_address.valid == 0 {
                            bail!("Invalid linear address given for start buffer : {v:#x}");
                        }

                        StartPhysicalAddress::WasVirtual(physical_address.address)
                    }
                    ManualStartAddress::Physical(p) => StartPhysicalAddress::WasPhysical(*p),
                };

                StartSize::SizePtrAndMaxSize {
                    address,
                    maximum_size: *maximum_size,
                }
            }
        };

        let contents = (0..size.maximum_size())
            .map(|i| {
                read_byte(
                    self.processor_info_v2().get_physical_memory()?,
                    buffer_physical_address + i as u64,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to read byte at {:#x}: {}",
                        buffer_physical_address + i as u64,
                        e
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(StartInfo::builder()
            .address(address)
            .contents(contents)
            .size(size)
            .build())
    }

    fn write_start(&mut self, testcase: &[u8], info: &StartInfo) -> Result<()> {
        let mut testcase = testcase.to_vec();
        // NOTE: We have to handle both riscv64 and riscv32 here
        let addr_size =
            self.processor_info_v2().get_logical_address_width()? as usize / u8::BITS as usize;

        let physical_memory = self.processor_info_v2().get_physical_memory()?;

        testcase.truncate(info.size.maximum_size());

        testcase.iter().enumerate().try_for_each(|(i, c)| {
            let physical_address = info.address.physical_address() + (i as u64);
            write_byte(physical_memory, physical_address, *c)
        })?;

        if let Some(size_address) = info.size.physical_address().map(|s| s.physical_address()) {
            testcase
                .len()
                .to_le_bytes()
                .iter()
                .take(addr_size)
                .enumerate()
                .try_for_each(|(i, c)| {
                    let physical_address = size_address + (i as u64);
                    write_byte(physical_memory, physical_address, *c)
                })?;
        }

        Ok(())
    }

    fn trace_pc(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry>;
    fn trace_cmp(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry>;
}

impl ArchitectureOperations for Architecture {
    const INDEX_SELECTOR_REGISTER: &'static str = "";
    const ARGUMENT_REGISTER_0: &'static str = "";
    const ARGUMENT_REGISTER_1: &'static str = "";
    const ARGUMENT_REGISTER_2: &'static str = "";

    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        if let Ok(x86_64) = X86_64ArchitectureOperations::new(cpu) {
            Ok(Self::X86_64(x86_64))
        } else if let Ok(x86) = X86ArchitectureOperations::new(cpu) {
            Ok(Self::I386(x86))
        } else if let Ok(riscv) = RISCVArchitectureOperations::new(cpu) {
            Ok(Self::Riscv(riscv))
        } else if let Ok(arm) = ARMArchitectureOperations::new(cpu) {
            Ok(Self::Arm(arm))
        } else if let Ok(aarch64) = AArch64ArchitectureOperations::new(cpu) {
            Ok(Self::Aarch64(aarch64))
        } else {
            bail!("Unsupported architecture");
        }
    }

    fn cpu(&self) -> *mut ConfObject {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu(),
            Architecture::I386(i386) => i386.cpu(),
            Architecture::Riscv(riscv) => riscv.cpu(),
            Architecture::Arm(arm) => arm.cpu(),
            Architecture::Aarch64(aarch64) => aarch64.cpu(),
        }
    }

    fn disassembler(&mut self) -> &mut dyn TracerDisassembler {
        match self {
            Architecture::X86_64(x86_64) => x86_64.disassembler(),
            Architecture::I386(i386) => i386.disassembler(),
            Architecture::Riscv(riscv) => riscv.disassembler(),
            Architecture::Arm(arm) => arm.disassembler(),
            Architecture::Aarch64(aarch64) => aarch64.disassembler(),
        }
    }

    fn int_register(&mut self) -> &mut IntRegisterInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.int_register(),
            Architecture::I386(i386) => i386.int_register(),
            Architecture::Riscv(riscv) => riscv.int_register(),
            Architecture::Arm(arm) => arm.int_register(),
            Architecture::Aarch64(aarch64) => aarch64.int_register(),
        }
    }

    fn processor_info_v2(&mut self) -> &mut ProcessorInfoV2Interface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.processor_info_v2(),
            Architecture::I386(i386) => i386.processor_info_v2(),
            Architecture::Riscv(riscv) => riscv.processor_info_v2(),
            Architecture::Arm(arm) => arm.processor_info_v2(),
            Architecture::Aarch64(aarch64) => aarch64.processor_info_v2(),
        }
    }

    fn cpu_instruction_query(&mut self) -> &mut CpuInstructionQueryInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu_instruction_query(),
            Architecture::I386(i386) => i386.cpu_instruction_query(),
            Architecture::Riscv(riscv) => riscv.cpu_instruction_query(),
            Architecture::Arm(arm) => arm.cpu_instruction_query(),
            Architecture::Aarch64(aarch64) => aarch64.cpu_instruction_query(),
        }
    }

    fn cpu_instrumentation_subscribe(&mut self) -> &mut CpuInstrumentationSubscribeInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cpu_instrumentation_subscribe(),
            Architecture::I386(i386) => i386.cpu_instrumentation_subscribe(),
            Architecture::Riscv(riscv) => riscv.cpu_instrumentation_subscribe(),
            Architecture::Arm(arm) => arm.cpu_instrumentation_subscribe(),
            Architecture::Aarch64(aarch64) => aarch64.cpu_instrumentation_subscribe(),
        }
    }

    fn cycle(&mut self) -> &mut CycleInterface {
        match self {
            Architecture::X86_64(x86_64) => x86_64.cycle(),
            Architecture::I386(i386) => i386.cycle(),
            Architecture::Riscv(riscv) => riscv.cycle(),
            Architecture::Arm(arm) => arm.cycle(),
            Architecture::Aarch64(aarch64) => aarch64.cycle(),
        }
    }

    fn get_magic_index_selector(&mut self) -> Result<u64> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_index_selector(),
            Architecture::I386(i386) => i386.get_magic_index_selector(),
            Architecture::Riscv(riscv) => riscv.get_magic_index_selector(),
            Architecture::Arm(arm) => arm.get_magic_index_selector(),
            Architecture::Aarch64(aarch64) => aarch64.get_magic_index_selector(),
        }
    }

    fn get_magic_start_buffer_ptr_size_ptr(&mut self) -> Result<StartInfo> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::I386(i386) => i386.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::Riscv(riscv) => riscv.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::Arm(arm) => arm.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::Aarch64(aarch64) => aarch64.get_magic_start_buffer_ptr_size_ptr(),
        }
    }

    fn get_magic_start_buffer_ptr_size_val(&mut self) -> Result<StartInfo> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer_ptr_size_val(),
            Architecture::I386(i386) => i386.get_magic_start_buffer_ptr_size_val(),
            Architecture::Riscv(riscv) => riscv.get_magic_start_buffer_ptr_size_val(),
            Architecture::Arm(arm) => arm.get_magic_start_buffer_ptr_size_val(),
            Architecture::Aarch64(aarch64) => aarch64.get_magic_start_buffer_ptr_size_val(),
        }
    }

    fn get_magic_start_buffer_ptr_size_ptr_val(&mut self) -> Result<StartInfo> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::I386(i386) => i386.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::Riscv(riscv) => riscv.get_magic_start_buffer_ptr_size_ptr(),
            Architecture::Arm(arm) => arm.get_magic_start_buffer_ptr_size_ptr_val(),
            Architecture::Aarch64(aarch64) => aarch64.get_magic_start_buffer_ptr_size_ptr_val(),
        }
    }

    fn get_manual_start_info(&mut self, info: &ManualStartInfo) -> Result<StartInfo> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_manual_start_info(info),
            Architecture::I386(i386) => i386.get_manual_start_info(info),
            Architecture::Riscv(riscv) => riscv.get_manual_start_info(info),
            Architecture::Arm(arm) => arm.get_manual_start_info(info),
            Architecture::Aarch64(aarch64) => aarch64.get_manual_start_info(info),
        }
    }

    fn write_start(&mut self, testcase: &[u8], info: &StartInfo) -> Result<()> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.write_start(testcase, info),
            Architecture::I386(i386) => i386.write_start(testcase, info),
            Architecture::Riscv(riscv) => riscv.write_start(testcase, info),
            Architecture::Arm(arm) => arm.write_start(testcase, info),
            Architecture::Aarch64(aarch64) => aarch64.write_start(testcase, info),
        }
    }

    fn trace_pc(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.trace_pc(instruction_query),
            Architecture::I386(i386) => i386.trace_pc(instruction_query),
            Architecture::Riscv(riscv) => riscv.trace_pc(instruction_query),
            Architecture::Arm(arm) => arm.trace_pc(instruction_query),
            Architecture::Aarch64(aarch64) => aarch64.trace_pc(instruction_query),
        }
    }

    fn trace_cmp(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.trace_cmp(instruction_query),
            Architecture::I386(i386) => i386.trace_cmp(instruction_query),
            Architecture::Riscv(riscv) => riscv.trace_cmp(instruction_query),
            Architecture::Arm(arm) => arm.trace_cmp(instruction_query),
            Architecture::Aarch64(aarch64) => aarch64.trace_cmp(instruction_query),
        }
    }
}
