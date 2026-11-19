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

/// Replays a directory of inputs with `repro` while Simics collects coverage of the UEFI
/// firmware context: the LCOV report credits the lines of every replayed path to the
/// source file of the test application, and the HTML report is written.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_replay_coverage_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_replay_coverage_6_0_185")
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
        // The directory also holds `debug-info`, the debug files of the test application
        // exported by its `build.sh`.
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86_64-uefi-edk2")])
        // The first byte of an input selects the path of the test application: 'A' is an
        // invalid opcode, 'B' a hang, 'C' a page fault, anything else returns normally.
        .files([
            (
                "replay.py".to_string(),
                include_bytes!("../examples/source-coverage/replay.py").to_vec(),
            ),
            ("replay-inputs/1_default".to_string(), b"Zdefault".to_vec()),
            ("replay-inputs/2_pagefault".to_string(), b"C".to_vec()),
            ("replay-inputs/3_timeout".to_string(), b"B".to_vec()),
            ("replay-inputs/4_invalid_opcode".to_string(), b"A".to_vec()),
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
            @tsffs.generate_random_corpus = True

            run-script "%simics%/replay.py"
            @tsffs_replay_arm("%simics%/replay-inputs")

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

            $cc = (collect-coverage context-query = "'UEFI Firmware'" -running)
            $cc.add-path-map "/edk2/HelloWorld" (lookup-file "%simics%/debug-info")
            @tsffs_replay("%simics%/replay-inputs")
            $cc.stop
            $cc.add-functions
            $cc.add-source-info
            @simenv.lcov = SIM_lookup_file("%simics%") + "/coverage-lcov"
            @simenv.html = SIM_lookup_file("%simics%") + "/coverage-html"
            $cc.lcov-output $lcov
            $cc.html-report $html

            run-script "%simics%/print_report.py"
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    for (input, expected) in [
        ("1/4 1_default", &["StopNormal"][..]),
        ("2/4 2_pagefault", &[PAGE_FAULT][..]),
        ("3/4 3_timeout", &["Timeout"][..]),
        ("4/4 4_invalid_opcode", &[INVALID_OPCODE][..]),
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
