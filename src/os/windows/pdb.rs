use anyhow::{ensure, Result};
use simics::ConfObject;

use crate::os::windows::util::{
    read_nul_terminated_string, read_nul_terminated_string_dtb, read_virtual, read_virtual_dtb,
};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Guid {
    pub data0: u32,
    pub data1: u16,
    pub data2: u16,
    pub data3: u64,
}

#[derive(Debug, Clone)]
pub struct CvInfoPdb70 {
    pub cv_signature: u32,
    pub signature: Guid,
    pub age: u32,
    pub file_name: String,
}

impl CvInfoPdb70 {
    pub fn new(processor: *mut ConfObject, address: u64) -> Result<Self> {
        let cv_signature = read_virtual::<u32>(processor, address)?;
        let signature =
            read_virtual::<Guid>(processor, address + std::mem::size_of::<u32>() as u64)?;
        let age = read_virtual::<u32>(
            processor,
            address + std::mem::size_of::<u32>() as u64 + std::mem::size_of::<Guid>() as u64,
        )?;
        let file_name = read_nul_terminated_string(
            processor,
            address
                + std::mem::size_of::<u32>() as u64
                + std::mem::size_of::<Guid>() as u64
                + std::mem::size_of::<u32>() as u64,
        )?;

        ensure!(cv_signature == 0x53445352, "Invalid CV signature");

        Ok(Self {
            cv_signature,
            signature,
            age,
            file_name,
        })
    }

    pub fn new_dtb(
        processor: *mut ConfObject,
        directory_table_base: u64,
        address: u64,
    ) -> Result<Self> {
        let cv_signature = read_virtual_dtb::<u32>(processor, directory_table_base, address)?;
        let signature = read_virtual_dtb::<Guid>(
            processor,
            directory_table_base,
            address + std::mem::size_of::<u32>() as u64,
        )?;
        let age = read_virtual_dtb::<u32>(
            processor,
            directory_table_base,
            address + std::mem::size_of::<u32>() as u64 + std::mem::size_of::<Guid>() as u64,
        )?;
        let file_name = read_nul_terminated_string_dtb(
            processor,
            directory_table_base,
            address
                + std::mem::size_of::<u32>() as u64
                + std::mem::size_of::<Guid>() as u64
                + std::mem::size_of::<u32>() as u64,
        )?;

        ensure!(cv_signature == 0x53445352, "Invalid CV signature");

        Ok(Self {
            cv_signature,
            signature,
            age,
            file_name,
        })
    }

    pub fn guid(&self) -> String {
        // Reverse the order of data3
        let data3 = u64::from_be_bytes(self.signature.data3.to_le_bytes());

        format!(
            "{:08X}{:04X}{:04X}{:016X}{:01X}",
            self.signature.data0, self.signature.data1, self.signature.data2, data3, self.age
        )
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: Option<String>,
    pub offset: Option<usize>,
    pub rva: usize,
    pub size: usize,
}

impl From<goblin::pe::export::Export<'_>> for Export {
    fn from(export: goblin::pe::export::Export) -> Self {
        Self {
            name: export.name.map(|s| s.to_string()),
            offset: export.offset,
            rva: export.rva,
            size: export.size,
        }
    }
}

impl From<&goblin::pe::export::Export<'_>> for Export {
    fn from(export: &goblin::pe::export::Export) -> Self {
        Self {
            name: export.name.map(|s| s.to_string()),
            offset: export.offset,
            rva: export.rva,
            size: export.size,
        }
    }
}
