use std::mem::MaybeUninit;

use anyhow::{anyhow, bail, Result};
use simics::{get_interface, read_byte, Access, ConfObject, ProcessorInfoV2Interface};

use super::paging::{
    DIR_TABLE_BASE, PAGE_1GB_SHIFT, PAGE_2MB_SHIFT, PAGE_4KB_SHIFT, PDE, PDPTE, PDPTE_LARGE, PML4E,
    PTE, VIRTUAL_ADDRESS,
};

pub fn read_virtual<T>(processor: *mut ConfObject, virtual_address: u64) -> Result<T>
where
    T: Sized,
{
    let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(processor)?;

    let size = std::mem::size_of::<T>();

    let mut t = MaybeUninit::<T>::uninit();

    let memory = processor_info_v2.get_physical_memory()?;

    let contents = (0..size)
        .map(|i| {
            processor_info_v2
                .logical_to_physical(virtual_address + i as u64, Access::Sim_Access_Read)
                .and_then(|b| read_byte(memory, b.address))
                .map_err(|e| anyhow!("Failed to read memory: {}", e))
        })
        .collect::<Result<Vec<u8>>>()?;

    unsafe {
        std::ptr::copy_nonoverlapping(contents.as_ptr(), t.as_mut_ptr() as *mut u8, size);
        Ok(t.assume_init())
    }
}

pub fn read_physical<T>(processor: *mut ConfObject, physical_address: u64) -> Result<T> {
    let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(processor)?;

    let size = std::mem::size_of::<T>();

    let mut t = MaybeUninit::<T>::uninit();

    let memory = processor_info_v2.get_physical_memory()?;

    let contents = (0..size)
        .map(|i| {
            read_byte(memory, physical_address + i as u64)
                .map_err(|e| anyhow!("Failed to read memory: {}", e))
        })
        .collect::<Result<Vec<u8>>>()?;

    unsafe {
        std::ptr::copy_nonoverlapping(contents.as_ptr(), t.as_mut_ptr() as *mut u8, size);
        Ok(t.assume_init())
    }
}

pub fn read_virtual_dtb<T>(
    processor: *mut ConfObject,
    directory_table_base: u64,
    virtual_address: u64,
) -> Result<T> {
    let physical_address = virtual_to_physical(processor, directory_table_base, virtual_address)?;
    read_physical(processor, physical_address)
}

pub fn virtual_to_physical(
    processor: *mut ConfObject,
    directory_table_base: u64,
    virtual_address: u64,
    // build: u32,
) -> Result<u64> {
    let virtual_address = VIRTUAL_ADDRESS {
        All: virtual_address,
    };
    let dir_table_base = DIR_TABLE_BASE {
        All: directory_table_base,
    };
    let pml4e = read_physical::<PML4E>(
        processor,
        (unsafe { dir_table_base.Bits }.PhysicalAddress() << PAGE_4KB_SHIFT as u64)
            + (unsafe { virtual_address.Bits }.Pml4Index() * 8),
    )?;

    if unsafe { pml4e.Bits }.Present() == 0 {
        bail!("PML4E not present");
    }

    let pdpte = read_physical::<PDPTE>(
        processor,
        (unsafe { pml4e.Bits }.PhysicalAddress() << PAGE_4KB_SHIFT)
            + (unsafe { virtual_address.Bits }.PdptIndex() * 8),
    )?;

    if unsafe { pdpte.Bits }.Present() == 0 {
        bail!("PDPTE not present");
    }

    if (unsafe { pdpte.All } >> 7) & 1 != 0 {
        let pdpte_large = PDPTE_LARGE {
            All: unsafe { pdpte.All },
        };
        return Ok(
            ((unsafe { pdpte_large.Bits }.PhysicalAddress()) << PAGE_1GB_SHIFT)
                + (unsafe { virtual_address.All } & (!(u64::MAX << PAGE_1GB_SHIFT))),
        );
    }

    let pde = read_physical::<PDE>(
        processor,
        (unsafe { pdpte.Bits }.PhysicalAddress() << PAGE_4KB_SHIFT)
            + (unsafe { virtual_address.Bits }.PdIndex() * 8),
    )?;

    if unsafe { pde.Bits }.Present() == 0 {
        bail!("PDE not present");
    }

    if (unsafe { pde.All } >> 7) & 1 != 0 {
        let pde_large = PDPTE_LARGE {
            All: unsafe { pde.All },
        };
        return Ok(
            ((unsafe { pde_large.Bits }.PhysicalAddress()) << PAGE_2MB_SHIFT)
                + (unsafe { virtual_address.All } & (!(u64::MAX << PAGE_2MB_SHIFT))),
        );
    }

    let pte = read_physical::<PTE>(
        processor,
        (unsafe { pde.Bits }.PhysicalAddress() << PAGE_4KB_SHIFT)
            + (unsafe { virtual_address.Bits }.PtIndex() * 8),
    )?;

    if unsafe { pte.Bits }.Present() == 0 {
        bail!("PTE not present");
    }

    Ok((unsafe { pte.Bits }.PhysicalAddress() << PAGE_4KB_SHIFT)
        + unsafe { virtual_address.Bits }.PageIndex())
}

pub fn read_unicode_string(
    processor: *mut ConfObject,
    length: usize,
    buffer: *const u16,
) -> Result<String> {
    let mut string = Vec::new();
    let mut address = buffer as u64;

    for _ in 0..length {
        let character = read_virtual::<u16>(processor, address)?;

        if character == 0 {
            break;
        }

        string.push(character);
        address += 2;
    }

    String::from_utf16(&string).map_err(|e| anyhow!("Failed to convert string: {}", e))
}

pub fn read_unicode_string_dtb(
    processor: *mut ConfObject,
    length: usize,
    buffer: *const u16,
    directory_table_base: u64,
) -> Result<String> {
    let mut string = Vec::new();
    let mut address = buffer as u64;

    for _ in 0..length {
        let character = read_virtual_dtb::<u16>(processor, directory_table_base, address)?;

        if character == 0 {
            break;
        }

        string.push(character);
        address += 2;
    }

    String::from_utf16(&string).map_err(|e| anyhow!("Failed to convert string: {}", e))
}

pub fn read_nul_terminated_string(processor: *mut ConfObject, address: u64) -> Result<String> {
    let mut string = String::new();
    let mut address = address;

    loop {
        let character = read_virtual::<u8>(processor, address)?;

        if character == 0 {
            break;
        }

        string.push(character as char);
        address += 1;
    }

    Ok(string)
}

pub fn read_nul_terminated_string_dtb(
    processor: *mut ConfObject,
    address: u64,
    directory_table_base: u64,
) -> Result<String> {
    let mut string = String::new();
    let mut address = address;

    loop {
        let character = read_virtual_dtb::<u8>(processor, directory_table_base, address)?;

        if character == 0 {
            break;
        }

        string.push(character as char);
        address += 1;
    }

    Ok(string)
}
