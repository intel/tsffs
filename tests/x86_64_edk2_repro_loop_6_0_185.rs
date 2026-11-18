// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// Prints `TSFFS_REPLAY_EMPTY ok` only if arming a replay on a directory without inputs is
/// refused with a `ValueError`.
const CHECK_EMPTY: &str = indoc! {r#"
    try:
        tsffs_replay_arm("%simics%/empty-inputs")
    except ValueError as e:
        print(f"TSFFS_REPLAY_EMPTY ok ({e})")
    else:
        print("TSFFS_REPLAY_EMPTY armed")
"#};

/// Replays a directory of inputs with `repro` in a session that never fuzzed: every input
/// runs from the snapshot, and one resume per input ends at that input's own stop.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_repro_loop_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_repro_loop_6_0_185")
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
            .join("x86_64-uefi-edk2")])
        // The first byte of an input selects the path of the test application: 'A' is an
        // invalid opcode, 'B' a hang, 'C' a page fault, anything else returns normally. The
        // hidden file and the subdirectory are what a real corpus directory also contains
        // and must not be replayed.
        .files([
            (
                "replay.py".to_string(),
                include_bytes!("../examples/source-coverage/replay.py").to_vec(),
            ),
            ("replay-inputs/0_empty".to_string(), vec![]),
            ("replay-inputs/1_default".to_string(), b"Zdefault".to_vec()),
            ("replay-inputs/2_pagefault".to_string(), b"C".to_vec()),
            ("replay-inputs/3_timeout".to_string(), b"B".to_vec()),
            ("replay-inputs/4_invalid_opcode".to_string(), b"A".to_vec()),
            ("replay-inputs/5_oversized".to_string(), vec![b'Z'; 200]),
            (
                "replay-inputs/.1_default.metadata".to_string(),
                b"{}".to_vec(),
            ),
            ("replay-inputs/subdir/ignored".to_string(), b"C".to_vec()),
            ("empty-inputs/.keep".to_string(), vec![]),
            (
                "check_empty.py".to_string(),
                CHECK_EMPTY.as_bytes().to_vec(),
            ),
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
            @tsffs.exceptions = [14, 6]
            @tsffs.generate_random_corpus = True

            run-script "%simics%/replay.py"
            run-script "%simics%/check_empty.py"
            @tsffs_replay_arm("%simics%/replay-inputs")

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

            # Snapshot restores rewind the simulated time, so a hang is caught on the host
            # clock instead.
            @import os, threading
            @guard = threading.Timer(600, lambda: os._exit(2))
            @guard.daemon = True
            @guard.start()

            run
            @tsffs_replay("%simics%/replay-inputs")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    assert!(
        output_str.contains("TSFFS_REPLAY_EMPTY ok"),
        "Arming a replay on a directory without inputs did not raise ValueError"
    );

    for (input, expected) in [
        ("1/6 0_empty", &["StopNormal"][..]),
        ("2/6 1_default", &["StopNormal"][..]),
        ("3/6 2_pagefault", &["Exception", "14"][..]),
        ("4/6 3_timeout", &["Timeout"][..]),
        ("5/6 4_invalid_opcode", &["Exception", "6"][..]),
        ("6/6 5_oversized", &["StopNormal"][..]),
    ] {
        let prefix = format!("TSFFS_REPLAY {input} ");
        let stop_reason = output_str
            .lines()
            .find_map(|line| line.split(&prefix).nth(1))
            .unwrap_or_else(|| panic!("No '{prefix}' line: the input was not replayed"));

        for word in expected {
            assert!(
                stop_reason.contains(word),
                "Input {input} stopped with '{stop_reason}', expected a stop reason with {expected:?}"
            );
        }
    }

    assert!(
        output_str.contains("TSFFS_REPLAY done 6"),
        "The replay did not run to the end of the directory"
    );

    Ok(())
}
