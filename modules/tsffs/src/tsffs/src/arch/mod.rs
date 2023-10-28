// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture specific data and definitions

use anyhow::{anyhow, bail, Result};
use simics::{
    api::{get_interface, ConfObject, IntRegisterInterface, ProcessorInfoV2Interface},
    debug,
};
use std::{ffi::CStr, fmt::Debug};

use crate::driver::{MagicStartBuffer, MagicStartSize};

use self::x86_64::X86_64ArchitectureOperations;

pub mod arc;
pub mod arm;
pub mod arm_thumb2;
pub mod armv8;
pub mod risc_v;
pub mod x86;
pub mod x86_64;

pub enum Architecture {
    X86_64(X86_64ArchitectureOperations),
    I386,
}

impl Debug for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Architecture::X86_64(_) => "x86-64",
                Architecture::I386 => "i386",
            }
        )
    }
}
/// Each architecture must provide a struct that performs architecture-specific operations
pub trait ArchitectureOperations {
    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized;
    /// Returns the address and whether the address is virtual for the testcase buffer used by
    /// the magic start functionality
    fn get_magic_start_buffer(&mut self) -> Result<MagicStartBuffer>;
    /// Returns the memory pointed to by the magic start functionality containing the maximum
    /// size of an input testcase
    fn get_magic_start_size(&mut self) -> Result<MagicStartSize>;
    /// Writes the magic buffer with a testcase of a certain size
    fn write_magic_start(
        &mut self,
        testcase: &[u8],
        buffer: &MagicStartBuffer,
        size: &MagicStartSize,
    ) -> Result<()>;
}

impl Architecture {
    /// Determine the architecture of a CPU
    ///
    /// This function cannot simply use [`simics::api::ProcessorInfoV2Interface::architecture`]
    /// to determine the architecture, because some processors do not advertise their actual
    /// architecture. Note from the docs:
    ///
    /// "The processor architecture is returned by calling the architecture function.
    /// The architecture should be one of arm, mips32, mips64, ppc32, ppc64, sparc-v8,
    /// sparc-v9, x86, x86-64, or something else if none of the listed is a good match."
    ///
    pub fn get(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        if arch == "x86-64" {
            // Check if the arch is actually x86-64, some x86-64 processors are actually
            // i386 under the hood
            let mut int_register: IntRegisterInterface = get_interface(cpu)?;
            let regs: Vec<u32> = int_register.all_registers()?.try_into()?;
            let reg_names: Vec<String> = regs
                .iter()
                .map(|r| {
                    int_register
                        .get_name(*r as i32)
                        .map_err(|e| anyhow!("Failed to get register name: {e}"))
                        .and_then(|n| {
                            unsafe { CStr::from_ptr(n) }
                                .to_str()
                                .map(|s| s.to_string())
                                .map_err(|e| anyhow!("Failed to convert string: {e}"))
                        })
                })
                .collect::<Result<Vec<_>>>()?;

            debug!("Got register names: {reg_names:?}");

            if reg_names.iter().any(|n| {
                [
                    "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "rip", "rsp", "rbp", "r8", "r9",
                    "r10", "r11", "r12", "r14", "r15",
                ]
                .contains(&n.to_ascii_lowercase().as_str())
            }) {
                Ok(Self::X86_64(X86_64ArchitectureOperations::new(cpu)?))
            } else if reg_names.iter().all(|n| {
                ![
                    "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "rip", "rsp", "rbp", "r8", "r9",
                    "r10", "r11", "r12", "r14", "r15",
                ]
                .contains(&n.to_ascii_lowercase().as_str())
            }) {
                Ok(Self::I386)
            } else {
                unreachable!("Register set must either contain a 64-bit register or no registers may be 64-bit");
            }
        } else if ["i386", "i486", "i586", "i686", "x86", "ia-32"]
            .contains(&arch.to_ascii_lowercase().as_str())
        {
            // No i386 processor will actually be x86-64 under the hood
            Ok(Self::I386)
        } else {
            bail!("Unsupported architecture {arch}");
        }
    }
}

impl ArchitectureOperations for Architecture {
    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        Self::get(cpu)
    }

    fn get_magic_start_buffer(&mut self) -> Result<MagicStartBuffer> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_buffer(),
            Architecture::I386 => todo!(),
        }
    }

    fn get_magic_start_size(&mut self) -> Result<MagicStartSize> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_magic_start_size(),
            Architecture::I386 => todo!(),
        }
    }

    fn write_magic_start(
        &mut self,
        testcase: &[u8],
        buffer: &MagicStartBuffer,
        size: &MagicStartSize,
    ) -> Result<()> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.write_magic_start(testcase, buffer, size),
            Architecture::I386 => todo!(),
        }
    }
}
