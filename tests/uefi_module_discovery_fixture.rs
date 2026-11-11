// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! End-to-end, offline test of UEFI module discovery's milestone-scope steps 1-2
//! (`tsffs::uefi::{parse_module_list, UefiOsInfo}`, UCOV-M2) against a synthetic
//! fixture, entirely without a live Simics session -- mirroring
//! `tests/dwarf_fixture.rs`'s pattern on the sibling DWARF milestone branch
//! (`feat/dwarf-source-coverage-ucov-m1`).
//!
//! This lives under `tests/` (an integration test / separate cargo crate) rather
//! than as a `#[cfg(test)]` module inside `src/uefi/mod.rs` for the same reason as
//! that file: this crate's `[lib]` section sets `test = false`, which disables the
//! implicit unit-test harness for the library target, so `#[cfg(test)]` code
//! inside `src/` is never compiled by `cargo test` for this crate. Integration
//! tests under `tests/` are a separate cargo target and unaffected by that
//! setting. Being a separate crate also means this file only sees `tsffs`'s `pub`
//! API -- `src/lib.rs` widens `uefi` to `pub` (from `pub(crate)`) for exactly this
//! reason, same as `dwarf`/`source_cov` on the DWARF branch.
//!
//! # No live Simics session was used, but the row shape itself is real
//!
//! There is no BIOS image, boot, or live Simics session available in this offline
//! environment, so this test still can't drive a real `run_command` call -- it
//! tests `parse_module_list` against a *hand-constructed* fake `AttrValueType`
//! rather than one actually returned by Simics. But unlike the superseded
//! `list-modules`-based design (whose row shape was an unconfirmed assumption),
//! the 7-element row shape used below is not a guess: it matches a real, live
//! `tracker_obj->maps` dump captured in a live test session (real Simics
//! 6.0.189 session, real checkpoint past DXE dispatch, 68 real loaded UEFI
//! modules) -- see `src/uefi/mod.rs`'s module doc comment ("Confirmed shape of
//! `tracker_obj->maps`' return value") for the exact shape and a real captured
//! example row.
//!
//! # Fixture
//!
//! The fixture models 6 rows:
//!
//! - `PeiCore.efi`: one unique module with a full embedded path, resolved via
//!   primary path-suffix matching.
//! - `BootScriptExecutorDxe.efi`: the real observed duplicate-name case
//!   (re-investigated under `tracker_obj->maps`) -- two loaded instances, two
//!   distinct base addresses, but the **identical** embedded path for both (the
//!   same build loaded twice, not two different binaries). Confirms this
//!   resolves both addresses to the same correct local file with **no**
//!   ambiguity warning, since a real matching path makes it not actually
//!   ambiguous.
//! - `AcpiVTD.efi`: a **fabricated** genuinely-different-path duplicate (two
//!   different, fabricated `PkgA`/`PkgB` package subdirectories, same basename)
//!   -- proves path-suffix matching still correctly disambiguates *real*
//!   ambiguity when it exists, which the `BootScriptExecutorDxe.efi` case above
//!   does not exercise (its two instances share one path, so a resolver that
//!   ignored the path entirely would pass that case by accident).
//! - An unknown/unresolved module with no embedded path at all
//!   (`AttrValueType::Nil`, matching the real capture -- see
//!   `fixture_attr_value`'s doc comment), confirming `parse_module_list` names
//!   it [`tsffs::uefi::UNKNOWN_MODULE_NAME`] without panicking or misparsing,
//!   and that `UefiOsInfo::resolve` fails that one module gracefully (a clear
//!   `Err`, not a panic) rather than guessing.

use std::{
    collections::HashMap,
    fs::{create_dir_all, write},
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use simics::AttrValueType;
use tempfile::tempdir;
use tracing_subscriber::fmt::MakeWriter;
use tsffs::uefi::{parse_module_list, UefiOsInfo, UNKNOWN_MODULE_NAME};

/// One row of the fixture, in the confirmed real `tracker_obj->maps` shape:
/// `[loaded_address, loaded_size, <bool>, adjusted_address, adjusted_size,
/// <bool>, full_path_string]`. `embedded_path` is `None` to model a genuinely
/// pathless ("unknown module") row.
struct FixtureRow {
    base: u64,
    size: u64,
    embedded_path: Option<String>,
}

/// The fixture's common embedded build-machine path prefix, matching the real
/// structure observed in a live tracker dump (see `src/uefi/mod.rs`'s
/// module doc comment for the real captured example this mirrors).
const PREFIX: &str = "/home/user/bios-x58i/project/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/X64";

/// Build the fixture's 6 rows:
/// - one unique module (`PeiCore.efi`)
/// - the real observed duplicate-name case (`BootScriptExecutorDxe.efi`), both
///   instances sharing one identical embedded path
/// - a fabricated genuinely-different-path duplicate (`AcpiVTD.efi`, under
///   fabricated `PkgA`/`PkgB` subdirectories)
/// - one genuinely pathless ("unknown module") row
fn fixture_rows() -> Vec<FixtureRow> {
    vec![
        FixtureRow {
            base: 0x0000_0000_0082_0000,
            size: 0x9000,
            embedded_path: Some(format!(
                "{PREFIX}/MdeModulePkg/Core/Pei/PeiMain/DEBUG/PeiCore.efi"
            )),
        },
        // Real observed duplicate-name case: two loaded instances, two base
        // addresses, but the identical embedded path for both.
        FixtureRow {
            base: 0x0000_0000_00d0_0000,
            size: 0x1_0000,
            embedded_path: Some(format!(
                "{PREFIX}/MdeModulePkg/Universal/Variable/RuntimeDxe/BootScriptExecutorDxe/DEBUG/BootScriptExecutorDxe.efi"
            )),
        },
        FixtureRow {
            base: 0x0000_0000_00e0_0000,
            size: 0x1_0100,
            embedded_path: Some(format!(
                "{PREFIX}/MdeModulePkg/Universal/Variable/RuntimeDxe/BootScriptExecutorDxe/DEBUG/BootScriptExecutorDxe.efi"
            )),
        },
        // Fabricated genuinely-different-path duplicate: same basename, two
        // different package subdirectories, to prove suffix-matching still
        // disambiguates real ambiguity when it exists.
        FixtureRow {
            base: 0x0000_0000_0700_0000,
            size: 0x4000,
            embedded_path: Some(format!("{PREFIX}/PkgA/Feature/AcpiVTD/DEBUG/AcpiVTD.efi")),
        },
        FixtureRow {
            base: 0x0000_0000_0710_0000,
            size: 0x4200,
            embedded_path: Some(format!("{PREFIX}/PkgB/Feature/AcpiVTD/DEBUG/AcpiVTD.efi")),
        },
        // Genuinely pathless row: a real "unknown"/unresolved module.
        FixtureRow {
            base: 0x0000_0000_0900_0000,
            size: 0x1000,
            embedded_path: None,
        },
    ]
}

/// Build the fake `AttrValueType` shape `tracker_obj->maps` is confirmed (see
/// `src/uefi/mod.rs`'s module doc comment) to return for a set of fixture rows: a
/// `List` of 7-element `List`s, `[loaded_address, loaded_size, <bool>,
/// adjusted_address, adjusted_size, <bool>, full_path_string]`. `adjusted_address`
/// is fabricated equal to `loaded_address` and `adjusted_size` equal to
/// `loaded_size`, and both booleans fabricated `true`, since this module doesn't
/// read those fields at all (see the module doc comment). A `None`
/// `embedded_path` becomes `AttrValueType::Nil`, not `AttrValueType::String(String::new())`
/// -- validated live against the real, full 68-row capture: the one
/// genuinely pathless row's Python value is `None`, which converts to
/// `AttrValueType::Nil` via `simics::AttrValueType::from(AttrValue)`'s `is_nil()`
/// check. An earlier revision of this fixture used an empty string instead,
/// which the real capture showed does not match what Simics actually returns.
fn fixture_attr_value(rows: &[FixtureRow]) -> AttrValueType {
    AttrValueType::List(
        rows.iter()
            .map(|row| {
                AttrValueType::List(vec![
                    AttrValueType::Unsigned(row.base),
                    AttrValueType::Unsigned(row.size),
                    AttrValueType::Bool(true),
                    AttrValueType::Unsigned(row.base),
                    AttrValueType::Unsigned(row.size),
                    AttrValueType::Bool(true),
                    match &row.embedded_path {
                        Some(path) => AttrValueType::String(path.clone()),
                        None => AttrValueType::Nil,
                    },
                ])
            })
            .collect(),
    )
}

#[test]
fn parses_synthetic_tracker_maps_including_unknown_module() -> Result<()> {
    let rows = fixture_rows();
    let value = fixture_attr_value(&rows);

    let parsed = parse_module_list(&value)?;
    assert_eq!(parsed.len(), rows.len());

    for (row, (name, base, size, embedded_path)) in rows.iter().zip(parsed.iter()) {
        assert_eq!(*base, row.base);
        assert_eq!(*size, row.size);

        match &row.embedded_path {
            Some(path) => {
                let expected_name = PathBuf::from(path)
                    .file_name()
                    .expect("fixture embedded path has a file name")
                    .to_str()
                    .expect("fixture file name is valid UTF-8")
                    .to_string();
                assert_eq!(name, &expected_name);
                assert_eq!(embedded_path, &PathBuf::from(path));
            }
            None => {
                // Graceful naming/fallback for a genuinely pathless row -- not a
                // crash, not a silent misparse (e.g. an empty name or a panic).
                assert_eq!(name, UNKNOWN_MODULE_NAME);
                assert_eq!(embedded_path, &PathBuf::new());
            }
        }
    }

    // The real observed duplicate-name case: both instances present, distinct
    // base addresses, but the identical embedded path (not accidentally
    // collapsed/deduplicated, and not given divergent paths).
    let boot_script_matches: Vec<_> = parsed
        .iter()
        .filter(|(name, ..)| name == "BootScriptExecutorDxe.efi")
        .collect();
    assert_eq!(boot_script_matches.len(), 2);
    assert_ne!(boot_script_matches[0].1, boot_script_matches[1].1);
    assert_eq!(boot_script_matches[0].3, boot_script_matches[1].3);

    // The fabricated genuinely-different-path duplicate: both instances present,
    // distinct base addresses, and distinct embedded paths.
    let acpi_matches: Vec<_> = parsed
        .iter()
        .filter(|(name, ..)| name == "AcpiVTD.efi")
        .collect();
    assert_eq!(acpi_matches.len(), 2);
    assert_ne!(acpi_matches[0].1, acpi_matches[1].1);
    assert_ne!(acpi_matches[0].3, acpi_matches[1].3);

    Ok(())
}

#[test]
fn resolves_identical_path_duplicate_with_no_ambiguity_warning() -> Result<()> {
    let tmp = tempdir()?;
    let root = tmp.path();

    let rows = fixture_rows();
    let modules = parse_module_list(&fixture_attr_value(&rows))?
        .into_iter()
        .filter(|(name, ..)| name == "BootScriptExecutorDxe.efi")
        .collect::<Vec<_>>();
    assert_eq!(modules.len(), 2);

    // Mirror the shared embedded path locally, from "DEBUG_GCC/" onward, as a
    // single real local file.
    let shared_embedded_path = rows
        .iter()
        .find(|row| {
            row.embedded_path
                .as_deref()
                .map(|p| p.contains("BootScriptExecutorDxe"))
                .unwrap_or(false)
        })
        .and_then(|row| row.embedded_path.as_deref())
        .expect("fixture has a BootScriptExecutorDxe.efi row with an embedded path");
    let suffix = shared_embedded_path
        .rsplit_once("DEBUG_GCC/")
        .expect("fixture embedded path contains the DEBUG_GCC/ prefix marker")
        .1;
    let local_path = root.join(suffix);
    create_dir_all(
        local_path
            .parent()
            .expect("local fixture path has a parent directory"),
    )?;
    write(&local_path, b"contents of BootScriptExecutorDxe.efi")?;

    let capturing_writer = CapturingWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(capturing_writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    let info =
        tracing::subscriber::with_default(subscriber, || UefiOsInfo::resolve(&modules, root))?;
    assert_eq!(info.modules.len(), 2);

    // Both instances resolve to the exact same real local file: not ambiguous
    // once a real distinguishing (here, shared) path is available.
    for (name, _base, resolved_path) in &info.modules {
        assert_eq!(name, "BootScriptExecutorDxe.efi");
        assert_eq!(
            resolved_path, &local_path,
            "both BootScriptExecutorDxe.efi instances must resolve to the shared local file"
        );
    }

    let log_output = String::from_utf8(
        capturing_writer
            .0
            .lock()
            .expect("capturing writer mutex not poisoned")
            .clone(),
    )?;
    assert!(
        !log_output.to_lowercase().contains("ambiguous"),
        "resolving the identical-path duplicate must not warn about ambiguity, got: {log_output:?}"
    );

    Ok(())
}

#[test]
fn resolves_different_path_duplicate_via_path_suffix_disambiguation() -> Result<()> {
    let tmp = tempdir()?;
    let root = tmp.path();

    let rows = fixture_rows();
    let modules = parse_module_list(&fixture_attr_value(&rows))?
        .into_iter()
        .filter(|(name, ..)| name == "AcpiVTD.efi")
        .collect::<Vec<_>>();
    assert_eq!(modules.len(), 2);

    // Mirror each relevant fixture row's embedded path locally, from
    // "DEBUG_GCC/" onward (i.e. the "PkgA/..." / "PkgB/..." part), as a real
    // local file with content unique to that row -- used below to positively
    // confirm which exact local file each module resolved to, not just "a"
    // file.
    let mut expected_local_paths: HashMap<String, PathBuf> = HashMap::new();
    for row in rows.iter().filter(|row| {
        row.embedded_path
            .as_deref()
            .map(|p| p.contains("AcpiVTD"))
            .unwrap_or(false)
    }) {
        let embedded_path = row
            .embedded_path
            .as_deref()
            .expect("filtered to rows with an embedded path");
        let suffix = embedded_path
            .rsplit_once("DEBUG_GCC/")
            .expect("fixture embedded path contains the DEBUG_GCC/ prefix marker")
            .1;
        let local_path = root.join(suffix);
        create_dir_all(
            local_path
                .parent()
                .expect("local fixture path has a parent directory"),
        )?;
        write(&local_path, format!("contents of {suffix}"))?;
        expected_local_paths.insert(embedded_path.to_string(), local_path);
    }

    let info = UefiOsInfo::resolve(&modules, root)?;
    assert_eq!(info.modules.len(), 2);

    for (name, base, resolved_path) in &info.modules {
        let row = rows
            .iter()
            .find(|row| row.base == *base)
            .expect("resolved module base matches a fixture row");
        let embedded_path = row
            .embedded_path
            .as_deref()
            .expect("fixture row has an embedded path");
        let expected = expected_local_paths
            .get(embedded_path)
            .expect("fixture row has a corresponding local fixture file");

        assert_eq!(
            resolved_path, expected,
            "module {name} (base {base:#x}) resolved to the wrong local file; \
             path-suffix disambiguation failed"
        );
    }

    // The actual point of this test, not a vacuous pass: the two AcpiVTD.efi
    // entries must resolve to two *distinct* local files -- a resolver that just
    // grabbed "any" file matching the bare name would pass the per-module checks
    // above only by accident, but could not pass this.
    let resolved: Vec<&PathBuf> = info.modules.iter().map(|(_, _, path)| path).collect();
    assert_eq!(resolved.len(), 2);
    assert_ne!(
        resolved[0], resolved[1],
        "the two AcpiVTD.efi modules must resolve to distinct local files"
    );

    Ok(())
}

#[test]
fn falls_back_to_stem_match_and_warns_on_ambiguity_when_suffix_match_fails() -> Result<()> {
    let tmp = tempdir()?;
    let root = tmp.path();

    // Use a module with a real embedded path (not the pathless row) whose parent
    // directories deliberately don't exist locally at all, so path-suffix
    // matching finds nothing and both instances fall through to the bare-stem
    // fallback.
    let rows = vec![
        FixtureRow {
            base: 0x0000_0000_0740_0000,
            size: 0x5000,
            embedded_path: Some(format!(
                "{PREFIX}/PkgA/Bus/Pci/SataControllerDxe/DEBUG/SataController.efi"
            )),
        },
        FixtureRow {
            base: 0x0000_0000_0750_0000,
            size: 0x5100,
            embedded_path: Some(format!(
                "{PREFIX}/PkgB/Bus/Pci/SataControllerDxe/DEBUG/SataController.efi"
            )),
        },
    ];
    let modules = parse_module_list(&fixture_attr_value(&rows))?;
    assert_eq!(modules.len(), 2);

    // Deliberately unrelated local directory layouts: neither shares any path
    // component with the fixture's ".../PkgA|PkgB/Bus/Pci/SataControllerDxe/
    // DEBUG/" embedded-path tail beyond the bare file name itself. Path-suffix
    // matching therefore finds the bare file name "SataController.efi" as a
    // candidate suffix, but with *two* different local files sharing it -- an
    // ambiguous, not unique, match -- so per
    // `PathSuffixIndex::lookup_components_unambiguous`'s contract it must return
    // no match at all, forcing both modules through the bare-stem fallback path.
    let path_1 = root.join("unrelated_layout_one").join("SataController.efi");
    let path_2 = root.join("unrelated_layout_two").join("SataController.efi");
    create_dir_all(path_1.parent().expect("has parent"))?;
    create_dir_all(path_2.parent().expect("has parent"))?;
    write(&path_1, b"layout one")?;
    write(&path_2, b"layout two")?;

    let capturing_writer = CapturingWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(capturing_writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    let info = tracing::subscriber::with_default(subscriber, || {
        UefiOsInfo::resolve(&modules, root)
    })?;
    assert_eq!(info.modules.len(), 2);

    // Fail-open (the spec's own explicit decision): both modules still resolve --
    // not an error, not a dropped module -- to the first candidate in sorted
    // order. Both modules share the exact same ambiguous candidate set (the same
    // two unrelated local files), so both must fail open to the exact same
    // resolved path.
    let mut sorted_candidates = [path_1.clone(), path_2.clone()];
    sorted_candidates.sort();
    let expected_fail_open_path = sorted_candidates[0].clone();

    for (name, _base, resolved_path) in &info.modules {
        assert_eq!(name, "SataController.efi");
        assert_eq!(
            resolved_path, &expected_fail_open_path,
            "expected fail-open fallback to deterministically pick the first \
             (sorted) ambiguous candidate"
        );
    }

    let log_output = String::from_utf8(
        capturing_writer
            .0
            .lock()
            .expect("capturing writer mutex not poisoned")
            .clone(),
    )?;

    assert!(
        log_output.contains("WARN") && log_output.to_lowercase().contains("ambiguous"),
        "expected a WARN-level log message about ambiguous debug info resolution, got: {log_output:?}"
    );

    Ok(())
}

#[test]
fn resolve_skips_gracefully_not_panics_for_pathless_unknown_module() -> Result<()> {
    let tmp = tempdir()?;
    let root = tmp.path();

    let rows = fixture_rows();
    let modules = parse_module_list(&fixture_attr_value(&rows))?
        .into_iter()
        .filter(|(name, ..)| name == UNKNOWN_MODULE_NAME)
        .collect::<Vec<_>>();
    assert_eq!(modules.len(), 1);

    // Resolving a genuinely pathless ("unknown module") row must not panic or
    // abort the batch (a real `tracker_obj->maps` capture always has at least
    // one such row -- see UefiOsInfo::resolve's doc comment) -- it's skipped
    // with a warning, leaving an empty (not missing) result.
    let info = UefiOsInfo::resolve(&modules, root)?;
    assert!(
        info.modules.is_empty(),
        "an unknown/pathless module must be skipped, not resolved to a bogus path"
    );

    Ok(())
}

#[test]
fn resolve_skips_only_the_unresolvable_module_in_a_mixed_batch() -> Result<()> {
    // The real-world case this guards: a real tracker_obj->maps capture is a
    // mix of resolvable and genuinely pathless modules (confirmed live on
    // a live test session: 66 resolvable real modules plus 1 pathless "<unknown>" one, in
    // a single 67-row capture). One unresolvable module must not discard
    // source coverage for every other resolvable module in the same batch.
    let tmp = tempdir()?;
    let root = tmp.path();

    let rows = fixture_rows();
    let modules = parse_module_list(&fixture_attr_value(&rows))?;
    let resolvable_count = modules
        .iter()
        .filter(|(name, ..)| name != UNKNOWN_MODULE_NAME)
        .count();

    for (name, _base, _size, embedded_path) in &modules {
        if name == UNKNOWN_MODULE_NAME {
            continue;
        }
        let suffix = embedded_path
            .to_string_lossy()
            .rsplit_once("DEBUG_GCC/")
            .expect("fixture embedded path contains the DEBUG_GCC/ prefix marker")
            .1
            .to_string();
        let local_path = root.join(suffix);
        create_dir_all(
            local_path
                .parent()
                .expect("local fixture path has a parent directory"),
        )?;
        write(&local_path, b"contents")?;
    }

    let info = UefiOsInfo::resolve(&modules, root)?;
    assert_eq!(
        info.modules.len(),
        resolvable_count,
        "every resolvable module in the batch must still resolve despite the one unresolvable module"
    );

    Ok(())
}

#[test]
fn skips_invalid_top_level_entries_without_erroring() -> Result<()> {
    // Confirmed live (a real fuzzing run whose `HARNESS_START`
    // fired early in DXE dispatch): `tracker_obj->maps` can return a list
    // containing `AttrValueType::Invalid` entries -- reserved but not-yet-
    // populated slots -- alongside well-formed 7-element module rows. This must
    // not error the whole batch; those entries should simply be skipped.
    let rows = fixture_rows();
    let mut value = fixture_attr_value(&rows);

    let AttrValueType::List(ref mut top_level) = value else {
        panic!("fixture_attr_value did not return an AttrValueType::List");
    };
    top_level.insert(0, AttrValueType::Invalid);
    top_level.push(AttrValueType::Invalid);

    let parsed = parse_module_list(&value)?;
    assert_eq!(
        parsed.len(),
        rows.len(),
        "Invalid top-level entries must be skipped, not counted as modules or cause an error"
    );

    Ok(())
}

/// A `tracing_subscriber::fmt::MakeWriter` that captures formatted log output into
/// a shared in-memory buffer, so tests can assert on it directly instead of only
/// inferring the warning fired from behavior.
#[derive(Clone, Default)]
struct CapturingWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for CapturingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capturing writer mutex not poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CapturingWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
