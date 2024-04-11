// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_apitest_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_apitest_6_0_185")
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
            .join("x86_64-uefi")])
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module tsffs
            init-tsffs

            @tsffs.log_level = 2

            @tsffs.all_breakpoints_are_solutions = True
            @tsffs.all_breakpoints_are_solutions = False
            @tsffs.all_exceptions_are_solutions = True
            @tsffs.all_exceptions_are_solutions = False
            @tsffs.exceptions = [14]
            @tsffs.exceptions.remove(14)
            @tsffs.exceptions = [14]
            @tsffs.breakpoints = [1]
            @tsffs.breakpoints.remove(1)
            @tsffs.timeout = 3.0
            @tsffs.start_on_harness = True
            @tsffs.stop_on_harness = True
            @tsffs.iteration_limit = 100
            @tsffs.initial_random_corpus_size = 32
            @tsffs.corpus_directory = SIM_lookup_file("%simics%") + "/corpus"
            @tsffs.solutions_directory = SIM_lookup_file("%simics%") + "/solutions"
            @tsffs.generate_random_corpus = True
            @tsffs.cmplog = True
            @tsffs.coverage_reporting = True
            @tsffs.token_executables += [SIM_lookup_file("%simics%/test.efi")]
            @tsffs.pre_snapshot_checkpoint = False

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
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())
}
