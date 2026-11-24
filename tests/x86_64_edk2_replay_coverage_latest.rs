// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::{collections::BTreeSet, path::PathBuf};

/// The source of the test application, as built by `tests/rsrc/x86_64-uefi-edk2/build.sh`
const SOURCE: &str = include_str!("rsrc/x86_64-uefi-edk2/src/HelloWorld.c");

/// Prints every line of the LCOV records of `HelloWorld.c` found in the report directory
/// `$lcov`, prefixed with `TSFFS_LCOV `. Then prints, as `TSFFS_HTML index <bytes>`, the size
/// of the first page of the HTML report in `$html`, and as `TSFFS_HTML source <path>` each
/// page of that report that names `HelloWorld.c`.
const PRINT_REPORT: &str = indoc! {r#"
    import os

    from cli import simenv

    for name in sorted(os.listdir(simenv.lcov)):
        with open(os.path.join(simenv.lcov, name)) as report:
            for record in report.read().split("end_of_record"):
                lines = [line for line in record.splitlines() if line]
                if any(l.startswith("SF:") and l.endswith("/HelloWorld.c") for l in lines):
                    for line in lines:
                        print(f"TSFFS_LCOV {line}")

    print(f"TSFFS_HTML index {os.path.getsize(os.path.join(simenv.html, 'index.html'))}")

    for directory, _, names in sorted(os.walk(simenv.html)):
        for name in sorted(names):
            if name.endswith(".html"):
                with open(os.path.join(directory, name), errors="replace") as page:
                    if "HelloWorld.c" in page.read():
                        path = os.path.relpath(os.path.join(directory, name), simenv.html)
                        print(f"TSFFS_HTML source {path}")
"#};

/// The stop reason TSFFS reports for a page fault, as `last_stop_reason` serializes it
const PAGE_FAULT: &str = r#"{"Solution":{"kind":{"Exception":{"number":14}}}}"#;

/// The stop reason TSFFS reports for an invalid opcode
const INVALID_OPCODE: &str = r#"{"Solution":{"kind":{"Exception":{"number":6}}}}"#;

/// The number of the only source line of the test application's `UefiMain` containing
/// `marker`
fn source_line(marker: &str) -> usize {
    let lines = SOURCE
        .lines()
        .enumerate()
        .skip_while(|(_, line)| !line.contains("UefiMain("))
        .filter(|(_, line)| line.contains(marker))
        .map(|(index, _)| index + 1)
        .collect::<Vec<_>>();

    assert_eq!(
        lines.len(),
        1,
        "Expected one line of UefiMain in HelloWorld.c with '{marker}', found {lines:?}"
    );

    lines[0]
}

/// Replays a directory of testcases with one `repro` call in a session that never fuzzed
/// and has no corpus, while Simics collects coverage of the UEFI firmware context. The
/// session stops at the harness for coverage to start, one resume then runs every testcase
/// from the snapshot to its own stop, the LCOV report credits the lines of every replayed
/// path to the source file of the test application, and the HTML report is written.
/// `repro` then runs single testcases again in the same session.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_replay_coverage_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_replay_coverage_latest")
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
        // The directory also holds `debug-info`, the debug files of the test application
        // exported by its `build.sh`.
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86_64-uefi-edk2")])
        // The first byte of a testcase selects the path of the test application: 'A' is an
        // invalid opcode, 'B' a hang, 'C' a page fault, anything else returns normally. The
        // hidden file and the subdirectory are what a real corpus directory also contains
        // and must not be replayed.
        .files([
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
            (
                "print_report.py".to_string(),
                PRINT_REPORT.as_bytes().to_vec(),
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
            # A replay must neither need nor write the corpus directory
            @tsffs.keep_all_corpus = True

            @tsffs.repro_auto_continue = False
            @tsffs.iface.fuzz.repro("%simics%/replay-inputs")

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

            load-module uefi-fw-tracker
            new-os-awareness name = qsp.software
            qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
            qsp.software.tracker.detect-parameters -load
            qsp.software.enable-tracker

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

            # Stops at the harness with the snapshot taken. Coverage starts here, which
            # keeps the boot out of the report.
            run
            @print(f"TSFFS_EVENT at start {tsffs.last_stop_reason}")

            $cc = (collect-coverage context-query = "'UEFI Firmware'" -running)
            $cc.add-path-map "/edk2/HelloWorld" (lookup-file "%simics%/debug-info")
            # Runs every testcase of the directory
            run
            @print("TSFFS_EVENT directory ended")
            $cc.stop
            $cc.add-functions
            $cc.add-source-info
            @simenv.lcov = SIM_lookup_file("%simics%") + "/coverage-lcov"
            @simenv.html = SIM_lookup_file("%simics%") + "/coverage-html"
            $cc.lcov-output $lcov
            $cc.html-report $html

            run-script "%simics%/print_report.py"

            # One more testcase in the same session. The snapshot was taken with
            # repro_auto_continue = False, so this one waits at the harness too.
            @tsffs.iface.fuzz.repro("%simics%/replay-inputs/2_pagefault")
            @print(f"TSFFS_EVENT prepared {tsffs.last_stop_reason} {tsffs.last_exception_processor_number}")
            run
            @print(f"TSFFS_EVENT ended {tsffs.last_stop_reason} {tsffs.last_exception_processor_number}")

            # Restoring the snapshot must not bring repro_auto_continue = False back, here
            # and between the two testcases: the setting of the moment decides, so this
            # repro resumes by itself and has run its testcases before the next line.
            @tsffs.repro_auto_continue = True
            @tsffs.iface.fuzz.repro("%simics%/replay-again")
            @print(f"TSFFS_EVENT auto resumed {tsffs.repro_auto_continue} {tsffs.last_stop_reason}")
            @import os
            @print(f"TSFFS_EVENT corpus directory {os.path.exists(SIM_lookup_file('%simics%') + '/corpus')}")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    // What the session did, in order: the lines the script prints and the `Replayed
    // <file>: <stop reason>` line TSFFS logs when a testcase ends.
    let events = output_str
        .lines()
        .filter_map(|line| {
            line.split_once("TSFFS_EVENT ")
                .or_else(|| line.split_once("Replayed "))
                .map(|(_, event)| event.trim_end())
        })
        .collect::<Vec<_>>();

    // Each expected event is given by the parts its line must contain
    let expected = [
        // With `repro_auto_continue = False` the first run ends at the harness start,
        // before any testcase runs
        vec!["at start", "StartBufferPtrSizePtr"],
        // The second run executes every testcase of the directory, sorted by name and
        // without the hidden file and the subdirectory, each to its own stop
        vec!["replay-inputs/0_empty: ", "StopNormal"],
        vec!["replay-inputs/1_default: ", "StopNormal"],
        vec!["replay-inputs/2_pagefault: ", PAGE_FAULT],
        vec!["replay-inputs/3_timeout: ", "Timeout"],
        vec!["replay-inputs/4_invalid_opcode: ", INVALID_OPCODE],
        vec!["replay-inputs/5_oversized: ", "StopNormal"],
        // and only returns to the script after the last one
        vec!["directory ended"],
        // A testcase prepared by a later `repro` does not report the stop of the previous
        // one, and reports its own once it has run
        vec!["prepared None None"],
        vec!["replay-inputs/2_pagefault: ", PAGE_FAULT],
        vec!["ended ", PAGE_FAULT, " 0"],
        // `repro_auto_continue` set after the snapshot was taken is the one that counts,
        // and the restores between the testcases of a directory do not bring the old
        // value back
        vec!["replay-again/0_pagefault: ", PAGE_FAULT],
        vec!["replay-again/1_default: ", "StopNormal"],
        vec!["auto resumed True", "StopNormal"],
        vec!["corpus directory False"],
    ];

    assert_eq!(
        events.len(),
        expected.len(),
        "Expected {} events, got {events:#?}",
        expected.len()
    );

    for (event, parts) in events.iter().zip(&expected) {
        assert!(
            parts.iter().all(|part| event.contains(part)),
            "Event '{event}' does not contain {parts:?}, events: {events:#?}"
        );
    }

    let lcov = output_str
        .lines()
        .filter_map(|line| line.split("TSFFS_LCOV ").nth(1))
        .map(str::trim_end)
        .collect::<Vec<_>>();

    assert!(
        lcov.iter()
            .any(|line| line.starts_with("SF:") && line.ends_with("debug-info/HelloWorld.c")),
        "No LCOV record for the source file under the path map, got {lcov:?}"
    );

    // `DA:<line>,<count>` entries of the source file with a non-zero count
    let hit_lines = lcov
        .iter()
        .filter_map(|line| line.strip_prefix("DA:"))
        .filter_map(|entry| entry.split_once(','))
        .filter(|(_, count)| count.parse::<u64>().is_ok_and(|count| count > 0))
        .filter_map(|(line, _)| line.parse::<usize>().ok())
        .collect::<BTreeSet<_>>();

    // One line per replayed path: the invalid opcode, the hang, the page fault and the
    // normal return. The page fault is the store through the invalid pointer: the line
    // that initializes the pointer has no code of its own, so no `DA:` entry.
    for marker in [".byte 0x06", "gBS->Stall", "*ptr = 0x00;", "Working..."] {
        let line = source_line(marker);

        assert!(
            hit_lines.contains(&line),
            "Line {line} of HelloWorld.c ('{marker}') is not covered, covered lines: {hit_lines:?}"
        );
    }

    // The HTML report was written too: its first page is not empty, and one of its pages
    // is about the source file. Its layout differs between Simics versions.
    let html = output_str
        .lines()
        .filter_map(|line| line.split("TSFFS_HTML ").nth(1))
        .map(str::trim_end)
        .collect::<Vec<_>>();

    assert!(
        html.iter()
            .filter_map(|line| line.strip_prefix("index "))
            .any(|size| size.parse::<u64>().is_ok_and(|size| size > 0)),
        "No first page in the HTML report, got {html:?}"
    );
    assert!(
        html.iter().any(|line| line.starts_with("source ")),
        "No page of the HTML report names HelloWorld.c, got {html:?}"
    );

    Ok(())
}
