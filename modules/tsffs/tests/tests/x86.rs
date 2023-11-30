// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test fuzzing an x86 user space application in Linux
//!
//! X86-64 architecture, hinted to x86

use anyhow::Result;
use command_ext::CommandExtCheck;
use ispm_wrapper::data::ProjectPackage;
use std::{path::PathBuf, process::Command};
use tests::{Architecture, TestEnvSpec};

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_user_magic() -> Result<()> {
    let mut env = TestEnvSpec::builder()
        .name("x86-user")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("examples")
            .join("tests")
            .join("x86-user")])
        .arch(Architecture::X86)
        .extra_packages([
            ProjectPackage::builder()
                .package_number(1030)
                .version("6.0.4")
                .build(),
            ProjectPackage::builder()
                .package_number(4094)
                .version("6.0.14")
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

    env.cleanup_if_env()?;

    Ok(())
}
