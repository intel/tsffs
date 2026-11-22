// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture specific data and definitions

use self::{
    aarch64::AArch64ArchitectureOperations, arm::ARMArchitectureOperations,
    risc_v::RISCVArchitectureOperations, x86::X86ArchitectureOperations,
    x86_64::X86_64ArchitectureOperations,
};
use crate::{
    tracer::{DecodedComparison, DecodedInstruction, TraceEntry},
    traits::TracerDisassembler,
    ManualStartAddress, ManualStartInfo, StartInfo, StartPhysicalAddress, StartSize,
};
use anyhow::anyhow;
use anyhow::{bail, ensure, Context, Error, Result};
use raw_cstr::AsRawCstr;
use simics::{
    api::{
        get_attribute, get_class, get_class_interface, get_interface, read_phys_memory,
        sys::instruction_handle_t, write_byte, Access, AttrValueType, ConfObject,
        CpuCachedInstructionInterface, CpuInstructionQueryInterface,
        CpuInstrumentationSubscribeInterface, CycleInterface, IntRegisterInterface,
        ProcessorInfoV2Interface,
    },
    read_byte,
};
use std::{fmt::Debug, str::FromStr};

pub mod aarch64;
pub mod arm;
pub mod risc_v;
pub mod x86;
pub mod x86_64;

/// The smallest page size of the supported architectures: a translation holds at least up to
/// the next multiple of it.
const PAGE_SIZE: u64 = 0x1000;

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

/// Check that `cpu` supports selective cached-instruction instrumentation and return its
/// cached-instruction interface. Individual methods are checked, not just interface presence.
pub(crate) fn cached_instruction_interface(
    cpu: *mut ConfObject,
    cmplog: bool,
) -> Result<CpuCachedInstructionInterface> {
    let class = get_class(String::try_from(get_attribute(cpu, "classname")?)?)?;
    let cached = get_class_interface::<CpuCachedInstructionInterface>(class)
        .context("selective instrumentation requires cpu_cached_instruction")?;
    let cached = unsafe { cached.as_ref() }
        .ok_or_else(|| anyhow!("CPU has no cpu_cached_instruction interface"))?;
    ensure!(
        cached.register_instruction_after_cb.is_some()
            && (!cmplog || cached.register_instruction_before_cb.is_some()),
        "CPU lacks required cached instruction execution callbacks"
    );
    let subscribe = get_class_interface::<CpuInstrumentationSubscribeInterface>(class)?;
    ensure!(
        unsafe { subscribe.as_ref() }
            .is_some_and(|interface| interface.register_cached_instruction_cb.is_some()),
        "CPU lacks register_cached_instruction_cb"
    );
    Ok(get_interface(cpu)?)
}

/// Selective cached-instruction instrumentation, implemented for x86-64 and i386.
impl Architecture {
    pub(crate) fn enable_cached_instrumentation(&mut self, cmplog: bool) -> Result<()> {
        match self {
            Architecture::X86_64(arch) => arch.enable_cached_instrumentation(cmplog),
            Architecture::I386(arch) => arch.enable_cached_instrumentation(cmplog),
            _ => bail!("only supported on x86-64 and i386"),
        }
    }

    pub(crate) fn decode_cached_instruction(
        &mut self,
        handle: *mut instruction_handle_t,
    ) -> Result<DecodedInstruction> {
        match self {
            Architecture::X86_64(arch) => arch.decode_cached_instruction(handle),
            Architecture::I386(arch) => arch.decode_cached_instruction(handle),
            _ => bail!("only supported on x86-64 and i386"),
        }
    }

    pub(crate) fn cached_instruction(&mut self) -> Option<&mut CpuCachedInstructionInterface> {
        match self {
            Architecture::X86_64(arch) => arch.cached_instruction.as_mut(),
            Architecture::I386(arch) => arch.cached_instruction.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn trace_decoded_cmp(
        &mut self,
        comparison: &DecodedComparison,
    ) -> Result<TraceEntry> {
        match self {
            Architecture::X86_64(arch) => arch.trace_decoded_cmp(comparison),
            Architecture::I386(arch) => arch.trace_decoded_cmp(comparison),
            _ => bail!("only supported on x86-64 and i386"),
        }
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
    #[allow(dead_code)]
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

    /// Translate `size` bytes at `logical_address` to `(physical address, length)` pieces, one
    /// page at a time: consecutive virtual pages need not be physically contiguous.
    fn translate_range(&mut self, logical_address: u64, size: u64) -> Result<Vec<(u64, u64)>> {
        let mut pieces: Vec<(u64, u64)> = Vec::new();
        let mut offset = 0;
        while offset < size {
            let logical = logical_address + offset;
            let block = self
                .processor_info_v2()
                .logical_to_physical(logical, Access::Sim_Access_Read)?;
            ensure!(
                block.valid != 0,
                "Invalid linear address {logical:#x} in the {size:#x} bytes at {logical_address:#x}"
            );
            let length = (PAGE_SIZE - logical % PAGE_SIZE).min(size - offset);
            match pieces.last_mut() {
                Some((address, piece_length)) if *address + *piece_length == block.address => {
                    *piece_length += length
                }
                _ => pieces.push((block.address, length)),
            }
            offset += length;
        }
        Ok(pieces)
    }

    /// Translate the address of the size variable, which is read and written as one block
    fn translate_size_ptr(&mut self, logical_address: u64) -> Result<StartPhysicalAddress> {
        let width = self.size_width()? as u64;
        match self.translate_range(logical_address, width)?[..] {
            [(address, _)] if address != logical_address => {
                Ok(StartPhysicalAddress::WasVirtual(address))
            }
            [(address, _)] => Ok(StartPhysicalAddress::WasPhysical(address)),
            _ => bail!(
                "Size variable at {logical_address:#x} crosses a page boundary and is not physically contiguous"
            ),
        }
    }

    /// Build the start information for a testcase buffer of `size.maximum_size()` bytes at
    /// `address`, reading its initial contents.
    fn start_info(&mut self, address: &ManualStartAddress, size: StartSize) -> Result<StartInfo> {
        let maximum_size = size.maximum_size() as u64;
        let pieces = match *address {
            ManualStartAddress::Virtual(logical) => self.translate_range(logical, maximum_size)?,
            ManualStartAddress::Physical(physical) => vec![(physical, maximum_size)],
        };
        let physical_memory = self.processor_info_v2().get_physical_memory()?;
        let contents = pieces
            .iter()
            .flat_map(|&(address, length)| address..address + length)
            .map(|address| {
                read_byte(physical_memory, address)
                    .map_err(|e| anyhow!("Failed to read byte at {address:#x}: {e}"))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(StartInfo::builder()
            .pieces(pieces)
            .contents(contents)
            .size(size)
            .build())
    }

    /// Read the value of the register named `name`
    fn read_register(&mut self, name: &str) -> Result<u64> {
        let number = self.int_register().get_number(name.as_raw_cstr()?)?;
        Ok(self.int_register().read(number)?)
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_ptr: A pointer to a pointer-sized variable containing the size of the testcase
    fn get_magic_start_buffer_ptr_size_ptr(&mut self) -> Result<StartInfo> {
        let buffer = self.read_register(Self::ARGUMENT_REGISTER_0)?;
        let size_ptr = self.read_register(Self::ARGUMENT_REGISTER_1)?;
        let address = self.translate_size_ptr(size_ptr)?;
        let size_size = self.size_width()?;
        let size = read_phys_memory(self.cpu(), address.physical_address(), size_size)?;

        self.start_info(
            &ManualStartAddress::Virtual(buffer),
            StartSize::SizePtr {
                address,
                maximum_size: size as usize,
            },
        )
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_val: The maximum size of the testcase
    fn get_magic_start_buffer_ptr_size_val(&mut self) -> Result<StartInfo> {
        let buffer = self.read_register(Self::ARGUMENT_REGISTER_0)?;
        let size_val = self.read_register(Self::ARGUMENT_REGISTER_1)?;

        self.start_info(
            &ManualStartAddress::Virtual(buffer),
            StartSize::MaxSize(size_val as usize),
        )
    }

    /// Get the magic start information from the harness which takes the arguments:
    ///
    /// - buffer: The address of the buffer containing the testcase
    /// - size_ptr: A pointer to a pointer-sized variable to which the size is written
    /// - size_val: The maximum size of the testcase
    fn get_magic_start_buffer_ptr_size_ptr_val(&mut self) -> Result<StartInfo> {
        let buffer = self.read_register(Self::ARGUMENT_REGISTER_0)?;
        let size_ptr = self.read_register(Self::ARGUMENT_REGISTER_1)?;
        let size_val = self.read_register(Self::ARGUMENT_REGISTER_2)?;
        let address = self.translate_size_ptr(size_ptr)?;

        self.start_info(
            &ManualStartAddress::Virtual(buffer),
            StartSize::SizePtrAndMaxSize {
                address,
                maximum_size: size_val as usize,
            },
        )
    }

    /// Get the start information for the testcase buffer and size the user gave to the manual
    /// start functionality
    fn get_manual_start_info(&mut self, info: &ManualStartInfo) -> Result<StartInfo> {
        let mut size_ptr = |address: &ManualStartAddress| match *address {
            ManualStartAddress::Virtual(v) => self.translate_size_ptr(v),
            ManualStartAddress::Physical(p) => Ok(StartPhysicalAddress::WasPhysical(p)),
        };

        let size = match &info.size {
            crate::ManualStartSize::SizePtr { address } => {
                let address = size_ptr(address)?;
                let size_size = self.size_width()?;
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
            } => StartSize::SizePtrAndMaxSize {
                address: size_ptr(address)?,
                maximum_size: *maximum_size,
            },
        };

        self.start_info(&info.address, size)
    }

    /// Width in bytes of the size variable the harness passes by pointer. The maximum size is
    /// read with this width and the testcase size is written back with it.
    fn size_width(&mut self) -> Result<i32> {
        if let Some(width) = Self::POINTER_WIDTH_OVERRIDE {
            Ok(width)
        } else {
            Ok(self.processor_info_v2().get_logical_address_width()? / u8::BITS as i32)
        }
    }

    fn write_start(&mut self, testcase: &[u8], info: &StartInfo) -> Result<()> {
        // NOTE: The size is written back with the same width it was read with
        let size_width = self.size_width()? as usize;

        let physical_memory = self.processor_info_v2().get_physical_memory()?;

        let testcase = &testcase[..testcase.len().min(info.size.maximum_size())];

        let mut remaining = testcase;
        for &(address, length) in &info.pieces {
            let (piece, rest) = remaining.split_at(remaining.len().min(length as usize));
            for (i, c) in piece.iter().enumerate() {
                write_byte(physical_memory, address + i as u64, *c)?;
            }
            remaining = rest;
        }

        if let Some(size_address) = info.size.physical_address().map(|s| s.physical_address()) {
            testcase
                .len()
                .to_le_bytes()
                .iter()
                .take(size_width)
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
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer_ptr_size_ptr_val(),
            Architecture::I386(i386) => i386.get_magic_start_buffer_ptr_size_ptr_val(),
            Architecture::Riscv(riscv) => riscv.get_magic_start_buffer_ptr_size_ptr_val(),
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

    fn size_width(&mut self) -> Result<i32> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.size_width(),
            Architecture::I386(i386) => i386.size_width(),
            Architecture::Riscv(riscv) => riscv.size_width(),
            Architecture::Arm(arm) => arm.size_width(),
            Architecture::Aarch64(aarch64) => aarch64.size_width(),
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
