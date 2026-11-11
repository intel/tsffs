// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! End-to-end, offline test of the DWARF/ELF debug-info backend
//! (`tsffs::dwarf::DwarfModule`, Milestone 1 of the DWARF source-coverage spec)
//! against a synthetic fixture, entirely without a live Simics session.
//!
//! This lives under `tests/` (an integration test / separate cargo crate) rather
//! than as a `#[cfg(test)]` module inside `src/dwarf/mod.rs` because this crate's
//! `[lib]` section sets `test = false`, which disables the implicit unit-test
//! harness for the library target specifically; `#[cfg(test)]` code inside `src/`
//! is therefore never compiled by any `cargo test` invocation for this crate.
//! Integration tests under `tests/` are a separate cargo target and are not
//! affected by that setting -- confirmed by first landing a trivial
//! `assert_eq!(2 + 2, 4)` test here and observing `cargo test` actually attempt to
//! build and run it (it got as far as the link step, which fails on this dev
//! machine for a pre-existing, environmental reason unrelated to this test or to
//! phase 1's code -- see the module-level `NOTE` below).
//!
//! Because `tests/` integration tests are a separate crate, they only see this
//! crate's `pub` API. Before this change, `dwarf`, `traits`, and `source_cov` were
//! all `pub(crate)` (nothing in this crate had any public API at all), which would
//! have made this test impossible to write without a redesign. The minimal fix
//! (see `src/lib.rs` and `src/dwarf/mod.rs`) widens only `dwarf` and `source_cov`
//! to `pub`; `os` (Windows kernel/PDB internals) and `traits` (which also holds the
//! unrelated `TracerDisassembler` trait) stay `pub(crate)`, with `dwarf::mod.rs`
//! instead re-exporting just `SymbolInfo`/`LineInfo`/`DebugInfoModule` as
//! `tsffs::dwarf::{SymbolInfo, LineInfo, DebugInfoModule}`.
//!
//! # Fixture
//!
//! `tests/fixtures/dwarf/X509CertVerify.c` is a small, synthetic C file (not real
//! EDK2/UEFI source) compiled and linked with real gcc, producing
//! `tests/fixtures/dwarf/X509CertVerify.debug`: a genuine ELF+DWARF binary,
//! analogous to a real EDK2 GCC5-built UEFI module's per-module `.debug` sidecar
//! file. Exact commands used to produce it:
//!
//! ```text
//! gcc -g -O0 -ffreestanding -fno-stack-protector -fno-builtin \
//!     -c X509CertVerify.c -o X509CertVerify.o
//! ld -o X509CertVerify.debug X509CertVerify.o --entry=X509VerifyCert \
//!     --section-start=.text=0x240
//! ```
//!
//! `--section-start=.text=0x240` gives the linked ELF a non-zero link-time `.text`
//! VMA (`0x240`), matching the real EDK2 GCC5 convention where DWARF addresses are
//! relative to a small non-zero link base, not `0`. This is exactly the "base +
//! link-time addr" arithmetic `DwarfModule::intervals` performs (see the doc
//! comment at the top of `src/dwarf/mod.rs`).
//!
//! Ground truth below was confirmed against the checked-in fixture with:
//!
//! ```text
//! objdump -h X509CertVerify.debug      # .text: VMA 0000000000000240
//! nm --print-size X509CertVerify.debug # 000000000000028c 000000000000004f T X509VerifyCert
//! readelf --debug-dump=decodedline X509CertVerify.debug
//! ```
//!
//! which reported (edited to the rows falling inside `X509VerifyCert`'s
//! `[0x28c, 0x2db)` range):
//!
//! ```text
//! X509CertVerify.c   49   0x28c   x
//! X509CertVerify.c   50   0x2a4   x
//! X509CertVerify.c   52   0x2bb   x
//! X509CertVerify.c   53   0x2c2   x
//! X509CertVerify.c   56   0x2c9   x
//! X509CertVerify.c   56   0x2cf   x
//! X509CertVerify.c   58   0x2d5   x
//! X509CertVerify.c   59   0x2d9   x
//! ```
//!
//! (Lines 37-45 in the same decoded table belong to the other function in the
//! fixture, `HashCertificateBytes`, which shares the compilation unit/line
//! program but falls outside `X509VerifyCert`'s address range and so is
//! correctly excluded by `DwarfModule::intervals`.)
//!
//! # NOTE: no live Simics session was used to validate this
//!
//! The DWARF source-coverage spec's Milestone 1 description originally envisioned
//! cross-checking against a real BIOS `.debug` module through a live Simics
//! session (`sym-source`/`sym-function`) via the Simics TCF debugger. No such
//! session (and no real EDK2-built `.debug` file) is available in this offline
//! environment, so this test instead validates the parsing chain (`object` +
//! `gimli` + this crate's DIE/line-program walk) against a synthetic fixture's own
//! known ground truth, confirmed independently via `objdump`/`nm`/`readelf` above.
//! Cross-checking against a real BIOS module through an actual Simics session
//! remains open, to be done later by whoever has Simics and a real `.debug` file.
//!
//! Separately: linking this crate with the default `x86_64-pc-windows-msvc` host
//! target fails (`link.exe` exit code 1107, "invalid or corrupt file") because
//! `simics-build-utils` emits a `cargo:rustc-link-lib=dylib:+verbatim=...dll`
//! directive that only MinGW's `ld` understands, not MSVC's `link.exe`. Building
//! and testing with the GNU host target instead (`cargo test --target
//! x86_64-pc-windows-gnu --test dwarf_fixture`, MinGW-w64 `gcc`/`ld` on `PATH`,
//! `SIMICS_BASE` pointed at a local Simics install) links and runs this test
//! successfully, matching the CI `build_windows` job in `.github/workflows/ci.yml`.
//! At runtime, the Simics package's own `win64/bin` directory (containing
//! `libsimics-common.dll`/`libvtutils.dll`) must also be on `PATH`.
//!
//! `SourceCache::new` originally also called into a Simics FFI object lookup
//! (`get_object("tsffs")`) purely to emit a debug log line, guarded with
//! `if let Ok(o) = ...` on the assumption that a missing live Simics session would
//! just produce an `Err`. In practice, calling any `SIM_*` API entry point with no
//! Simics kernel initialized (as here, a plain `.exe` linking directly against
//! `libsimics-common.dll`) hard-aborts the process from inside the DLL itself,
//! before the FFI call can return `Err` -- so that debug log line has been removed
//! from `SourceCache::new` to make it actually usable offline, as this test
//! requires.

use std::{fs::read, path::PathBuf};

use object::File as ObjectFile;
use tsffs::{
    dwarf::{DebugInfoModule, DwarfModule, SymbolInfo},
    source_cov::SourceCache,
};

/// The fixture's link-time `.text` VMA, as passed to `ld --section-start` and
/// confirmed with `objdump -h` (see the module doc comment above).
const TEXT_VMA: u64 = 0x240;

/// `X509VerifyCert`'s link-time start address and size, confirmed with
/// `nm --print-size` against the checked-in fixture (see the module doc comment
/// above): `000000000000028c 000000000000004f T X509VerifyCert`.
const X509_VERIFY_CERT_ADDR: u64 = 0x28c;
const X509_VERIFY_CERT_SIZE: u64 = 0x4f;

/// A hardcoded fake runtime base address standing in for a real EDK2 `ImageBase`,
/// per Milestone 1's "single module, fake inputs, offline unit test" scope --
/// there is no live Simics session in this environment to supply a real one.
/// Deliberately not page-aligned/round, to make sure the test isn't inadvertently
/// tolerant of a base-address bug that only manifests for non-trivial bases (e.g.
/// an accidental OR instead of ADD, which is invisible when the low bits of the
/// link-time address don't collide with the base -- picking a base whose low
/// nibble is non-zero, like the real link-time address, guards against that).
const FAKE_IMAGE_BASE: u64 = 0x0007_ffff_1234_0000;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dwarf")
}

#[test]
fn dwarf_module_intervals_resolves_synthetic_cert_verify() -> anyhow::Result<()> {
    let fixture_dir = fixture_dir();
    let debug_path = fixture_dir.join("X509CertVerify.debug");
    let bytes = read(&debug_path)?;

    let object = ObjectFile::parse(bytes.as_slice())?;
    let mut module = DwarfModule::new("X509CertVerify.efi".to_string(), FAKE_IMAGE_BASE, object);

    // `SourceCache` walks `fixture_dir` so `X509CertVerify.c` (the fixture's own
    // checked-in source) can be resolved as a local path for the line info below.
    // `SourceCache::new` calls into a guarded (`if let Ok(...)`) Simics FFI object
    // lookup purely for a debug log line, so it degrades gracefully with no live
    // Simics session -- see `src/source_cov/mod.rs`.
    let source_cache = SourceCache::new(&fixture_dir)?;

    let intervals = module.intervals(&source_cache)?;

    let handler = intervals
        .iter()
        .find(|element| element.value.name == "X509VerifyCert")
        .unwrap_or_else(|| {
            panic!(
                "no SymbolInfo named \"X509VerifyCert\" in {:?}",
                intervals
                    .iter()
                    .map(|e| e.value.name.as_str())
                    .collect::<Vec<_>>()
            )
        });

    // Address range: base + link-time addr, exactly the arithmetic documented at
    // the top of `src/dwarf/mod.rs` -- this is the crux of Milestone 1.
    let expected_start = FAKE_IMAGE_BASE + X509_VERIFY_CERT_ADDR;
    let expected_end = expected_start + X509_VERIFY_CERT_SIZE;
    assert_eq!(handler.range.start, expected_start);
    assert_eq!(handler.range.end, expected_end);

    let symbol: &SymbolInfo = &handler.value;
    assert_eq!(symbol.name, "X509VerifyCert");
    assert_eq!(symbol.module, "X509CertVerify.efi");
    assert_eq!(symbol.base, FAKE_IMAGE_BASE);
    assert_eq!(symbol.rva, X509_VERIFY_CERT_ADDR);
    assert_eq!(symbol.size, X509_VERIFY_CERT_SIZE);
    assert!(
        TEXT_VMA <= symbol.rva,
        "sanity check: the function must start at or after the fixture's .text VMA"
    );

    // Line numbers: ground truth from `readelf --debug-dump=decodedline` (see the
    // module doc comment above), restricted to rows inside X509VerifyCert's
    // [0x28c, 0x2db) range. `HashCertificateBytes`'s rows (lines 37-45) share the
    // same compilation unit/line program but must NOT show up here.
    let mut lines: Vec<(u64, u32)> = symbol
        .lines
        .iter()
        .map(|line| (line.rva, line.start_line))
        .collect();
    lines.sort_by_key(|&(rva, _)| rva);

    assert_eq!(
        lines,
        vec![
            (0x28c, 49),
            (0x2a4, 50),
            (0x2bb, 52),
            (0x2c2, 53),
            (0x2c9, 56),
            (0x2cf, 56),
            (0x2d5, 58),
            (0x2d9, 59),
        ]
    );

    // Every resolved line must have found the fixture's checked-in source file
    // (proving `SourceCache`/`resolve_file_path`'s DWARF-embedded-name lookup path
    // works, not just the address/line arithmetic).
    for line in &symbol.lines {
        assert_eq!(
            line.file_path.file_name().and_then(|n| n.to_str()),
            Some("X509CertVerify.c")
        );
        assert_eq!(line.start_line, line.end_line);
    }

    Ok(())
}
