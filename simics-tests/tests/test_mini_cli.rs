// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::{
    env::var,
    fs::{read_dir, write},
};

use anyhow::Result;
use clap::Parser;
use simics::{manifest::package_latest, simics::home::simics_home};
use simics_fuzz::{args::Args, fuzzer::SimicsFuzzer};

const CARGO_MANIFEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../");
const ITERATIONS: usize = 3;

#[test]
#[cfg_attr(miri, ignore)]
fn test_harnessing_mini_cli() -> Result<()> {
    use tmp_dir::TmpDirBuilder;

    let mut tmp_input_dir = TmpDirBuilder::default()
        .prefix("test_mini_cli_input")
        .permissions(0o40755u32)
        .remove_on_drop(false)
        .build()?;
    let mut tmp_corpus_dir = TmpDirBuilder::default()
        .prefix("test_mini_cli_corpus")
        .permissions(0o40755u32)
        .remove_on_drop(false)
        .build()?;
    let mut tmp_solution_dir = TmpDirBuilder::default()
        .prefix("test_mini_cli_solution")
        .permissions(0o40755u32)
        .remove_on_drop(false)
        .build()?;

    let package_version_2096 =
        var("SIMICS_PACKAGE_VERSION_2096").unwrap_or(package_latest(simics_home()?, 2096)?.version);

    eprintln!("Created tmp corpus: {}", tmp_corpus_dir.path().display());

    // For this test, we set up an input corpus
    write(tmp_input_dir.path().join("1"), "racecar".as_bytes())?;

    let args = &[
        "simics-fuzz",
        "-i",
        &tmp_input_dir.path().to_string_lossy(),
        "-c",
        &tmp_corpus_dir.path().to_string_lossy(),
        "-o",
        &tmp_solution_dir.path().to_string_lossy(),
        "-l",
        "INFO",
        "-C",
        "1",
        "--iterations",
        &format!("{}", ITERATIONS),
        "--no-keep-temp-projects",
        "--package",
        &format!("2096:{}", package_version_2096),
        "--file",
        &format!(
            "{}/examples/mini/rsrc/mini.efi:%simics%/mini.efi",
            CARGO_MANIFEST_DIR
        ),
        "--file",
        &format!(
            "{}/examples/mini/rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff",
            CARGO_MANIFEST_DIR
        ),
        "--file",
        &format!(
            "{}/examples/mini/rsrc/fuzz.simics:%simics%/fuzz.simics",
            CARGO_MANIFEST_DIR
        ),
        "--command",
        "CONFIG:%simics%/fuzz.simics",
    ];

    println!("{}", args.join(" "));

    let args = Args::parse_from(args);

    println!("{:?}", args);

    SimicsFuzzer::cli_main(args)?;

    let corpus_entries = read_dir(tmp_corpus_dir.path())
        .map_err(|e| {
            eprintln!(
                "Couldn't read corpus directory {}: {}",
                tmp_corpus_dir.path().display(),
                e
            );
            e
        })?
        .count();

    // NOTE: We enable this after cli main runs because otherwise they are dropped multiple times,
    // in the fuzzer children *and* in this thread
    tmp_input_dir.remove_on_drop(true);
    tmp_corpus_dir.remove_on_drop(true);
    tmp_solution_dir.remove_on_drop(true);

    assert!(corpus_entries > 0, "No corpus in {} iterations", ITERATIONS);
    Ok(())
}
