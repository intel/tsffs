// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! DWARF/ELF debug info backend.
//!
//! UEFI/SMM BIOS modules built with the EDK2 GCC5 toolchain ship debug info as DWARF
//! embedded in an ELF `.debug` sidecar file, unlike Windows kernel/PE modules, which
//! ship Microsoft PDB (see `crate::os::windows::debug_info`). [`DwarfModule`] parses
//! that DWARF/ELF debug info and implements the shared [`DebugInfoModule`] trait
//! (`crate::traits::DebugInfoModule`), so callers get the exact same
//! [`SymbolInfo`]/[`LineInfo`] output regardless of whether a module's debug info
//! came from a PDB or from DWARF/ELF.
//!
//! # Address translation
//!
//! DWARF line-table and DIE addresses in these per-module ELF files are link-time
//! addresses relative to the module's own image (the linked `.text` VMA is commonly a
//! small non-zero value such as `0x240`, not `0`). The runtime address for any DWARF
//! address `a` in a module loaded at `base` (its EDK2 `ImageBase`) is simply
//! `base + a` -- a flat addition, exactly analogous to the PDB backend's
//! `self.base + rva`. No special-case subtraction of a link base is needed.
//!
//! # Open questions / explicitly out of scope for this milestone
//!
//! - `addr2line` (gimli's usual companion crate for point address lookups) is
//!   intentionally not used here. Whether it's viable for *proactive* enumeration of
//!   every function/line (needed to seed coverage denominators, mirroring what
//!   `Module`/`ProcessModule::intervals` do for PDB) is an open question -- its public
//!   API is oriented around "what function/line is at this address", not "list all
//!   functions/lines". Until that's resolved, this module hand-rolls the DIE and line
//!   program walk directly on `gimli`.
//! - How a `DwarfModule` actually gets constructed at runtime (Simics-side UEFI/SMM
//!   module discovery: finding the loaded module's `ImageBase` and locating its
//!   `.debug` file) and the trigger question (there is no CR3-equivalent signal for
//!   SMM, unlike `Tsffs::on_control_register_write_windows_symcov` for Windows) are
//!   out of scope here.
//! - Performance at scale (~247 modules) is not addressed; `intervals` below does a
//!   straightforward per-unit, per-subprogram walk.

use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use gimli::{
    AttributeValue, DebuggingInformationEntry, DwarfSections, EndianSlice, LineProgramHeader,
    Reader, RunTimeEndian, SectionId, Unit, UnitRef,
};
use intervaltree::Element;
use object::{Object, ObjectSection};

use crate::source_cov::SourceCache;

// Re-exported (rather than left as a plain `use`) so that `tests/dwarf_fixture.rs`
// -- a separate crate, since it's an integration test -- can name these as
// `tsffs::dwarf::{SymbolInfo, LineInfo, DebugInfoModule}` without requiring all of
// `crate::os` (Windows kernel/PDB internals) or `crate::traits` (which also holds
// the unrelated `TracerDisassembler` trait) to be made public too.
pub use crate::os::windows::debug_info::{LineInfo, SymbolInfo};
pub use crate::traits::DebugInfoModule;

/// A UEFI/SMM module's DWARF/ELF debug info, resolved into the same
/// [`SymbolInfo`]/[`LineInfo`] shape the PDB backend produces.
///
/// Unlike the PDB backend (`crate::os::windows::debug_info::DebugInfo`), which owns
/// the file handle it parses, `DwarfModule` is constructed from an already-parsed
/// [`object::File`] -- the caller is responsible for reading the module's `.debug`
/// ELF file into a buffer that outlives this module and parsing it with
/// `object::File::parse`.
#[derive(Debug)]
pub struct DwarfModule<'data> {
    /// The runtime base address (EDK2 `ImageBase`) this module is loaded at in guest
    /// memory. Every DWARF address in `object` is a link-time address relative to the
    /// module's own image and is translated to a runtime address via `base + addr`
    /// (see the module-level docs above).
    pub base: u64,
    /// The name of the module (e.g. its `.efi`/driver name), used to populate
    /// `SymbolInfo::module`.
    pub full_name: String,
    /// The already-parsed ELF file containing the DWARF debug info.
    object: object::File<'data>,
}

impl<'data> DwarfModule<'data> {
    /// Construct a new DWARF-backed debug info module from an already-parsed ELF
    /// file, treating it as loaded at `base` in guest memory.
    pub fn new(full_name: String, base: u64, object: object::File<'data>) -> Self {
        Self {
            base,
            full_name,
            object,
        }
    }

    /// Load the raw contents of a DWARF section from `self.object`, decompressing it
    /// if necessary. Returns an empty slice for sections that aren't present, which is
    /// how `gimli::Dwarf::load` expects missing sections to be reported.
    fn load_section(&self, id: SectionId) -> Result<Cow<'data, [u8]>, object::Error> {
        Ok(match self.object.section_by_name(id.name()) {
            Some(section) => section.uncompressed_data()?,
            None => Cow::Borrowed(&[][..]),
        })
    }

    /// Render a DWARF line-program file entry's directory + file name into a single
    /// path-like string suitable as the fallback lookup key for
    /// `SourceCache::lookup_dwarf` (i.e. the DWARF-embedded name, not a resolved local
    /// path).
    fn render_file_name<R: Reader>(
        unit_ref: UnitRef<R>,
        file: &gimli::FileEntry<R, R::Offset>,
        header: &LineProgramHeader<R, R::Offset>,
    ) -> Result<String> {
        let mut components = Vec::new();

        // Directory index 0 is defined to mean the compilation directory, which we
        // don't have a reliable local analog for, so we only record explicit
        // subdirectories here. `SourceCache::lookup_file_name_components` matches by
        // path suffix, so omitting the compilation directory prefix does not affect
        // correctness.
        if file.directory_index() != 0 {
            if let Some(directory) = file.directory(header) {
                let directory = unit_ref
                    .attr_string(directory)
                    .map_err(|e| anyhow!("Failed to read DWARF directory name: {e}"))?
                    .to_string_lossy()
                    .map_err(|e| anyhow!("Failed to decode DWARF directory name: {e}"))?
                    .into_owned();
                components.push(directory);
            }
        }

        let name = unit_ref
            .attr_string(file.path_name())
            .map_err(|e| anyhow!("Failed to read DWARF file name: {e}"))?
            .to_string_lossy()
            .map_err(|e| anyhow!("Failed to decode DWARF file name: {e}"))?
            .into_owned();
        components.push(name);

        Ok(components.join("/"))
    }

    /// Resolve a DWARF line-program file entry (by index) to a local source file path
    /// via `source_cache`, trying the DWARF5 `DW_LNCT_MD5` checksum first (mirroring
    /// `SourceCache::lookup_pdb`'s use of the PDB-embedded checksum) and falling back
    /// to path-suffix matching on the DWARF-embedded file name.
    fn resolve_file_path<R: Reader>(
        unit_ref: UnitRef<R>,
        header: &LineProgramHeader<R, R::Offset>,
        file_index: u64,
        source_cache: &SourceCache,
    ) -> Option<PathBuf> {
        let file = header.file(file_index)?;
        let rendered = Self::render_file_name(unit_ref, file, header).ok()?;
        let md5 = header.file_has_md5().then(|| *file.md5());

        source_cache
            .lookup_dwarf(md5.as_ref(), &rendered)
            .ok()
            .flatten()
            .map(|p| p.to_path_buf())
    }

    /// Walk every `DW_TAG_subprogram` DIE in `unit_ref`'s compilation unit, resolving
    /// each one's address range and source lines (via its line number program) into
    /// [`SymbolInfo`] with module-relative (link-time) addresses in `rva`/`LineInfo::rva`.
    fn unit_symbols<R: Reader>(
        &self,
        unit_ref: UnitRef<R>,
        all_units: &[Unit<R>],
        source_cache: &SourceCache,
    ) -> Result<Vec<SymbolInfo>> {
        let Some(incomplete_line_program) = unit_ref.line_program.clone() else {
            // No line number program for this unit (e.g. a unit with no debug lines);
            // there is nothing to resolve lines against, so skip it entirely.
            return Ok(Vec::new());
        };

        // Build a flat, address-sorted list of line rows for the unit, and (lazily,
        // memoized by file index) resolve each referenced file to a local source path
        // up front, since many rows share the same file.
        let mut file_paths: HashMap<u64, Option<PathBuf>> = HashMap::new();
        let mut rows: Vec<(u64, u64, u32, bool)> = Vec::new();

        let mut line_rows = incomplete_line_program.rows();

        while let Some((header, row)) = line_rows
            .next_row()
            .map_err(|e| anyhow!("Failed to read DWARF line program row: {e}"))?
        {
            let file_index = row.file_index();

            file_paths.entry(file_index).or_insert_with(|| {
                Self::resolve_file_path(unit_ref, header, file_index, source_cache)
            });

            rows.push((
                row.address(),
                file_index,
                row.line().map(|line| line.get() as u32).unwrap_or(0),
                row.end_sequence(),
            ));
        }

        rows.sort_by_key(|row| row.0);

        let mut symbols = Vec::new();

        let mut cursor = unit_ref.entries();

        while let Some(entry) = cursor
            .next_dfs()
            .map_err(|e| anyhow!("Failed to walk DWARF DIE tree: {e}"))?
        {
            if entry.tag() != gimli::DW_TAG_subprogram {
                continue;
            }

            let Some((low, high)) = Self::subprogram_range(unit_ref, entry)? else {
                // No low_pc/high_pc/ranges on this DIE -- it's a declaration or
                // abstract instance root with no code of its own, not a concrete
                // subprogram we can key an interval on.
                continue;
            };

            if high <= low {
                continue;
            }

            let Some((name, name_unit_ref)) = Self::resolve_name(entry, unit_ref, all_units) else {
                // No name resolvable via DW_AT_name, DW_AT_abstract_origin, or
                // DW_AT_specification -- not a concrete named subprogram we can key
                // symbol info on.
                continue;
            };

            let name = name_unit_ref
                .attr_string(name)
                .map_err(|e| anyhow!("Failed to read DWARF subprogram name: {e}"))?
                .to_string_lossy()
                .map_err(|e| anyhow!("Failed to decode DWARF subprogram name: {e}"))?
                .into_owned();

            let lines = Self::lines_in_range(&rows, low, high, &file_paths);

            symbols.push(SymbolInfo::new(
                low,
                self.base,
                high - low,
                name,
                self.full_name.clone(),
                lines,
            ));
        }

        Ok(symbols)
    }

    /// Resolve a DIE's effective `DW_AT_name` attribute for stringification, following
    /// `DW_AT_abstract_origin` (falling back to `DW_AT_specification`) to the referenced
    /// DIE when `entry` has no direct `DW_AT_name` of its own. Returns the resolved
    /// `DW_AT_name` attribute value together with the `UnitRef` of the unit that DIE
    /// actually lives in -- needed to correctly stringify forms such as
    /// `DW_FORM_strx` (relative to a per-unit string-offsets base); not needed for the
    /// common `DW_FORM_strp` (absolute `.debug_str` offset) case seen in practice, but
    /// cheap to keep correct either way.
    ///
    /// GCC5/EDK2 universally emits the standard "abstract instance / concrete
    /// instance" DWARF split for every real function: the concrete DIE (the one with
    /// `DW_AT_low_pc`/`DW_AT_high_pc`, i.e. `entry` here) has `DW_AT_abstract_origin`
    /// pointing at a separate DIE that carries the real `DW_AT_name`, instead of a
    /// direct name on itself. Confirmed against 914 real EDK2 GCC5 `.debug` files,
    /// that reference is universally `DW_FORM_ref_addr` (a raw `.debug_info`-section
    /// offset, decoded by gimli as `AttributeValue::DebugInfoRef`), rather than a same-unit
    /// `AttributeValue::UnitRef` -- GCC emits each "abstract instance" DIE once, in
    /// whichever compilation unit first defines it, and references it from every other
    /// unit that inlines/instantiates it, so the referenced DIE is frequently in a
    /// *different* CU than `entry`. Hence resolving it requires searching
    /// `all_units` (every unit in this module, pre-parsed by `intervals`), not just
    /// `unit_ref`'s own unit -- `AttributeValue::UnitRef` is still handled too, in case
    /// some DIEs reference same-unit offsets instead.
    fn resolve_name<'u, R: Reader>(
        entry: &DebuggingInformationEntry<R>,
        unit_ref: UnitRef<'u, R>,
        all_units: &'u [Unit<R>],
    ) -> Option<(AttributeValue<R>, UnitRef<'u, R>)> {
        if let Some(name) = entry.attr_value(gimli::DW_AT_name) {
            return Some((name, unit_ref));
        }

        let origin = entry
            .attr_value(gimli::DW_AT_abstract_origin)
            .or_else(|| entry.attr_value(gimli::DW_AT_specification))?;

        match origin {
            AttributeValue::UnitRef(offset) => {
                let origin_entry = unit_ref.entry(offset).ok()?;
                let name = origin_entry.attr_value(gimli::DW_AT_name)?;
                Some((name, unit_ref))
            }
            AttributeValue::DebugInfoRef(offset) => all_units.iter().find_map(|candidate| {
                let local_offset = offset.to_unit_offset(&candidate.header)?;
                let origin_entry = candidate.entry(local_offset).ok()?;
                let name = origin_entry.attr_value(gimli::DW_AT_name)?;
                Some((name, candidate.unit_ref(unit_ref.dwarf)))
            }),
            _ => None,
        }
    }

    /// Compute the `[low, high)` link-time address range of a DIE from its
    /// `DW_AT_low_pc`/`DW_AT_high_pc`/`DW_AT_ranges` attributes, aggregating over all
    /// ranges if there is more than one (e.g. for a DIE split into disjoint pieces).
    /// Returns `None` if the DIE has no address range at all (e.g. a declaration).
    fn subprogram_range<R: Reader>(
        unit_ref: UnitRef<R>,
        entry: &DebuggingInformationEntry<R>,
    ) -> Result<Option<(u64, u64)>> {
        let mut ranges = unit_ref
            .die_ranges(entry)
            .map_err(|e| anyhow!("Failed to read DWARF DIE address ranges: {e}"))?;

        let mut result: Option<(u64, u64)> = None;

        while let Some(range) = ranges
            .next()
            .map_err(|e| anyhow!("Failed to read next DWARF address range: {e}"))?
        {
            result = Some(match result {
                Some((low, high)) => (low.min(range.begin), high.max(range.end)),
                None => (range.begin, range.end),
            });
        }

        Ok(result)
    }

    /// Collect the `LineInfo`s for every non-synthetic (`line != 0`), non-end-of-sequence
    /// row in `rows` (sorted by address, as built by `unit_symbols`) that falls within
    /// `[low, high)`, sizing each line by the address of the following row, resolving
    /// its file via the memoized `file_paths` (skipping rows whose file didn't resolve
    /// to a local path).
    fn lines_in_range(
        rows: &[(u64, u64, u32, bool)],
        low: u64,
        high: u64,
        file_paths: &HashMap<u64, Option<PathBuf>>,
    ) -> Vec<LineInfo> {
        rows.iter()
            .enumerate()
            .filter(|(_, (address, _, line, end_sequence))| {
                !end_sequence && *line != 0 && *address >= low && *address < high
            })
            .filter_map(|(index, &(address, file_index, line, _))| {
                let file_path = file_paths.get(&file_index)?.clone()?;

                let next_address = rows
                    .get(index + 1)
                    .map(|next| next.0)
                    .unwrap_or(high)
                    .min(high);

                Some(LineInfo {
                    rva: address,
                    size: next_address.saturating_sub(address).max(1) as u32,
                    file_path,
                    start_line: line,
                    end_line: line,
                })
            })
            .collect()
    }
}

impl<'data> DebugInfoModule for DwarfModule<'data> {
    /// Resolve every `DW_TAG_subprogram` in this module's DWARF debug info into
    /// interval-tree elements keyed by `[base + low_pc, base + high_pc)`, with source
    /// lines resolved through `source_cache`. Mirrors
    /// `Module::intervals`/`ProcessModule::intervals` for the PDB backend.
    fn intervals(&mut self, source_cache: &SourceCache) -> Result<Vec<Element<u64, SymbolInfo>>> {
        let endian = if self.object.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };

        let dwarf_sections = DwarfSections::load(|id| self.load_section(id))
            .map_err(|e| anyhow!("Failed to load DWARF sections: {e}"))?;
        let dwarf = dwarf_sections.borrow(|section| EndianSlice::new(section, endian));

        // Pre-parse every compilation unit up front, rather than one at a time as
        // they're walked below, so that DW_AT_abstract_origin/DW_AT_specification
        // references that cross compilation-unit boundaries (see `resolve_name`) can
        // be resolved against *any* unit, not just whichever one is currently being
        // walked.
        let mut all_units = Vec::new();
        let mut unit_headers = dwarf.units();

        while let Some(header) = unit_headers
            .next()
            .map_err(|e| anyhow!("Failed to read next DWARF unit header: {e}"))?
        {
            all_units.push(
                dwarf
                    .unit(header)
                    .map_err(|e| anyhow!("Failed to parse DWARF unit: {e}"))?,
            );
        }

        let mut symbols = Vec::new();

        for unit in &all_units {
            let unit_ref = unit.unit_ref(&dwarf);

            symbols.extend(self.unit_symbols(unit_ref, &all_units, source_cache)?);
        }

        Ok(symbols
            .into_iter()
            .map(|s| (self.base + s.rva..self.base + s.rva + s.size, s).into())
            .collect())
    }
}

// NOTE: There is intentionally no `#[cfg(test)] mod test` here. `[lib] test = false`
// in Cargo.toml disables the implicit unit-test harness for *this* (library) target,
// so `#[cfg(test)]` code in this file is never compiled by any `cargo test`
// invocation. The offline end-to-end test of `DwarfModule::intervals` (against a
// synthetic ELF+DWARF fixture built with WSL gcc) lives in `tests/dwarf_fixture.rs`
// instead, which is a separate cargo target/crate not affected by `test = false`.
