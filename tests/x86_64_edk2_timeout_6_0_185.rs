// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_timeout_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_timeout_6_0_185")
        .package_crates([PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .packages([
            ProjectPackage::builder()
                .package_number(1000)
                .version("6.0.185")
                .build(),
            ProjectPackage::builder()
                .package_number(2096)
                .version("6.0.73")
                .build(),
            ProjectPackage::builder()
                .package_number(8112)
                .version("6.0.21")
                .build(),
        ])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86_64-timeout-uefi-edk2")])
        // 'A' makes the harness stall past the timeout; 'D' runs normally so the corpus is
        // not empty once the timeout input is moved to solutions.
        .files([
            ("corpus/timeout".to_string(), vec![b'A']),
            ("corpus/normal".to_string(), vec![b'D']),
        ])
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
            @tsffs.generate_random_corpus = False
            @tsffs.iteration_limit = 30
            @tsffs.quit_on_iteration_limit = False

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

            script-branch {
                bp.time.wait-for seconds = 15
                qsp.serconsole.con.input "\n"
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "FS0:\n"
                bp.time.wait-for seconds = .5
                local $manager = (start-agent-manager)
                qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test.efi") + "\n")
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "test.efi\n"
            }

            script-branch {
                bp.time.wait-for seconds = 240
                quit 1
            }

            run

            @import os; solutions = os.path.join(SIM_lookup_file("%simics%"), "solutions"); print("TSFFS_" + "SOLUTIONS " + ",".join(open(os.path.join(solutions, f), "rb").read(1).hex() for f in sorted(os.listdir(solutions)) if not f.startswith(".")) + " END")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    let solutions = output_str
        .lines()
        .find_map(|line| line.split("TSFFS_SOLUTIONS ").nth(1))
        .and_then(|rest| rest.split(" END").next())
        .expect("Simics script did not print TSFFS_SOLUTIONS");

    // Only timeouts can be solutions for this harness, and only inputs starting with 'A' time out.
    assert!(
        !solutions.is_empty() && solutions.split(',').all(|first_byte| first_byte == "41"),
        "Expected timeout solutions (inputs starting with 'A') in the solutions directory, \
         found first bytes: [{solutions}]"
    );

    Ok(())
}
