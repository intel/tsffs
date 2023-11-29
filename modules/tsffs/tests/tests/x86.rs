// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test fuzzing an x86 user space application in Linux
//!
//! X86-64 architecture, hinted to x86

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use std::{process::Command, path::PathBuf};
use tests::{Architecture, TestEnvSpec};

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_magic() -> Result<()> {
    let env = TestEnvSpec::builder()
        .name("x86_64-edk2-magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("examples")
                .join("tests")
                .join("x86-user")
        ])
        .arch(Architecture::X86)
        .extra_packages([
            ProjectPackage::builder()
                .package_number(1030)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(4094)
                .version("latest")
                .build(),
        ])
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test-user.py")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())

}