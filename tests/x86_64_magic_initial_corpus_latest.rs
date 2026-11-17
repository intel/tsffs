// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// With `use_initial_as_corpus`, the initial contents of the harness buffer must seed the
/// corpus. The corpus directory starts empty and no random corpus is generated, so the initial
/// buffer is the only possible seed.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_initial_corpus_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_initial_corpus_latest")
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
            @tsffs.generate_random_corpus = False
            @tsffs.use_initial_as_corpus = True
            @tsffs.iteration_limit = 20
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

            @import os; corpus = os.path.join(SIM_lookup_file("%simics%"), "corpus"); print("TSFFS_" + "CORPUS " + ",".join(open(os.path.join(corpus, f), "rb").read(16).hex() for f in sorted(os.listdir(corpus)) if not f.startswith(".")) + " END")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    let corpus = output_str
        .lines()
        .find_map(|line| line.split("TSFFS_CORPUS ").nth(1))
        .and_then(|rest| rest.split(" END").next())
        .expect("Simics script did not print TSFFS_CORPUS");

    // The harness buffer initially holds eight 'A' bytes
    assert!(
        corpus.split(',').any(|entry| entry == "4141414141414141"),
        "Expected the initial buffer contents (eight 'A' bytes) in the corpus directory, \
         found: [{corpus}]"
    );

    Ok(())
}
