// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// The target compares its 4-byte testcase against an immediate operand
/// (`cmp dword [rsp + N], 0x21584946`) and prints `MAGIC_FOUND` on a match. Guessing 32 bits at
/// once is out of reach in this many iterations, so the match is only found when compares against
/// immediates are logged for the fuzzer.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_cmp_immediate_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_cmp_immediate_latest")
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
            @tsffs.iteration_limit = 2000

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

            script-branch {
                bp.time.wait-for seconds = 15
                qsp.serconsole.con.input "\n"
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "FS0:\n"
                bp.time.wait-for seconds = .5
                local $manager = (start-agent-manager)
                qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test-cmp-imm.efi") + "\n")
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "test-cmp-imm.efi\n"
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
        output_str.contains("MAGIC_FOUND"),
        "The fuzzer did not find the immediate the target compares its testcase against"
    );

    Ok(())
}
