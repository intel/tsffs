// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// A solution found on the final iteration before `iteration_limit` must still be saved. The only
/// corpus entry is the input that crashes the target and the limit is one iteration, so the
/// final iteration is the crash.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_crash_iteration_limit_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_crash_iteration_limit_latest")
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
        .files([("corpus/crash".to_string(), b"fuzzing!".to_vec())])
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
            @tsffs.iteration_limit = 1
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

            @import os; solutions = os.path.join(SIM_lookup_file("%simics%"), "solutions"); print("TSFFS_" + "SOLUTIONS " + ",".join(open(os.path.join(solutions, f), "rb").read(16).hex() for f in sorted(os.listdir(solutions)) if not f.startswith(".")) + " END")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    let solutions = output_str
        .lines()
        .find_map(|line| line.split("TSFFS_SOLUTIONS ").nth(1))
        .and_then(|rest| rest.split(" END").next())
        .expect("Simics script did not print TSFFS_SOLUTIONS");

    // "fuzzing!"
    assert!(
        solutions
            .split(',')
            .any(|entry| entry == "66757a7a696e6721"),
        "Expected the crashing input of the final iteration in the solutions directory, \
         found: [{solutions}]"
    );

    Ok(())
}
