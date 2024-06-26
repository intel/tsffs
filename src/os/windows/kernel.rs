use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use pdb::{FallibleIterator, SymbolData};
use simics::{debug, get_attribute, get_object, ConfObject};
use vergilius::bindings::*;
use windows::Win32::System::{
    Diagnostics::Debug::{IMAGE_DIRECTORY_ENTRY_EXPORT, IMAGE_NT_HEADERS64},
    Kernel::LIST_ENTRY,
    SystemServices::{
        IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_EXPORT_DIRECTORY, IMAGE_NT_SIGNATURE,
    },
};

use crate::os::windows::{
    debug_info::DebugInfo,
    idt::IdtEntry64,
    util::{read_nul_terminated_string, read_unicode_string, read_virtual},
};

use super::{
    debug_info::{Module, Process},
    structs::{WindowsEProcess, WindowsKThread, WindowsKpcr, WindowsKprcb},
};

const KUSER_SHARED_DATA_ADDRESS_X86_64: u64 = 0xFFFFF78000000000;

pub fn page_is_kernel(processor: *mut ConfObject, address: u64) -> Result<bool> {
    const OPTIONAL_HEADER_SIGNATURE_PE32: u16 = 0x10b;
    const OPTIONAL_HEADER_SIGNATURE_PE32_PLUS: u16 = 0x20b;

    let dos_header = read_virtual::<IMAGE_DOS_HEADER>(processor, address)?;

    if dos_header.e_magic != IMAGE_DOS_SIGNATURE {
        return Ok(false);
    }

    let nt_header =
        read_virtual::<IMAGE_NT_HEADERS64>(processor, address + dos_header.e_lfanew as u64)?;

    if nt_header.Signature != IMAGE_NT_SIGNATURE {
        debug!(
            "NT Signature {:#x} != {:#x}",
            nt_header.Signature, IMAGE_NT_SIGNATURE
        );
        return Ok(false);
    }

    debug!(
        "Found NT signature at {:#x}",
        address + dos_header.e_lfanew as u64
    );

    if nt_header.FileHeader.SizeOfOptionalHeader == 0 {
        debug!(get_object("tsffs")?, "Optional header size was 0");
        return Ok(false);
    }

    if ![
        OPTIONAL_HEADER_SIGNATURE_PE32,
        OPTIONAL_HEADER_SIGNATURE_PE32_PLUS,
    ]
    .contains(&nt_header.OptionalHeader.Magic.0)
    {
        debug!(
            "Optional header magic {:#x} unrecognized",
            nt_header.OptionalHeader.Magic.0
        );
        return Ok(false);
    }

    let image_size = nt_header.OptionalHeader.SizeOfImage as u64;

    debug!(get_object("tsffs")?, "Image size is {:#x}", image_size);

    let export_header_offset = nt_header.OptionalHeader.DataDirectory
        [IMAGE_DIRECTORY_ENTRY_EXPORT.0 as usize]
        .VirtualAddress as u64;
    let export_header_size =
        nt_header.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT.0 as usize].Size as u64;

    if export_header_offset == 0 || export_header_offset >= image_size {
        debug!(
            "Export header offset {:#x} invalid for image sized {:#x}",
            export_header_offset, image_size
        );
        return Ok(false);
    }

    if export_header_size == 0 || export_header_offset + export_header_size > image_size {
        debug!(
            "Export header size {:#x} and offset {:#x} invalid for image sized {:#x}",
            export_header_size, export_header_offset, image_size
        );
        return Ok(false);
    }

    debug!(
        "Export header offset {:#x} size {:#x}",
        export_header_offset, export_header_size
    );

    let export_directory =
        read_virtual::<IMAGE_EXPORT_DIRECTORY>(processor, address + export_header_offset)?;

    let name = read_nul_terminated_string(processor, address + export_directory.Name as u64)?;

    debug!(get_object("tsffs")?, "Read image name {}", name);

    if name == "ntoskrnl.exe" {
        return Ok(true);
    }

    Ok(false)
}

pub fn find_kernel(processor: *mut ConfObject, start: u64, step: u64) -> Result<u64> {
    let mut scan_address = start & !(step - 1);
    let stop_address = start & !(0x1000000000000 - 1);

    debug!(
        "Scanning for kernel from {:#x}->{:#x}",
        stop_address, scan_address
    );

    while scan_address >= stop_address {
        if page_is_kernel(processor, scan_address)? {
            return Ok(scan_address);
        }

        scan_address -= step;
    }

    bail!("Kernel not found");
}

pub fn find_kernel_with_idt(processor: *mut ConfObject, build: u32) -> Result<u64> {
    let sim_idtr_base: u64 = get_attribute(processor, "idtr_base")?.try_into()?;

    for i in 0..256 {
        // try each idtr entry
        let idtr_entry0 = read_virtual::<IdtEntry64>(
            processor,
            sim_idtr_base + (i * std::mem::size_of::<IdtEntry64>() as u64),
        )?;
        if !idtr_entry0.present() {
            debug!(get_object("tsffs")?, "Entry {} not present, skipping", i);
            continue;
        }
        let idtr_entry0_offset = idtr_entry0.offset();
        debug!(
            get_object("tsffs")?,
            "Got valid IDT entry with offset {:#x}", idtr_entry0_offset
        );
        return find_kernel(
            processor,
            idtr_entry0_offset,
            if build >= 19000 { 0x200000 } else { 0x1000 },
        );
    }

    bail!("Kernel not found");
}

#[derive(Debug)]
pub struct KernelInfo {
    pub base: u64,
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub debug_info: DebugInfo<'static>,
}

impl KernelInfo {
    pub fn new<P>(
        processor: *mut ConfObject,
        name: &str,
        base: u64,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let debug_info = DebugInfo::new(
            processor,
            name,
            base,
            download_directory,
            not_found_full_name_cache,
            user_debug_info,
        )?;

        let kuser_shared_data = read_virtual::<windows_10_0_22631_2428_x64::_KUSER_SHARED_DATA>(
            processor,
            KUSER_SHARED_DATA_ADDRESS_X86_64,
        )?;

        Ok(Self {
            base,
            major: kuser_shared_data.NtMajorVersion,
            minor: kuser_shared_data.NtMinorVersion,
            build: kuser_shared_data.NtBuildNumber,
            debug_info,
        })
    }

    fn find_ps_loaded_module_list_address(&mut self) -> Result<u64> {
        // PsLoadedModuleList is either a public(publicsymbol) in the PDB file, or if it is not
        // in the PDB file, we can find it via the export table in the PE file.
        let pdb_symbols = self.debug_info.pdb.global_symbols()?;
        let pdb_address_map = self.debug_info.pdb.address_map()?;
        if let Ok(Some(ps_loaded_module_list_symbol)) =
            pdb_symbols.iter().find_map(|symbol| match symbol.parse() {
                Ok(symbol) => {
                    if let SymbolData::Public(public_symbol) = symbol {
                        if public_symbol.name.to_string() == "PsLoadedModuleList" {
                            Ok(Some(
                                public_symbol
                                    .offset
                                    .to_rva(&pdb_address_map)
                                    .ok_or_else(|| pdb::Error::AddressMapNotFound)?
                                    .0 as u64
                                    + self.base,
                            ))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                }
                Err(e) => Err(e),
            })
        {
            Ok(ps_loaded_module_list_symbol)
        } else {
            self.debug_info
                .exports()?
                .iter()
                .find(|export| {
                    export
                        .name
                        .as_ref()
                        .is_some_and(|name| name == "PsLoadedModuleList")
                })
                .map(|export| export.rva as u64 + self.base)
                .ok_or_else(|| anyhow!("PsLoadedModuleList not found"))
        }
    }

    pub fn loaded_module_list<P>(
        &mut self,
        processor: *mut ConfObject,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Vec<Module>>
    where
        P: AsRef<Path>,
    {
        let list_address = self.find_ps_loaded_module_list_address()?;
        debug!(
            get_object("tsffs")?,
            "PsLoadedModuleList: {:#x}", list_address
        );
        let list = read_virtual::<LIST_ENTRY>(processor, list_address)?;
        let mut modules = Vec::new();

        let mut current = list.Flink;

        while current != list_address as *mut _ {
            // NOTE: _KLDR_DATA_TABLE_ENTRY struct is stable for all versions of 10, *except* for the following fields:
            // union
            // {
            //     VOID* Spare;                                                        //0x90
            //     struct _KLDR_DATA_TABLE_ENTRY* NtDataTableEntry;                    //0x90
            // };
            // ULONG SizeOfImageNotRounded;                                            //0x98
            // ULONG TimeDateStamp;                                                    //0x9c
            //
            // We don't use these, so it's ok to just use the latest version of this struct instead of generalizing
            let ldr_data_table_entry = read_virtual::<
                windows_10_0_22631_2428_x64::_KLDR_DATA_TABLE_ENTRY,
            >(processor, current as u64)?;

            let base = ldr_data_table_entry.DllBase as u64;
            let entry = ldr_data_table_entry.EntryPoint as u64;
            let size = ldr_data_table_entry.SizeOfImage as u64;
            let full_name = read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            )?;
            let base_name = read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            )?;
            let debug_info = full_name
                .split('\\')
                .last()
                .ok_or_else(|| anyhow!("Failed to get file name"))
                .and_then(|fname| {
                    // No need for DTB version because kernel is always mapped
                    DebugInfo::new(
                        processor,
                        fname,
                        base,
                        download_directory.as_ref(),
                        not_found_full_name_cache,
                        user_debug_info,
                    )
                })
                .ok();

            modules.push(Module {
                base,
                entry,
                size,
                full_name,
                base_name,
                debug_info,
            });

            current = ldr_data_table_entry.InLoadOrderLinks.Flink as *mut _;
        }

        Ok(modules)
    }

    fn find_ps_initial_system_process_address(&mut self) -> Result<u64> {
        // PsInitialSystemProcess is either a public(publicsymbol) in the PDB file, or if it is not
        // in the PDB file, we can find it via the export table in the PE file.
        let pdb_symbols = self.debug_info.pdb.global_symbols()?;
        let pdb_address_map = self.debug_info.pdb.address_map()?;
        if let Ok(Some(ps_initial_system_process_symbol)) =
            pdb_symbols.iter().find_map(|symbol| match symbol.parse() {
                Ok(symbol) => {
                    if let SymbolData::Public(public_symbol) = symbol {
                        if public_symbol.name.to_string() == "PsInitialSystemProcess" {
                            Ok(Some(
                                public_symbol
                                    .offset
                                    .to_rva(&pdb_address_map)
                                    .ok_or_else(|| pdb::Error::AddressMapNotFound)?
                                    .0 as u64
                                    + self.base,
                            ))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                }
                Err(e) => Err(e),
            })
        {
            Ok(ps_initial_system_process_symbol)
        } else {
            self.debug_info
                .exports()?
                .iter()
                .find(|export| {
                    export
                        .name
                        .as_ref()
                        .is_some_and(|name| name == "PsInitialSystemProcess")
                })
                .map(|export| export.rva as u64 + self.base)
                .ok_or_else(|| anyhow!("PsInitialSystemProcess not found"))
        }
    }

    pub fn current_process<P>(
        &mut self,
        processor: *mut ConfObject,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Process>
    where
        P: AsRef<Path>,
    {
        let kpcr = WindowsKpcr::new(processor, self.major, self.minor, self.build)?;
        let kprcb = WindowsKprcb::new(
            processor,
            self.major,
            self.minor,
            self.build,
            kpcr.kpcrb_address(),
        )?;
        let kthread = WindowsKThread::new(
            processor,
            self.major,
            self.minor,
            self.build,
            kprcb.current_thread(),
        )?;
        let eprocess = kthread.process(processor, self.major, self.minor, self.build)?;

        Ok(Process {
            pid: eprocess.pid(),
            file_name: eprocess.file_name(processor)?,
            base_address: eprocess.base_address(processor, self.major, self.minor, self.build)?,
            modules: eprocess
                .modules(
                    processor,
                    self.major,
                    self.minor,
                    self.build,
                    download_directory.as_ref(),
                    not_found_full_name_cache,
                    user_debug_info,
                )
                .unwrap_or_default(),
        })
    }

    pub fn process_list<P>(
        &mut self,
        processor: *mut ConfObject,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Vec<Process>>
    where
        P: AsRef<Path>,
    {
        let kpcr = WindowsKpcr::new(processor, self.major, self.minor, self.build)?;
        let kprcb = WindowsKprcb::new(
            processor,
            self.major,
            self.minor,
            self.build,
            kpcr.kpcrb_address(),
        )?;
        let kthread = WindowsKThread::new(
            processor,
            self.major,
            self.minor,
            self.build,
            kprcb.current_thread(),
        )?;
        let eprocess = kthread.process(processor, self.major, self.minor, self.build)?;
        // Print initial process info

        let mut processes = Vec::new();

        let mut list_entry = eprocess.active_process_links();
        let last_entry = list_entry.Blink;

        while !list_entry.Flink.is_null() {
            let eprocess = WindowsEProcess::new_from_active_process_links_address(
                processor,
                self.major,
                self.minor,
                self.build,
                list_entry.Flink as u64,
            )?;

            processes.push(Process {
                pid: eprocess.pid(),
                file_name: eprocess.file_name(processor)?,
                base_address: eprocess
                    .base_address(processor, self.major, self.minor, self.build)?,
                modules: eprocess
                    .modules(
                        processor,
                        self.major,
                        self.minor,
                        self.build,
                        download_directory.as_ref(),
                        not_found_full_name_cache,
                        user_debug_info,
                    )
                    .unwrap_or_default(),
            });

            list_entry = eprocess.active_process_links();

            if list_entry.Flink == last_entry {
                break;
            }
        }

        Ok(processes)
    }
}
