// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// Prints `TSFFS_REPLAY_EMPTY ok` and `TSFFS_REPLAY_MISSING ok` only if arming a replay is
/// refused with a `ValueError`, on a directory without inputs and on a directory that does
/// not exist.
const CHECK_EMPTY: &str = indoc! {r#"
    for label, directory in [
        ("EMPTY", "%simics%/empty-inputs"),
        ("MISSING", "%simics%/missing-inputs"),
    ]:
        try:
            tsffs_replay_arm(directory)
        except ValueError as e:
            print(f"TSFFS_REPLAY_{label} ok ({e})")
        else:
            print(f"TSFFS_REPLAY_{label} armed")
"#};

/// Reproduces the page fault once more, then prepares another testcase without running it.
/// Prints the stop reason and the exception processor number TSFFS reports at both points,
/// as `TSFFS_REPRO_ENDED` and `TSFFS_REPRO_PREPARED`.
const CHECK_CLEARED: &str = indoc! {r#"
    import cli
    import conf


    def report(label):
        print(
            f"TSFFS_REPRO_{label} {conf.tsffs.last_stop_reason} "
            f"{conf.tsffs.last_exception_processor_number}"
        )


    conf.tsffs.iface.fuzz.repro("%simics%/replay-inputs/2_pagefault")
    cli.quiet_run_command("run")
    report("ENDED")
    conf.tsffs.iface.fuzz.repro("%simics%/replay-inputs/1_default")
    report("PREPARED")
"#};

/// The stop reason TSFFS reports for a page fault, as `last_stop_reason` serializes it
const PAGE_FAULT: &str = r#"{"Solution":{"kind":{"Exception":{"number":14}}}}"#;

/// The stop reason TSFFS reports for an invalid opcode
const INVALID_OPCODE: &str = r#"{"Solution":{"kind":{"Exception":{"number":6}}}}"#;

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
            ("replay-again/0_pagefault".to_string(), b"C".to_vec()),
            ("replay-again/1_default".to_string(), b"Zdefault".to_vec()),
            ("empty-inputs/.keep".to_string(), vec![]),
            (
                "check_empty.py".to_string(),
                CHECK_EMPTY.as_bytes().to_vec(),
            ),
            (
                "check_cleared.py".to_string(),
                CHECK_CLEARED.as_bytes().to_vec(),
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

            run-script "%simics%/check_cleared.py"

            # The snapshot was taken with repro_auto_continue = False. Restoring it must not
            # bring that value back: the setting of the moment decides, so this repro
            # resumes by itself and has run its testcase before the next line.
            @tsffs.repro_auto_continue = True
            @tsffs.iface.fuzz.repro("%simics%/replay-inputs/2_pagefault")
            @print(f"TSFFS_AUTO_RESUMED {tsffs.repro_auto_continue} {tsffs.last_stop_reason}")

            # A replay without arming first, with repro_auto_continue enabled: the helper
            # still owns the resumes.
            @tsffs_replay("%simics%/replay-again")
            @print(f"TSFFS_AUTO_CONTINUE {tsffs.repro_auto_continue}")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    assert!(
        output_str.contains("TSFFS_REPLAY_EMPTY ok (no inputs to replay in %simics%/empty-inputs)"),
        "Arming a replay on a directory without inputs did not raise ValueError"
    );
    assert!(
        output_str
            .contains("TSFFS_REPLAY_MISSING ok (no inputs to replay in %simics%/missing-inputs)"),
        "Arming a replay on a directory that does not exist did not raise ValueError"
    );

    for (input, expected) in [
        ("1/6 0_empty", &["StopNormal"][..]),
        ("2/6 1_default", &["StopNormal"][..]),
        ("3/6 2_pagefault", &[PAGE_FAULT][..]),
        ("4/6 3_timeout", &["Timeout"][..]),
        ("5/6 4_invalid_opcode", &[INVALID_OPCODE][..]),
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

    // What a reproduced execution reports is that execution's own: once `repro` has
    // prepared another testcase, the page fault of the previous one is gone.
    assert!(
        output_str.contains(&format!("TSFFS_REPRO_ENDED {PAGE_FAULT} 0")),
        "The reproduced page fault did not report exception 14 on processor 0"
    );
    assert!(
        output_str.contains("TSFFS_REPRO_PREPARED None None"),
        "A testcase prepared by repro still reports the stop of the previous testcase"
    );

    // Restoring the snapshot restores the attributes TSFFS had when it was taken. `repro`
    // must keep the `repro_auto_continue` of the moment, here `True` over a snapshot taken
    // with `False`.
    assert!(
        output_str.contains(&format!("TSFFS_AUTO_RESUMED True {PAGE_FAULT}")),
        "repro did not resume by itself with repro_auto_continue set after the snapshot"
    );

    // `tsffs_replay` without `tsffs_replay_arm`, with `repro_auto_continue` enabled:
    // `repro` must not also resume by itself, or an input runs past its stop.
    for (input, expected) in [
        ("1/2 0_pagefault", PAGE_FAULT),
        ("2/2 1_default", "StopNormal"),
    ] {
        let prefix = format!("TSFFS_REPLAY {input} ");
        let stop_reason = output_str
            .lines()
            .find_map(|line| line.split(&prefix).nth(1))
            .unwrap_or_else(|| panic!("No '{prefix}' line: the input was not replayed"));

        assert!(
            stop_reason.contains(expected),
            "Input {input} stopped with '{stop_reason}', expected a stop reason with '{expected}'"
        );
    }

    assert!(
        output_str.contains("TSFFS_REPLAY done 2"),
        "The replay without arming did not run to the end of the directory"
    );
    assert!(
        output_str.contains("TSFFS_AUTO_CONTINUE False"),
        "tsffs_replay left repro_auto_continue enabled"
    );

    Ok(())
}
