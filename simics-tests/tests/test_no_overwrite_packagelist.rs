// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::{
    env::var,
    fs::{read_dir, write},
    process::Command,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use simics::{
    api::sys::SIMICS_VERSION,
    manifest::{package_latest, package_version},
    package::PublicPackageNumber,
    project::Project,
    simics::home::simics_home,
};
use simics_fuzz::{args::Args, fuzzer::SimicsFuzzer};

const CARGO_MANIFEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../");
const ITERATIONS: usize = 1;

#[test]
#[cfg_attr(miri, ignore)]
fn test_no_overwrite_packagelist() -> Result<()> {
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
    let mut tmp_project_dir = TmpDirBuilder::default()
        .prefix("test_mini_cli_solution")
        .permissions(0o40755u32)
        .remove_on_drop(false)
        .build()?;

    let package_1000 = package_version(
        simics_home()?,
        PublicPackageNumber::Base.into(),
        SIMICS_VERSION.parse()?,
    )?;

    let project_setup = package_1000.path.join("bin").join("project-setup");

    let package_version_2096 =
        var("SIMICS_PACKAGE_VERSION_2096").unwrap_or(package_latest(simics_home()?, 2096)?.version);

    let package_2096 = package_version(
        simics_home()?,
        PublicPackageNumber::QspX86.into(),
        package_version_2096.parse()?,
    )?;

    Command::new(project_setup)
        .arg("--ignore-existing-files")
        .arg("--force")
        .arg(tmp_project_dir.path())
        .output()
        .map_err(|e| anyhow!("Error running project-setup: {}", e))
        .and_then(|s| {
            if s.status.success() {
                Ok(())
            } else {
                Err(anyhow!(
                    "Error doing project-setup:\nstderr: {}\nstdout: {}",
                    String::from_utf8_lossy(&s.stderr),
                    String::from_utf8_lossy(&s.stdout)
                ))
            }
        })?;

    let package_list_contents = format!(
        "{}\n{}",
        package_2096.path.canonicalize()?.display(),
        package_1000.path.canonicalize()?.display()
    );

    write(
        tmp_project_dir.path().join(".package-list"),
        package_list_contents.as_bytes(),
    )?;

    let project_project_setup = tmp_project_dir.path().join("bin").join("project-setup");

    Command::new(project_project_setup)
        .current_dir(tmp_project_dir.path())
        .output()
        .map_err(|e| anyhow!("Error running project-setup: {}", e))
        .and_then(|s| {
            if s.status.success() {
                Ok(())
            } else {
                Err(anyhow!(
                    "Error doing project-setup:\nstderr: {}\nstdout: {}",
                    String::from_utf8_lossy(&s.stderr),
                    String::from_utf8_lossy(&s.stdout)
                ))
            }
        })?;

    eprintln!("Created tmp corpus: {}", tmp_corpus_dir.path().display());

    let project_before: Project = tmp_project_dir.path().to_path_buf().try_into()?;

    // For this test, we set up an input corpus
    write(tmp_input_dir.path().join("1"), "racecar".as_bytes())?;

    let args = &[
        "simics-fuzz",
        "-p",
        &tmp_project_dir.path().to_string_lossy(),
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

    let project_after: Project = tmp_project_dir.path().to_path_buf().try_into()?;

    assert_eq!(
        project_after.packages(),
        project_before.packages(),
        "Package list contents differed"
    );

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
    tmp_project_dir.remove_on_drop(true);

    assert!(corpus_entries > 0, "No corpus in {} iterations", ITERATIONS);
    Ok(())
}
