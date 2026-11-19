// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// The target passes a 4-byte size variable followed by a canary and prints whether the canary
/// survived `HARNESS_START`. It is 64-bit code run under the i386 architecture hint, as there is
/// no 32-bit target for Simics 7: the i386 backend reads the size as 4 bytes, and must write the
/// testcase size back as 4 bytes too, not with the 8-byte logical address width of the CPU.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_size_width_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_size_width_latest")
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
            @tsffs.iteration_limit = 20

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"
            @tsffs.iface.config.add_architecture_hint(conf.qsp.mb.cpu0.core[0][0], "i386")

            script-branch {
                bp.time.wait-for seconds = 15
                qsp.serconsole.con.input "\n"
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "FS0:\n"
                bp.time.wait-for seconds = .5
                local $manager = (start-agent-manager)
                qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test-size-width.efi") + "\n")
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "test-size-width.efi\n"
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
        output_str.contains("CANARY_INTACT") && !output_str.contains("CANARY_CLOBBERED"),
        "Writing the testcase size back overwrote the bytes after the 4-byte size variable"
    );

    Ok(())
}
