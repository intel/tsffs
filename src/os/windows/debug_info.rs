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
use simics::{debug, get_object, info, warn, ConfObject};
use windows_sys::Win32::System::{
    Diagnostics::Debug::{
        IMAGE_DEBUG_DIRECTORY, IMAGE_DEBUG_TYPE_CODEVIEW, IMAGE_DIRECTORY_ENTRY_DEBUG,
        IMAGE_NT_HEADERS64,
    },
    SystemServices::{FILE_NOTIFY_FULL_INFORMATION, IMAGE_DOS_HEADER},
};

use crate::{os::DebugInfoConfig, source_cov::SourceCache};

use super::{
    pdb::{CvInfoPdb70, Export},
    util::{read_virtual, read_virtual_dtb},
};

#[derive(Debug)]
/// Debug info for an executable (which may be a .exe, .sys, etc)
pub struct DebugInfo<'a> {
    /// The path to the executable file on the local system
    pub exe_path: PathBuf,
    /// The path to the PDB file corresponding to the executable on the local system
    pub pdb_path: PathBuf,
    /// The contents of the executable file
    pub exe_file_contents: Vec<u8>,
    /// The loaded PDB info
    pub pdb: PDB<'a, File>,
}

impl<'a> DebugInfo<'a> {
    /// Instantiate a new debug info for an object
    pub fn new<P>(
        processor: *mut ConfObject,
        name: &str,
        base: u64,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &DebugInfoConfig,
    ) -> Result<Option<Self>>
    where
        P: AsRef<Path>,
    {
        if let Some(info) = user_debug_info.user_debug_info.get(name) {
            debug!(
                get_object("tsffs")?,
                "Have user-provided debug info for {name}"
            );
            let exe_path = info[0].clone();
            let pdb_path = info[1].clone();

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Some(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            }))
        } else if user_debug_info.system {
            let dos_header = read_virtual::<IMAGE_DOS_HEADER>(processor, base)?;
            let nt_header =
                read_virtual::<IMAGE_NT_HEADERS64>(processor, base + dos_header.e_lfanew as u64)?;
            let debug_data_directory_offset = nt_header.OptionalHeader.DataDirectory
                [IMAGE_DIRECTORY_ENTRY_DEBUG as usize]
                .VirtualAddress;
            let debug_data_directory_size =
                nt_header.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG as usize].Size;
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
                info!(get_object("tsffs")?, "Downloading PE file from {}", exe_url);
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
                info!(
                    get_object("tsffs")?,
                    "Downloading PDB file from {}", pdb_url
                );
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

            Ok(Some(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            }))
        } else {
            // bail!("No debug info provided for {name}");
            Ok(None)
        }
    }

    /// Instantiate a new debug info for an object with a specific directory table base
    pub fn new_dtb<P>(
        processor: *mut ConfObject,
        name: &str,
        base: u64,
        download_directory: P,
        directory_table_base: u64,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: DebugInfoConfig,
    ) -> Result<Option<Self>>
    where
        P: AsRef<Path>,
    {
        if let Some(info) = user_debug_info.user_debug_info.get(name) {
            debug!(
                get_object("tsffs")?,
                "Have user-provided debug info for {name}"
            );
            let exe_path = info[0].clone();
            let pdb_path = info[1].clone();

            let exe_file_contents = std::fs::read(&exe_path)?;

            let pdb_file = File::open(&pdb_path)?;

            let pdb = PDB::open(pdb_file)?;

            Ok(Some(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            }))
        } else if user_debug_info.system {
            let dos_header =
                read_virtual_dtb::<IMAGE_DOS_HEADER>(processor, directory_table_base, base)?;
            let nt_header = read_virtual_dtb::<IMAGE_NT_HEADERS64>(
                processor,
                directory_table_base,
                base + dos_header.e_lfanew as u64,
            )?;
            let debug_data_directory_offset = nt_header.OptionalHeader.DataDirectory
                [IMAGE_DIRECTORY_ENTRY_DEBUG as usize]
                .VirtualAddress;
            let debug_data_directory_size =
                nt_header.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG as usize].Size;
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
                info!(get_object("tsffs")?, "Downloading PE file from {}", exe_url);
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
                info!(
                    get_object("tsffs")?,
                    "Downloading PDB file from {}", pdb_url
                );
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

            Ok(Some(Self {
                exe_path,
                pdb_path,
                exe_file_contents,
                pdb,
            }))
        } else {
            Ok(None)
        }
    }

    /// Return the parsed PE file
    pub fn exe(&self) -> Result<PE<'_>> {
        PE::parse(&self.exe_file_contents)
            .map_err(move |e| anyhow!("Failed to parse PE file: {}", e))
    }

    /// Get a list of exports from the PE file
    pub fn exports(&self) -> Result<Vec<Export>> {
        Ok(self.exe()?.exports.iter().map(Export::from).collect())
    }
}

#[derive(Debug)]
/// A module (or object) loaded in a specific process
pub struct ProcessModule {
    /// The base of the object
    pub base: u64,
    /// The size of the object
    pub size: u64,
    /// The full name (typically a path) of the object on disk
    pub full_name: String,
    /// The base name of the object
    pub base_name: String,
    /// Loaded debug info for the object
    pub debug_info: Option<DebugInfo<'static>>,
}

impl ProcessModule {
    /// Return lookup intervals for symbols in the process module which can be used to build
    /// an interval tree
    pub fn intervals(
        &mut self,
        source_cache: &SourceCache,
    ) -> Result<Vec<Element<u64, SymbolInfo>>> {
        let Some(debug_info) = self.debug_info.as_mut() else {
            bail!("No debug info for module {}", self.full_name);
        };

        let string_table = debug_info.pdb.string_table()?;
        let address_map = debug_info.pdb.address_map()?;
        let symbols = debug_info
            .pdb
            .debug_information()?
            .modules()?
            .iterator()
            .filter_map(|module| module.ok())
            .filter_map(|module| {
                debug_info
                    .pdb
                    .module_info(&module)
                    .ok()
                    .flatten()
                    .map(|module_info| (module, module_info))
            })
            .flat_map(|(_module, module_info)| {
                let Ok(line_program) = module_info.line_program() else {
                    return Vec::new();
                };

                let Ok(symbols) = module_info.symbols() else {
                    return Vec::new();
                };

                symbols
                    .iterator()
                    .filter_map(|symbol| symbol.ok())
                    .filter_map(|symbol| {
                        symbol.parse().ok().map(|symbol_data| (symbol, symbol_data))
                    })
                    .filter_map(|(_symbol, symbol_data)| {
                        let SymbolData::Procedure(procedure_symbol) = symbol_data else {
                            return None;
                        };
                        let symbol_name = symbol_data.name()?;
                        let procedure_rva = procedure_symbol.offset.to_rva(&address_map)?;

                        let lines = line_program
                            .lines_for_symbol(procedure_symbol.offset)
                            .iterator()
                            .filter_map(|line| line.ok())
                            .filter_map(|line_info| {
                                line_program
                                    .get_file_info(line_info.file_index)
                                    .ok()
                                    .and_then(|line_file_info| {
                                        string_table
                                            .get(line_file_info.name)
                                            .map(|line_file_name| (line_file_info, line_file_name))
                                            .ok()
                                    })
                                    .and_then(|(line_file_info, line_file_name)| {
                                        line_info.offset.to_rva(&address_map).map(|line_rva| {
                                            (line_file_info, line_file_name, line_rva, line_info)
                                        })
                                    })
                                    .and_then(
                                        |(line_file_info, line_file_name, line_rva, line_info)| {
                                            source_cache
                                                .lookup_pdb(
                                                    &line_file_info,
                                                    &line_file_name.to_string(),
                                                )
                                                .ok()
                                                .flatten()
                                                .map(|p| p.to_path_buf())
                                                .map(|file_path| LineInfo {
                                                    rva: line_rva.0 as u64,
                                                    size: line_info.length.unwrap_or(1),
                                                    file_path,
                                                    start_line: line_info.line_start,
                                                    end_line: line_info.line_end,
                                                })
                                        },
                                    )
                            })
                            .collect::<Vec<_>>();
                        let info = SymbolInfo::new(
                            procedure_rva.0 as u64,
                            self.base,
                            procedure_symbol.len as u64,
                            symbol_name.to_string().to_string(),
                            self.full_name.clone(),
                            lines,
                        );

                        Some(info)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(symbols
            .into_iter()
            .map(|s| (self.base + s.rva..self.base + s.rva + s.size, s).into())
            .collect())
    }
}

#[derive(Debug)]
/// A process
pub struct Process {
    /// The unique PID of the process
    pub pid: u64,
    /// The file name of the process's main object
    pub file_name: String,
    /// The base address of the process's main object
    pub base_address: u64,
    /// The list of modules/objects loaded into the process's address space
    pub modules: Vec<ProcessModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Information about a line in a source file from a PDB
pub struct LineInfo {
    /// The relative virtual address in the executable image
    pub rva: u64,
    /// The size in bytes of the code this line represents
    pub size: u32,
    /// The file path of the source file on the *local* filesystem. This path is found by
    /// looking up the pdb source path in the source cache on a best-effort approach
    pub file_path: PathBuf,
    /// The line number in the source file that this line starts at
    pub start_line: u32,
    /// The line number in the source file that this line ends at
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Information about a symbol in a PDB, including the member lines of the symbol, if any.
pub struct SymbolInfo {
    /// The relative virtual address in the executable image
    pub rva: u64,
    /// The base address of the executable image
    pub base: u64,
    /// The size of the symbol (e.g. function size)
    pub size: u64,
    /// The (possibly mangled) name of the symbol
    pub name: String,
    /// The name of the module the symbol is in
    pub module: String,
    /// The source lines of code for the symbol
    pub lines: Vec<LineInfo>,
}

impl SymbolInfo {
    pub fn new(
        rva: u64,
        base: u64,
        size: u64,
        name: String,
        module: String,
        lines: Vec<LineInfo>,
    ) -> Self {
        Self {
            rva,
            base,
            size,
            name,
            module,
            lines,
        }
    }
}

#[derive(Debug)]
/// A kernel module/driver
pub struct Module {
    /// The base address of the module
    pub base: u64,
    /// The entrypoint of the module
    pub entry: u64,
    /// The size of the module
    pub size: u64,
    /// The full name of the module
    pub full_name: String,
    /// The base name of the module
    pub base_name: String,
    /// The loaded debug info for the module
    pub debug_info: Option<DebugInfo<'static>>,
}

impl Module {
    /// Return lookup intervals for symbols in the module which can be used to build an interval tree
    pub fn intervals(
        &mut self,
        source_cache: &SourceCache,
    ) -> Result<Vec<Element<u64, SymbolInfo>>> {
        let Some(debug_info) = self.debug_info.as_mut() else {
            bail!("No debug info for module {}", self.full_name);
        };

        let string_table = debug_info.pdb.string_table()?;
        let address_map = debug_info.pdb.address_map()?;
        let symbols = debug_info
            .pdb
            .debug_information()?
            .modules()?
            .iterator()
            .filter_map(|module| module.ok())
            .filter_map(|module| {
                debug_info
                    .pdb
                    .module_info(&module)
                    .ok()
                    .flatten()
                    .map(|module_info| (module, module_info))
            })
            .flat_map(|(_module, module_info)| {
                let Ok(line_program) = module_info.line_program() else {
                    return Vec::new();
                };

                let Ok(symbols) = module_info.symbols() else {
                    return Vec::new();
                };

                symbols
                    .iterator()
                    .filter_map(|symbol| symbol.ok())
                    .filter_map(|symbol| {
                        symbol.parse().ok().map(|symbol_data| (symbol, symbol_data))
                    })
                    .filter_map(|(_symbol, symbol_data)| {
                        let SymbolData::Procedure(procedure_symbol) = symbol_data else {
                            return None;
                        };
                        let symbol_name = symbol_data.name()?;
                        let procedure_rva = procedure_symbol.offset.to_rva(&address_map)?;

                        let lines = line_program
                            .lines_for_symbol(procedure_symbol.offset)
                            .iterator()
                            .filter_map(|line| line.ok())
                            .filter_map(|line_info| {
                                let Ok(line_file_info) =
                                    line_program.get_file_info(line_info.file_index)
                                else {
                                    if let Ok(o) = get_object("tsffs") {
                                        debug!(o, "No file info for line {:?}", line_info);
                                    }
                                    return None;
                                };

                                let Ok(line_file_name) = string_table.get(line_file_info.name)
                                else {
                                    if let Ok(o) = get_object("tsffs") {
                                        debug!(o, "No file name for line {:?}", line_file_info);
                                    }
                                    return None;
                                };

                                let Some(line_rva) = line_info.offset.to_rva(&address_map) else {
                                    if let Ok(o) = get_object("tsffs") {
                                        debug!(o, "No RVA for line {:?}", line_info);
                                    }
                                    return None;
                                };

                                let Ok(Some(source_file)) = source_cache
                                    .lookup_pdb(&line_file_info, &line_file_name.to_string())
                                else {
                                    if let Ok(o) = get_object("tsffs") {
                                        debug!(o, "No source file path for line {:?}", line_info);
                                    }
                                    return None;
                                };

                                let info = LineInfo {
                                    rva: line_rva.0 as u64,
                                    size: line_info.length.unwrap_or(1),
                                    file_path: source_file.to_path_buf(),
                                    start_line: line_info.line_start,
                                    end_line: line_info.line_end,
                                };
                                if let Ok(o) = get_object("tsffs") {
                                    debug!(o, "Got line info {:?}", line_info);
                                }

                                Some(info)
                            })
                            .collect::<Vec<_>>();

                        let info = SymbolInfo::new(
                            procedure_rva.0 as u64,
                            self.base,
                            procedure_symbol.len as u64,
                            symbol_name.to_string().to_string(),
                            self.full_name.clone(),
                            lines,
                        );

                        Some(info)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(symbols
            .into_iter()
            .map(|s| (self.base + s.rva..self.base + s.rva + s.size, s).into())
            .collect())
    }
}
