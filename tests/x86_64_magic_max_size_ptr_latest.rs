// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR` passes the maximum testcase size as its own
/// argument, and the size variable does not have to hold it. The target leaves the size variable
/// at 0 and prints the size of every testcase it receives, so a testcase of the maximum size
/// (8 bytes) only reaches it when the maximum size argument is honoured.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_max_size_ptr_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_max_size_ptr_latest")
        .package_crates([PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .packages([
            ProjectPackage::builder()
                .package_number(1000)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(1030)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(2096)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(8112)
                .version("latest")
                .build(),
        ])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86_64-uefi")])
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module tsffs
            init-tsffs

            @tsffs.log_level = 2
            @tsffs.start_on_harness = True
            @tsffs.stop_on_harness = True
            @tsffs.timeout = 3.0
            @tsffs.exceptions = [14]
            @tsffs.generate_random_corpus = True
            @tsffs.iteration_limit = 100

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

            script-branch {
                bp.time.wait-for seconds = 15
                qsp.serconsole.con.input "\n"
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "FS0:\n"
                bp.time.wait-for seconds = .5
                local $manager = (start-agent-manager)
                qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test-max-size.efi") + "\n")
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "test-max-size.efi\n"
            }

            script-branch {
                bp.time.wait-for seconds = 240
                quit 1
            }

            run
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    assert!(
        output_str.contains("TESTCASE_SIZE 8"),
        "No testcase of the maximum size (8 bytes) reached the harness"
    );

    Ok(())
}
