use std::{
    collections::HashMap,
    fs::read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use md5::compute;
use pdb::{FileChecksum, FileInfo};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use walkdir::WalkDir;

use crate::util::path_suffix_index::PathSuffixIndex;

#[derive(Debug, Clone, Default)]
pub struct SourceCache {
    // Path-component-suffix lookup, e.g. matching a DWARF/PDB-recorded source path
    // against a locally-checked-out file by its longest matching path suffix.
    // Extracted into `PathSuffixIndex` (`crate::util::path_suffix_index`), which is
    // also used by the UEFI module debug-info resolver (`crate::uefi`), so the two
    // don't duplicate this logic.
    suffix_index: PathSuffixIndex,
    md5_lookup: HashMap<Vec<u8>, PathBuf>,
    sha1_lookup: HashMap<Vec<u8>, PathBuf>,
    sha256_lookup: HashMap<Vec<u8>, PathBuf>,
}

impl SourceCache {
    pub fn new<P>(src_dir: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut suffix_index = PathSuffixIndex::new();
        let mut md5_lookup = HashMap::new();
        let mut sha1_lookup = HashMap::new();
        let mut sha256_lookup = HashMap::new();

        let file_paths = WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<_>>();

        for path in &file_paths {
            let contents = read(path)?;
            let md5 = compute(&contents).0.to_vec();
            let sha1 = Sha1::digest(&contents).to_vec();
            let sha256 = Sha256::digest(&contents).to_vec();
            md5_lookup.insert(md5, path.clone());
            sha1_lookup.insert(sha1, path.clone());
            sha256_lookup.insert(sha256, path.clone());
            suffix_index.insert(path);
        }

        Ok(Self {
            suffix_index,
            md5_lookup,
            sha1_lookup,
            sha256_lookup,
        })
    }

    pub fn lookup_file_name_components(&self, file_name: &str) -> Option<&Path> {
        self.suffix_index.lookup_str(file_name)
    }

    pub fn lookup_pdb(&self, file_info: &FileInfo, file_name: &str) -> Result<Option<&Path>> {
        Ok(match file_info.checksum {
            FileChecksum::None => self.lookup_file_name_components(file_name),
            FileChecksum::Md5(m) => self
                .md5_lookup
                .get(m)
                .map(|p| p.as_path())
                .or_else(|| self.lookup_file_name_components(file_name)),
            FileChecksum::Sha1(s1) => self
                .sha1_lookup
                .get(s1)
                .map(|p| p.as_path())
                .or_else(|| self.lookup_file_name_components(file_name)),
            FileChecksum::Sha256(s256) => self
                .sha256_lookup
                .get(s256)
                .map(|p| p.as_path())
                .or_else(|| self.lookup_file_name_components(file_name)),
        })
    }

    /// Look up a source file referenced from a DWARF line number program, mirroring
    /// `lookup_pdb`. DWARF5 line-number program headers may carry an optional
    /// `DW_LNCT_MD5` checksum entry per file (`gimli`'s `FileEntry::md5`, valid only
    /// when `LineProgramHeader::file_has_md5` returns `true`); when present, we try
    /// that checksum against the same MD5 table populated by `SourceCache::new`
    /// (which hashes every candidate source file with MD5/SHA1/SHA256 up front).
    /// DWARF doesn't specify SHA1/SHA256 file checksums, so there's nothing to try
    /// there. If there's no MD5 (DWARF <= 4, or a DWARF5 producer that omitted it,
    /// e.g. some EDK2 GCC5 builds), we fall back to the same format-agnostic
    /// path-suffix lookup PDB uses.
    pub fn lookup_dwarf(&self, md5: Option<&[u8; 16]>, file_name: &str) -> Result<Option<&Path>> {
        Ok(match md5 {
            Some(m) => self
                .md5_lookup
                .get(m.as_slice())
                .map(|p| p.as_path())
                .or_else(|| self.lookup_file_name_components(file_name)),
            None => self.lookup_file_name_components(file_name),
        })
    }
}
