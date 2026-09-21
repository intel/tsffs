// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    os::windows::debug_info::SymbolInfo,
    source_cov::SourceCache,
    tracer::{CmpExpr, CmpType},
};
use anyhow::Result;
use intervaltree::Element;

/// Trait for disassemblers of various architectures to implement to permit branch
/// and compare tracing
pub trait TracerDisassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()>;
    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String>;
    fn last_was_control_flow(&self) -> bool;
    fn last_was_call(&self) -> bool;
    fn last_was_ret(&self) -> bool;
    fn last_was_cmp(&self) -> bool;
    fn cmp(&self) -> Vec<CmpExpr>;
    fn cmp_type(&self) -> Vec<CmpType>;
}

/// Trait implemented by debug-info backends (e.g. Windows PDB, DWARF/ELF) which can
/// resolve the symbols and source lines of a loaded module into lookup intervals
/// keyed by absolute runtime address range (`base + rva .. base + rva + size`).
///
/// This allows callers to build a single interval tree covering modules backed by
/// different debug info formats (PDB for Windows kernel/PE modules, DWARF for
/// UEFI/SMM ELF modules, ...) without caring which backend produced each module's
/// symbols.
pub trait DebugInfoModule {
    /// Resolve this module's procedures/subprograms and their source lines into
    /// interval-tree elements, using `source_cache` to map embedded source file
    /// references (by checksum, falling back to path suffix matching) to files on
    /// the local filesystem.
    fn intervals(&mut self, source_cache: &SourceCache) -> Result<Vec<Element<u64, SymbolInfo>>>;
}
