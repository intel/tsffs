use anyhow::{anyhow, bail, Result};
use goblin::pe::PE;
use intervaltree::Element;
use pdb::{FallibleIterator, SymbolData, PDB};
use reqwest::blocking::get;
use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, File},
    io::{copy, Write},
    path::{Path, PathBuf},
};

use lending_iterator::{windows_mut, LendingIterator};
use simics::{debug, info, ConfObject};
use windows::Win32::System::{
    Diagnostics::Debug::{
        IMAGE_DEBUG_DIRECTORY, IMAGE_DEBUG_TYPE_CODEVIEW, IMAGE_DIRECTORY_ENTRY_DEBUG,
        IMAGE_NT_HEADERS64,
    },
    SystemServices::IMAGE_DOS_HEADER,
};

use super::{
    pdb::{CvInfoPdb70, Export},
    util::{read_virtual, read_virtual_dtb},
};

#[derive(Debug)]
pub struct DebugInfo<'a> {
    pub exe_path: PathBuf,
    pub pdb_path: PathBuf,
    pub exe_file_contents: Vec<u8>,
    pub pdb: PDB<'a, File>,
}

impl<'a> DebugInfo<'a> {
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
        if let Some(info) = user_debug_info.get(name) {
            debug!("Have user-provided debug info for {name}");
            let exe_path = info[0].clone();
            let pdb_path = info[1].clone();

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            })
        } else {
            let dos_header = read_virtual::<IMAGE_DOS_HEADER>(processor, base)?;
            let nt_header =
                read_virtual::<IMAGE_NT_HEADERS64>(processor, base + dos_header.e_lfanew as u64)?;
            let debug_data_directory_offset = nt_header.OptionalHeader.DataDirectory
                [IMAGE_DIRECTORY_ENTRY_DEBUG.0 as usize]
                .VirtualAddress;
            let debug_data_directory_size =
                nt_header.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG.0 as usize].Size;
            let debug_directory = (base + debug_data_directory_offset as u64
                ..base + debug_data_directory_offset as u64 + debug_data_directory_size as u64)
                .step_by(std::mem::size_of::<IMAGE_DEBUG_DIRECTORY>())
                .filter_map(|offset| read_virtual::<IMAGE_DEBUG_DIRECTORY>(processor, offset).ok())
                .filter(|dd| dd.Type == IMAGE_DEBUG_TYPE_CODEVIEW)
                .take(1)
                .next()
                .ok_or_else(|| anyhow!("Failed to find debug data directory with codeview type"))?;

            if debug_directory.SizeOfData == 0 || debug_directory.AddressOfRawData == 0 {
                bail!("Invalid debug data directory");
            }

            let cv_info_pdb70 =
                CvInfoPdb70::new(processor, base + debug_directory.AddressOfRawData as u64)?;

            let exe_guid = format!(
                "{:08X}{:05X}",
                nt_header.FileHeader.TimeDateStamp, nt_header.OptionalHeader.SizeOfImage
            );

            // Download kernel PDB file
            let pdb_url = format!(
                "https://msdl.microsoft.com/download/symbols/{}/{}/{}",
                cv_info_pdb70.file_name(),
                cv_info_pdb70.guid(),
                cv_info_pdb70.file_name()
            );

            let exe_url = format!(
                "https://msdl.microsoft.com/download/symbols/{}/{}/{}",
                name, exe_guid, name
            );

            if !download_directory.as_ref().is_dir() {
                create_dir_all(&download_directory)?;
            }

            // Download kernel PE file
            let exe_path = download_directory
                .as_ref()
                .join(format!("{}.exe", &exe_guid));

            if !exe_path.exists() && !not_found_full_name_cache.contains(name) {
                info!("Downloading PE file from {}", exe_url);
                match get(&exe_url)?.error_for_status() {
                    Ok(response) => {
                        let mut file = File::create(&exe_path)?;
                        copy(&mut response.bytes()?.as_ref(), &mut file)?;
                        file.flush()?;
                    }
                    Err(e) => {
                        not_found_full_name_cache.insert(name.to_string());
                        bail!("Failed to download PE file: {}", e);
                    }
                }
            }

            let pdb_path = download_directory
                .as_ref()
                .join(format!("{}.pdb", cv_info_pdb70.guid()));

            if !pdb_path.exists() && !not_found_full_name_cache.contains(cv_info_pdb70.file_name())
            {
                info!("Downloading PDB file from {}", pdb_url);
                match get(&pdb_url)?.error_for_status() {
                    Ok(response) => {
                        let mut file = File::create(&pdb_path)?;
                        copy(&mut response.bytes()?.as_ref(), &mut file)?;
                        file.flush()?;
                    }
                    Err(e) => {
                        not_found_full_name_cache.insert(cv_info_pdb70.guid().to_string());
                        bail!("Failed to download PDB file: {}", e);
                    }
                }
            }

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            })
        }
    }

    pub fn new_dtb<P>(
        processor: *mut ConfObject,
        name: &str,
        base: u64,
        download_directory: P,
        directory_table_base: u64,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        if let Some(info) = user_debug_info.get(name) {
            debug!("Have user-provided debug info for {name}");
            let exe_path = info[0].clone();
            let pdb_path = info[1].clone();

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            })
        } else {
            let dos_header =
                read_virtual_dtb::<IMAGE_DOS_HEADER>(processor, directory_table_base, base)?;
            let nt_header = read_virtual_dtb::<IMAGE_NT_HEADERS64>(
                processor,
                directory_table_base,
                base + dos_header.e_lfanew as u64,
            )?;
            let debug_data_directory_offset = nt_header.OptionalHeader.DataDirectory
                [IMAGE_DIRECTORY_ENTRY_DEBUG.0 as usize]
                .VirtualAddress;
            let debug_data_directory_size =
                nt_header.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG.0 as usize].Size;
            let debug_directory = (base + debug_data_directory_offset as u64
                ..base + debug_data_directory_offset as u64 + debug_data_directory_size as u64)
                .step_by(std::mem::size_of::<IMAGE_DEBUG_DIRECTORY>())
                .filter_map(|offset| {
                    read_virtual_dtb::<IMAGE_DEBUG_DIRECTORY>(
                        processor,
                        directory_table_base,
                        offset,
                    )
                    .ok()
                })
                .filter(|dd| dd.Type == IMAGE_DEBUG_TYPE_CODEVIEW)
                .take(1)
                .next()
                .ok_or_else(|| anyhow!("Failed to find debug data directory with codeview type"))?;

            if debug_directory.SizeOfData == 0 || debug_directory.AddressOfRawData == 0 {
                bail!("Invalid debug data directory");
            }

            let cv_info_pdb70 =
                CvInfoPdb70::new(processor, base + debug_directory.AddressOfRawData as u64)?;

            let exe_guid = format!(
                "{:08X}{:05X}",
                nt_header.FileHeader.TimeDateStamp, nt_header.OptionalHeader.SizeOfImage
            );

            // Download kernel PDB file
            let pdb_url = format!(
                "https://msdl.microsoft.com/download/symbols/{}/{}/{}",
                cv_info_pdb70.file_name(),
                cv_info_pdb70.guid(),
                cv_info_pdb70.file_name()
            );

            let exe_url = format!(
                "https://msdl.microsoft.com/download/symbols/{}/{}/{}",
                name, exe_guid, name
            );

            if !download_directory.as_ref().is_dir() {
                create_dir_all(&download_directory)?;
            }

            // Download kernel PE file
            let exe_path = download_directory
                .as_ref()
                .join(format!("{}.exe", &exe_guid));

            if !exe_path.exists() && !not_found_full_name_cache.contains(name) {
                info!("Downloading PE file from {}", exe_url);
                match get(&exe_url)?.error_for_status() {
                    Ok(response) => {
                        let mut file = File::create(&exe_path)?;
                        copy(&mut response.bytes()?.as_ref(), &mut file)?;
                        file.flush()?;
                    }
                    Err(e) => {
                        not_found_full_name_cache.insert(name.to_string());
                        bail!("Failed to download PE file: {}", e);
                    }
                }
            }

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_path = download_directory
                .as_ref()
                .join(format!("{}.pdb", cv_info_pdb70.guid()));

            if !pdb_path.exists() && !not_found_full_name_cache.contains(cv_info_pdb70.file_name())
            {
                info!("Downloading PDB file from {}", pdb_url);
                match get(&pdb_url)?.error_for_status() {
                    Ok(response) => {
                        let mut file = File::create(&pdb_path)?;
                        copy(&mut response.bytes()?.as_ref(), &mut file)?;
                        file.flush()?;
                    }
                    Err(e) => {
                        not_found_full_name_cache.insert(cv_info_pdb70.guid().to_string());
                        bail!("Failed to download PDB file: {}", e);
                    }
                }
            }

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            })
        }
    }

    pub fn exe(&self) -> Result<PE<'_>> {
        PE::parse(&self.exe_file_contents)
            .map_err(move |e| anyhow!("Failed to parse PE file: {}", e))
    }

    pub fn exports(&self) -> Result<Vec<Export>> {
        Ok(self.exe()?.exports.iter().map(Export::from).collect())
    }
}

#[derive(Debug)]
pub struct ProcessModule {
    pub base: u64,
    pub size: u64,
    pub full_name: String,
    pub base_name: String,
    pub debug_info: Option<DebugInfo<'static>>,
}

impl ProcessModule {
    pub fn intervals(
        &mut self,
        guess_pdb_function_size: bool,
    ) -> Result<Vec<Element<u64, SymbolInfo>>> {
        let mut syms = Vec::new();

        if let Some(debug_info) = self.debug_info.as_mut() {
            let symbol_table = debug_info.pdb.global_symbols()?;
            let address_map = debug_info.pdb.address_map()?;
            // let debug_information = debug_info.pdb.debug_information()?;
            let mut symbols = symbol_table.iter();
            while let Some(symbol) = symbols.next()? {
                match symbol.parse() {
                    Ok(sd) => {
                        match sd {
                            SymbolData::Public(p) => {
                                if p.function {
                                    // NOTE: Public symbols don't have sizes, the address is just
                                    // the RVA of their entry point, so we just do an entry of size 1
                                    if let Some(rva) = p.offset.to_rva(&address_map) {
                                        let info = SymbolInfo::new(
                                            rva.0 as u64,
                                            0,
                                            p.name.to_string().to_string(),
                                            self.full_name.clone(),
                                        );
                                        syms.push(info);
                                    }
                                }
                            }
                            SymbolData::Procedure(p) => {
                                if let Some(rva) = p.offset.to_rva(&address_map) {
                                    let info = SymbolInfo::new(
                                        rva.0 as u64,
                                        p.len as u64,
                                        p.name.to_string().to_string(),
                                        self.full_name.clone(),
                                    );
                                    syms.push(info);
                                }
                            }
                            SymbolData::ProcedureReference(_p) => {
                                // TODO
                            }
                            SymbolData::Trampoline(_t) => {
                                // TODO
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let _ = e;
                    }
                }
            }
        }

        if guess_pdb_function_size {
            syms.sort_by(|a, b| a.rva.cmp(&b.rva));
            windows_mut(&mut syms).for_each(|w: &mut [SymbolInfo; 2]| {
                if w[0].size == 0 {
                    w[0].size = w[1].rva - w[0].rva;
                }
            });
        }

        Ok(syms
            .into_iter()
            .map(|s| (self.base + s.rva..self.base + s.rva + s.size, s).into())
            .collect())
    }
}

#[derive(Debug)]
pub struct Process {
    pub pid: u64,
    pub file_name: String,
    pub base_address: u64,
    pub modules: Vec<ProcessModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolInfo {
    pub rva: u64,
    pub size: u64,
    pub name: String,
    pub module: String,
}

impl SymbolInfo {
    pub fn new(rva: u64, size: u64, name: String, module: String) -> Self {
        Self {
            rva,
            size,
            name,
            module,
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub base: u64,
    pub entry: u64,
    pub size: u64,
    pub full_name: String,
    pub base_name: String,
    pub debug_info: Option<DebugInfo<'static>>,
}

impl Module {
    pub fn intervals(
        &mut self,
        guess_pdb_function_size: bool,
    ) -> Result<Vec<Element<u64, SymbolInfo>>> {
        let mut syms = Vec::new();

        if let Some(debug_info) = self.debug_info.as_mut() {
            let symbol_table = debug_info.pdb.global_symbols()?;
            let address_map = debug_info.pdb.address_map()?;
            let mut symbols = symbol_table.iter();
            while let Some(symbol) = symbols.next()? {
                match symbol.parse() {
                    Ok(sd) => {
                        match sd {
                            SymbolData::Public(p) => {
                                if p.function {
                                    // NOTE: Public symbols don't have sizes, the address is just
                                    // the RVA of their entry point, so we just do an entry of size 1
                                    if let Some(rva) = p.offset.to_rva(&address_map) {
                                        let info = SymbolInfo::new(
                                            rva.0 as u64,
                                            1,
                                            p.name.to_string().to_string(),
                                            self.full_name.clone(),
                                        );
                                        syms.push(info);
                                    }
                                }
                            }
                            SymbolData::Procedure(p) => {
                                if let Some(rva) = p.offset.to_rva(&address_map) {
                                    let info = SymbolInfo::new(
                                        rva.0 as u64,
                                        p.len as u64,
                                        p.name.to_string().to_string(),
                                        self.full_name.clone(),
                                    );
                                    syms.push(info);
                                }
                            }
                            SymbolData::ProcedureReference(_p) => {
                                // TODO
                            }
                            SymbolData::Trampoline(_t) => {
                                // TODO
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        let _ = e;
                    }
                }
            }
        }

        if guess_pdb_function_size {
            syms.sort_by(|a, b| a.rva.cmp(&b.rva));
            windows_mut(&mut syms).for_each(|w: &mut [SymbolInfo; 2]| {
                if w[0].size == 0 {
                    w[0].size = w[1].rva - w[0].rva;
                }
            });
        }

        Ok(syms
            .into_iter()
            .map(|s| (self.base + s.rva..self.base + s.rva + s.size, s).into())
            .collect())
    }
}
