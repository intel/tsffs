// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! CdyLib signing tools for signing ELF and PE libraries to be loaded by Simics

#![deny(missing_docs)]

use chrono::Local;
use object::{
    elf::FileHeader64,
    endian::LittleEndian,
    read::{elf::ElfFile, pe::PeFile64},
    Object, ObjectSection, ObjectSymbol,
};
use std::{
    fs::{read, OpenOptions},
    io::Write,
    iter::once,
    num::Wrapping,
    path::{Path, PathBuf},
};

/// The symbol name containing the capabilities and signature of a module
pub const MODULE_CAPABILITIES_SYMNAME: &str = "_module_capabilities_";
/// The symbol name containing the date of the module
pub const MODULE_DATE_SYMNAME: &str = "_module_date";
/// The name of the text section for both Linux and Windows Gnu toolchains
pub const TEXT_SECTION_NAME: &str = ".text";
/// The name of the data section for both Linux and Windows Gnu toolchains
pub const DATA_SECTION_NAME: &str = ".data";
/// The maximum amount of data in a given section (text or data) to checksum
pub const MAX_SECTION_CSUM_SIZE: usize = 256;
// NOTE: Simics does not handle a username longer than 20 characters in its signing check and
// may clobber the ELF if it sees a longer one. We won't allow that (20 chars + nul =
// 21)
/// The maximum length of a username used in a signature
pub const SIMICS_SIGNATURE_UNAME_MAX_LEN: usize = 20;
/// The minimum length of the signature field of the module capabilities symbol
pub const SIMICS_SIGNATURE_MIN_LENGTH: usize = 44;

type Elf<'data> = ElfFile<'data, FileHeader64<LittleEndian>>;

#[derive(Debug, thiserror::Error)]
/// An error type raised during singing
pub enum Error {
    #[error("File type of {path:?} not recognized. Must be PE64 or ELF64.")]
    /// The file type of the input is not recognized and does not correctly parse as either
    /// PE64 or ELF64.
    FileTypeNotRecognized {
        /// The path to the file that is not recognized
        path: PathBuf,
    },
    #[error("_module_capabilities_ symbol missing")]
    /// The _module_capabilities_ symbol could not be found in the file
    ModuleCapabilitiesMissing,
    #[error("Section not found for symbol {symbol} in {path:?}")]
    /// A section containing a symbol with a given name could not be found in the file
    SectionNotFound {
        /// The symbol whose section could not be found
        symbol: String,
        /// The path which was missing the symbol
        path: PathBuf,
    },
    #[error("_module_capabilities_ split sequence missing from symbol value")]
    /// The sequence that splits the _module_capabilities_ symbol (usually '; ') was not found
    SplitSequenceNotFound,
    #[error("Section {section} not found in {path:?}")]
    /// A section with a given name was not found
    SectionMissing {
        /// The name of the section that could not be found
        section: String,
        /// The path which was missing the symbol
        path: PathBuf,
    },
    #[error("Signature unchanged after signing")]
    /// The signature block was not modified by the signing process. This is a sanity check.
    SignatureUnchanged,
    #[error("Module unchanged after signing")]
    /// The module was not modified by the signing process. This is a sanity check.
    ModuleUnchanged,
    #[error("File range for section {section} not found")]
    /// No range of offsets in the file were found for a secion
    SectionFileRangeMissing {
        /// The name of the section whose offset range could not be determined
        section: String,
    },
    #[error("Original and signed module lengths differ")]
    /// The length of the original and signed modules differ. This is a sanity check.
    ModuleLengthMismatch,
    #[error("Missing terminating null byte in module capabilities")]
    /// A null byte was not found in the _module_capabilities_ symbol
    NullByteMissing,
    #[error("Missing parent directory for path {path:?}")]
    /// No parent directory for a path
    MissingParentDirectory {
        /// The path whose parent directory was not found
        path: PathBuf,
    },
    #[error("Module is not signed.")]
    /// The module was not signed
    ModuleNotSigned,
    #[error("Failed to open module output file {path:?}: {source}")]
    /// An error occurred while opening the output file
    OpenOutputFile {
        /// The path to the output file that could not be opened
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Failed to set permissions for output file {path:?}: {source}")]
    /// An error occurred while setting the permissions of the output file
    SetPermissions {
        /// The path to the output file that could not have its permissions set
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Failed to get metadata for output file {path:?}: {source}")]
    /// An error occurred while getting the metadata of the output file
    GetMetadata {
        /// The path to the output file that could not have its metadata retrieved
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Failed to read directory {path:?}: {source}")]
    /// An error occurred while reading a directory
    ReadDirectory {
        /// The path to the directory that could not be read
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Failed to write module output file to {path:?}: {source}")]
    /// An error occurred while writing the output file
    WriteOutputFile {
        /// The path to the output file that could not be written
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error(transparent)]
    /// A wrapped std::io::Error
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    /// A wrapped object::Error
    ObjectError(#[from] object::Error),
}

type Result<T> = std::result::Result<T, Error>;

/// A module for signing
pub struct Sign {
    module: PathBuf,
    data: Vec<u8>,
    signed: Vec<u8>,
}

impl Sign {
    /// Start a new sign operation on a module located at a path
    pub fn new<P>(module: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let data = read(module.as_ref())?;

        let mut slf = Self {
            module: module.as_ref().to_path_buf(),
            data: data.clone(),
            signed: Vec::new(),
        };

        let data = &data[..];

        if let Ok(elf) = Elf::parse(data) {
            slf.sign_elf(elf)?;
            Ok(slf)
        } else if let Ok(pe) = PeFile64::parse(data) {
            slf.sign_pe(pe)?;
            Ok(slf)
        } else {
            Err(Error::FileTypeNotRecognized {
                path: slf.module.clone(),
            })
        }
    }

    fn sign_elf(&mut self, elf: Elf) -> Result<()> {
        let module_capabilities = elf
            .symbols()
            .find(|s| s.name() == Ok(MODULE_CAPABILITIES_SYMNAME))
            .ok_or_else(|| Error::ModuleCapabilitiesMissing)?;

        let module_capabilities_data = &elf.data()[module_capabilities.address() as usize
            ..module_capabilities.address() as usize + module_capabilities.size() as usize];

        let signature = [b" ".repeat(43), b";\x00".to_vec()].concat();

        let elf_data = elf.data().to_vec();

        // Check if already signed -- ends with (" "*43);\x00
        if !module_capabilities_data.ends_with(&signature) {
            println!(
                "Already signed with signature {:?}",
                &module_capabilities_data[module_capabilities_data.len() - signature.len()..]
            );
            self.signed = elf_data;
            // Already signed
            return Ok(());
        }

        let split_seq = b"; ";

        let signature_position = module_capabilities_data
            .windows(split_seq.len())
            .position(|w| w == split_seq)
            .ok_or_else(|| Error::SplitSequenceNotFound)?
            + split_seq.len();

        let text_section =
            elf.section_by_name(TEXT_SECTION_NAME)
                .ok_or_else(|| Error::SectionMissing {
                    section: TEXT_SECTION_NAME.to_string(),
                    path: self.module.clone(),
                })?;

        let data_section =
            elf.section_by_name(DATA_SECTION_NAME)
                .ok_or_else(|| Error::SectionMissing {
                    section: DATA_SECTION_NAME.to_string(),
                    path: self.module.clone(),
                })?;

        // Checksum = 1 * (text_section.size * sum(text_section.data)) * (data_section.size * sum(data_section.data)) | 1
        let csum: Wrapping<u32> = (Wrapping(1u32)
            * (Wrapping(text_section.size() as u32)
                * text_section
                    .data()?
                    .iter()
                    .take(MAX_SECTION_CSUM_SIZE)
                    .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32)))
            * (Wrapping(data_section.size() as u32)
                * data_section
                    .data()?
                    .iter()
                    .take(MAX_SECTION_CSUM_SIZE)
                    .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32))))
            | Wrapping(1u32);

        let uname = "simics"
            .chars()
            .take(SIMICS_SIGNATURE_UNAME_MAX_LEN)
            .collect::<String>();

        let datetime_string = Local::now().format("%Y-%M-%d %H:%M").to_string();

        let mut signature = module_capabilities_data[..signature_position]
            .iter()
            .chain(once(&0u8))
            .chain(&csum.0.to_le_bytes())
            .chain(once(&0u8))
            .chain(datetime_string.as_bytes())
            .chain(once(&b';'))
            .chain(uname.as_bytes())
            .cloned()
            .collect::<Vec<_>>();

        signature.resize(
            module_capabilities_data[..signature_position].len() + SIMICS_SIGNATURE_MIN_LENGTH,
            0u8,
        );

        if signature == module_capabilities_data {
            return Err(Error::SignatureUnchanged);
        }

        let pre_sig = elf_data[..module_capabilities.address() as usize].to_vec();

        let post_sig = elf_data
            [module_capabilities.address() as usize + module_capabilities.size() as usize..]
            .to_vec();

        self.signed = pre_sig
            .iter()
            .chain(signature.iter())
            .chain(post_sig.iter())
            .cloned()
            .collect::<Vec<_>>();

        if self.data.len() != self.signed.len() {
            return Err(Error::ModuleLengthMismatch);
        }

        if self.data == self.signed {
            return Err(Error::ModuleUnchanged);
        }

        Ok(())
    }

    fn sign_pe(&mut self, pe: PeFile64) -> Result<()> {
        let module_capabilities = pe
            .symbols()
            .find(|s| s.name() == Ok(MODULE_CAPABILITIES_SYMNAME))
            .ok_or_else(|| Error::ModuleCapabilitiesMissing)?;

        let module_capabilities_section =
            pe.section_by_index(module_capabilities.section().index().ok_or_else(|| {
                Error::SectionNotFound {
                    symbol: MODULE_CAPABILITIES_SYMNAME.to_string(),
                    path: self.module.clone(),
                }
            })?)?;
        let module_capabilities_offset = ((module_capabilities.address()
            - module_capabilities_section.address())
            + module_capabilities_section
                .file_range()
                .ok_or_else(|| Error::SectionFileRangeMissing {
                    section: module_capabilities_section
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|_| "unknown".to_string()),
                })?
                .0) as usize;

        let module_capabilities_size = if module_capabilities.size() > 0 {
            module_capabilities.size() as usize
        } else {
            &pe.data()[module_capabilities_offset..]
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(|| Error::NullByteMissing)?
                + 1
        };

        let module_capabilities_data = &pe.data()
            [module_capabilities_offset..module_capabilities_offset + module_capabilities_size];

        let split_seq = b"; ";

        let signature_position = module_capabilities_data
            .windows(split_seq.len())
            .position(|w| w == split_seq)
            .ok_or_else(|| Error::SplitSequenceNotFound)?
            + split_seq.len();

        let text_section =
            pe.section_by_name(TEXT_SECTION_NAME)
                .ok_or_else(|| Error::SectionMissing {
                    section: TEXT_SECTION_NAME.to_string(),
                    path: self.module.clone(),
                })?;

        let data_section =
            pe.section_by_name(DATA_SECTION_NAME)
                .ok_or_else(|| Error::SectionMissing {
                    section: DATA_SECTION_NAME.to_string(),
                    path: self.module.clone(),
                })?;

        // Checksum = 1 * (text_section.size * sum(text_section.data)) * (data_section.size * sum(data_section.data)) | 1
        let csum: Wrapping<u32> = (Wrapping(1u32)
            * (Wrapping(text_section.size() as u32)
                * text_section
                    .data()?
                    .iter()
                    .take(MAX_SECTION_CSUM_SIZE)
                    .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32)))
            * (Wrapping(data_section.size() as u32)
                * data_section
                    .data()?
                    .iter()
                    .take(MAX_SECTION_CSUM_SIZE)
                    .fold(Wrapping(0u32), |a, e| a + Wrapping(*e as u32))))
            | Wrapping(1u32);

        let uname = "simics"
            .chars()
            .take(SIMICS_SIGNATURE_UNAME_MAX_LEN)
            .collect::<String>();

        let datetime_string = Local::now().format("%Y-%M-%d %H:%M").to_string();

        let mut signature = module_capabilities_data[..signature_position]
            .iter()
            .chain(once(&0u8))
            .chain(&csum.0.to_le_bytes())
            .chain(once(&0u8))
            .chain(datetime_string.as_bytes())
            .chain(once(&b';'))
            .chain(uname.as_bytes())
            .cloned()
            .collect::<Vec<_>>();

        signature.resize(
            module_capabilities_data[..signature_position].len() + SIMICS_SIGNATURE_MIN_LENGTH,
            0u8,
        );

        if signature == module_capabilities_data {
            return Err(Error::SignatureUnchanged);
        }

        let pe_data = pe.data().to_vec();
        let pre_sig = pe_data[..module_capabilities_offset].to_vec();
        let post_sig = pe_data[module_capabilities_offset + module_capabilities_size..].to_vec();
        self.signed = pre_sig
            .iter()
            .chain(signature.iter())
            .chain(post_sig.iter())
            .cloned()
            .collect::<Vec<_>>();

        if self.data.len() != self.signed.len() {
            return Err(Error::ModuleLengthMismatch);
        }

        if self.data == self.signed {
            return Err(Error::ModuleUnchanged);
        }

        Ok(())
    }

    /// Write the signed file to the same directory as the input module as a specific name
    pub fn write_as<S>(&mut self, name: S) -> Result<&mut Self>
    where
        S: AsRef<str>,
    {
        let output = self
            .module
            .parent()
            .ok_or_else(|| Error::MissingParentDirectory {
                path: self.module.clone(),
            })?
            .join(name.as_ref());
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o755);
            file.set_permissions(perms)?;
        }

        file.write_all(&self.signed)?;
        Ok(self)
    }

    /// Write the signed file to an output path
    pub fn write<P>(&mut self, output: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output.as_ref())
            .map_err(|e| Error::OpenOutputFile {
                path: output.as_ref().to_path_buf(),
                source: e,
            })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file
                .metadata()
                .map_err(|e| Error::GetMetadata {
                    path: output.as_ref().to_path_buf(),
                    source: e,
                })?
                .permissions();
            perms.set_mode(0o755);
            file.set_permissions(perms)
                .map_err(|e| Error::SetPermissions {
                    path: output.as_ref().to_path_buf(),
                    source: e,
                })?;
        }

        file.write_all(&self.signed)
            .map_err(|e| Error::WriteOutputFile {
                path: output.as_ref().to_path_buf(),
                source: e,
            })?;

        file.flush()?;

        if !output.as_ref().exists() {
            return Err(Error::WriteOutputFile {
                path: output.as_ref().to_path_buf(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            });
        }

        Ok(self)
    }

    /// Get the raw signed data
    pub fn data(&self) -> Result<Vec<u8>> {
        if self.signed.is_empty() {
            Err(Error::ModuleNotSigned)
        } else {
            Ok(self.data.clone())
        }
    }
}

#[cfg(test)]
mod test {
    use super::Sign;
    use std::{env::var, path::PathBuf};

    #[cfg(debug_assertions)]
    const TARGET_DIR: &str = "debug";

    #[cfg(not(debug_assertions))]
    const TARGET_DIR: &str = "release";

    #[test]
    #[cfg(windows)]
    fn test_windows() {
        let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_dir = manifest_dir.parent().unwrap();
        let hello_world = workspace_dir
            .join("target")
            .join(TARGET_DIR)
            .join("hello_world.dll");
        let _signed = Sign::sign(hello_world).unwrap().data().unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn test_linux() {
        let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_dir = manifest_dir.parent().unwrap();
        let hello_world = workspace_dir
            .join("target")
            .join(TARGET_DIR)
            .join("libhello_world.so");
        println!("{:?}", hello_world);
        let _signed = Sign::new(hello_world).unwrap().data().unwrap();
    }
}
