// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture specific data and definitions

use self::{
    risc_v::RISCVArchitectureOperations, x86::X86ArchitectureOperations,
    x86_64::X86_64ArchitectureOperations,
};
use crate::driver::{StartBuffer, StartSize};
use anyhow::{bail, Result};
use simics::api::{ConfObject, GenericAddress};
use std::fmt::Debug;

pub mod arc;
pub mod arm;
pub mod arm_thumb2;
pub mod armv8;
pub mod risc_v;
pub mod x86;
pub mod x86_64;

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
    fn new(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized;
    /// Returns the address and whether the address is virtual for the testcase buffer used by
    /// the magic start functionality
    fn get_magic_start_buffer(&mut self) -> Result<StartBuffer>;
    /// Returns the memory pointed to by the magic start functionality containing the maximum
    /// size of an input testcase
    fn get_magic_start_size(&mut self) -> Result<StartSize>;
    /// Returns the initial start size for non-magic instructions by reading it from a given
    /// (possibly virtual) address
    fn get_start_size(&mut self, size_address: GenericAddress, virt: bool) -> Result<StartSize>;
    /// Writes the buffer with a testcase of a certain size
    fn write_start(
        &mut self,
        testcase: &[u8],
        buffer: &StartBuffer,
        size: &StartSize,
    ) -> Result<()>;
}

impl ArchitectureOperations for Architecture {
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

    fn get_start_size(&mut self, size_address: GenericAddress, virt: bool) -> Result<StartSize> {
        match self {
            Architecture::X86_64(x86_64) => x86_64.get_start_size(size_address, virt),
            Architecture::I386(i386) => i386.get_start_size(size_address, virt),
            Architecture::RISCV(riscv) => riscv.get_start_size(size_address, virt),
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
}
