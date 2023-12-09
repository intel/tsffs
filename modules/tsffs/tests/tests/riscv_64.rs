// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test for fuzzing a kernel module using a harnessed user-space application
//! RISC-V architecture

use anyhow::Result;
use command_ext::CommandExtCheck;
use std::{path::PathBuf, process::Command};
use tests::{Architecture, TestEnvSpec};

#[test]
#[cfg_attr(miri, ignore)]
fn test_riscv_64_kernel_from_userspace_magic() -> Result<()> {
    let mut env = TestEnvSpec::builder()
        .name("test_riscv_64_kernel_from_userspace_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("examples")
            .join("tests")
            .join("riscv-64")])
        .arch(Architecture::Riscv)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir_ref())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-kernel-from-userspace-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    env.cleanup_if_env()?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_riscv_64_kernel_magic() -> Result<()> {
    let mut env = TestEnvSpec::builder()
        .name("test_riscv_64_kernel_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("examples")
            .join("tests")
            .join("riscv-64")])
        .arch(Architecture::Riscv)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir_ref())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-kernel-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    env.cleanup_if_env()?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_riscv_64_userspace_magic() -> Result<()> {
    let mut env = TestEnvSpec::builder()
        .name("test_riscv_64_userspace_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("examples")
            .join("tests")
            .join("riscv-64")])
        .arch(Architecture::Riscv)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir_ref())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-userspace-magic.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    env.cleanup_if_env()?;

    Ok(())
}
