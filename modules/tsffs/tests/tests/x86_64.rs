// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test fuzzing a UEFI firmware built with EDK2
//!
//! X86-64 architecture

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use std::{process::Command, path::PathBuf};
use tests::{Architecture, TestEnvSpec};
use ispm_wrapper::data::ProjectPackage;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_magic() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_edk2_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-uefi-edk2")
        ])
        .arch(Architecture::X86)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-uefi-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_crash() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_magic_crash")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-crash-uefi")
        ])
        .arch(Architecture::X86)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_timeout_edk2() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_timeout_edk2")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-timeout-uefi-edk2")
        ])
        .arch(Architecture::X86)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-uefi")
        ])
        .arch(Architecture::X86)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_manual() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_manual")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-uefi")
        ])
        .arch(Architecture::X86)
        .extra_packages([ProjectPackage::builder()
            .package_number(1030)
            .version("latest")
            .build()])
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-manual.py")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_manual_max() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("test_x86_64_manual_max")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86_64-uefi")
        ])
        .arch(Architecture::X86)
        .extra_packages([ProjectPackage::builder()
            .package_number(1030)
            .version("latest")
            .build()])
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-manual-max.py")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}