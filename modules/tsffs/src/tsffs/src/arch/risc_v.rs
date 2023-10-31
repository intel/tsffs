// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use raw_cstr::AsRawCstr;
use simics::api::{
    get_interface, read_phys_memory, write_phys_memory, Access, ConfObject, GenericAddress,
    IntRegisterInterface, ProcessorInfoV2Interface,
};
use std::ffi::CStr;

use crate::driver::{StartBuffer, StartSize};

use super::ArchitectureOperations;

/// The default register the fuzzer expects to contain a pointer to an area to write
/// each testcase into when using an in-target harness
pub const DEFAULT_TESTCASE_AREA_REGISTER_NAME: &str = "x10";
/// The default register the fuzzer expects to contain a pointer to a variable,
/// initially containing the maximum size of the area pointed to by
/// `DEFAULT_TESTCASE_AREA_REGISTER_NAME`, which will be written each fuzzer execution
/// to contain the actual size of the current testcase.
pub const DEFAULT_TESTCASE_SIZE_REGISTER_NAME: &str = "x11";

pub struct RISCVArchitectureOperations {
    cpu: *mut ConfObject,
    int_register: IntRegisterInterface,
    processor_info_v2: ProcessorInfoV2Interface,
}

impl ArchitectureOperations for RISCVArchitectureOperations {
    fn new(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        if arch == "risc-v" {
            Ok(Self {
                cpu,
                int_register: get_interface(cpu)?,
                processor_info_v2,
            })
        } else {
            bail!("Architecture {} is not risc-v", arch);
        }
    }

    fn get_magic_start_buffer(&mut self) -> Result<StartBuffer> {
        let number = self
            .int_register
            .get_number(DEFAULT_TESTCASE_AREA_REGISTER_NAME.as_raw_cstr()?)?;

        let logical_address = self.int_register.read(number)?;

        let physical_address_block = self
            .processor_info_v2
            // NOTE: Do we need to support segmented memory via logical_to_physical?
            .logical_to_physical(logical_address, Access::Sim_Access_Read)?;

        // NOTE: -1 signals no valid mapping, but this is equivalent to u64::MAX
        if physical_address_block.valid == 0 {
            bail!("Invalid linear address found in magic start buffer register {number}: {logical_address:#x}");
        } else {
            Ok(StartBuffer {
                physical_address: physical_address_block.address,
                virt: physical_address_block.address != logical_address,
            })
        }
    }

    fn get_magic_start_size(&mut self) -> Result<StartSize> {
        let number = self
            .int_register
            .get_number(DEFAULT_TESTCASE_SIZE_REGISTER_NAME.as_raw_cstr()?)?;
        let logical_address = self.int_register.read(number)?;
        let physical_address_block = self
            .processor_info_v2
            // NOTE: Do we need to support segmented memory via logical_to_physical?
            .logical_to_physical(logical_address, Access::Sim_Access_Read)?;

        // NOTE: -1 signals no valid mapping, but this is equivalent to u64::MAX
        if physical_address_block.valid == 0 {
            bail!("Invalid linear address found in magic start buffer register {number}: {logical_address:#x}");
        }

        let size_size = self.processor_info_v2.get_logical_address_width()? / u8::BITS as i32;
        let size = read_phys_memory(self.cpu, physical_address_block.address, size_size)?;

        Ok(StartSize {
            physical_address: Some(physical_address_block.address),
            initial_size: size,
            virt: physical_address_block.address != logical_address,
        })
    }

    fn write_start(
        &mut self,
        testcase: &[u8],
        buffer: &StartBuffer,
        size: &StartSize,
    ) -> Result<()> {
        let mut testcase = testcase.to_vec();
        testcase.truncate(size.initial_size as usize);

        testcase.chunks(8).try_for_each(|c| {
            println!("Writing {:#x} <- {:?}", buffer.physical_address, c);
            write_phys_memory(self.cpu, buffer.physical_address, c)
        })?;

        let value = testcase
            .len()
            .to_le_bytes()
            .iter()
            .take(self.processor_info_v2.get_logical_address_width()? as usize)
            .cloned()
            .collect::<Vec<_>>();

        if let Some(ref physical_address) = size.physical_address {
            println!(
                "Writing size {:#x} <- {:?}",
                *physical_address,
                value.as_slice()
            );
            write_phys_memory(self.cpu, *physical_address, value.as_slice())?;
        }

        Ok(())
    }

    fn get_start_size(&mut self, size_address: GenericAddress, virt: bool) -> Result<StartSize> {
        let original_size_address = size_address;
        let size_address = if virt {
            let physical_address_block = self
                .processor_info_v2
                // NOTE: Do we need to support segmented memory via logical_to_physical?
                .logical_to_physical(size_address, Access::Sim_Access_Read)?;

            if physical_address_block.valid == 0 {
                bail!("Invalid linear address given for start buffer : {size_address:#x}");
            }

            physical_address_block.address
        } else {
            size_address
        };
        let size_size = self.processor_info_v2.get_logical_address_width()? / u8::BITS as i32;
        let size = read_phys_memory(self.cpu, size_address, size_size)?;

        Ok(StartSize {
            physical_address: Some(size_address),
            initial_size: size,
            virt: original_size_address != size_address,
        })
    }
}
