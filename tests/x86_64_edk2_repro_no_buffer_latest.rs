// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

/// A harness that starts with `start_without_buffer`: TSFFS is not told where the
/// testcase goes, so the callback asks for it each time the start harness of the test
/// application is reached and writes it into the application's buffer itself.
const NO_BUFFER_HARNESS: &str = indoc! {r#"
    import conf
    import simics

    # The magic number of the start harness compiled into the test application, which
    # holds the address of its buffer in rsi
    MAGIC_START = 1
    BUFFER_SIZE = 64


    def on_magic(_data, cpu, number):
        if number != MAGIC_START:
            return

        testcase = bytes(conf.tsffs.iface.fuzz.start_without_buffer(cpu))
        registers = cpu.iface.int_register
        buffer = cpu.iface.processor_info.logical_to_physical(
            registers.read(registers.get_number("rsi")), simics.Sim_Access_Write
        )

        for offset, byte in enumerate(testcase[:BUFFER_SIZE]):
            simics.SIM_write_phys_memory(cpu, buffer.address + offset, byte, 1)


    simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
"#};

/// Replays a directory of testcases with `repro` when the harness starts with
/// `start_without_buffer`: TSFFS has no buffer to write to, so each testcase is handed
/// over where the restored snapshot reaches the start again, and runs to its own stop.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_repro_no_buffer_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_repro_no_buffer_latest")
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
            .join("x86_64-uefi-edk2")])
        // The first byte of a testcase selects the path of the test application: 'A' is an
        // invalid opcode, 'B' a hang, 'C' a page fault, anything else returns normally.
        .files([
            (
                "no_buffer_harness.py".to_string(),
                NO_BUFFER_HARNESS.as_bytes().to_vec(),
            ),
            ("replay-inputs/1_default".to_string(), b"Zdefault".to_vec()),
            ("replay-inputs/2_pagefault".to_string(), b"C".to_vec()),
            ("replay-inputs/3_timeout".to_string(), b"B".to_vec()),
            ("replay-inputs/4_invalid_opcode".to_string(), b"A".to_vec()),
        ])
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module tsffs
            init-tsffs

            @tsffs.log_level = 2
            @tsffs.start_on_harness = False
            @tsffs.stop_on_harness = True
            @tsffs.timeout = 3.0
            @tsffs.exceptions = [14, 6]

            run-script "%simics%/no_buffer_harness.py"
            @tsffs.repro_auto_continue = False
            @tsffs.iface.fuzz.repro("%simics%/replay-inputs")

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

            # Ends at the harness start: no testcase has run yet.
            run
            @print(f"TSFFS_EVENT at start {tsffs.last_stop_reason}")

            # Runs every testcase of the directory
            run
            @print("TSFFS_EVENT directory ended")
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
        // like with the other harnesses
        ["at start", "ManualStartWithoutBuffer"],
        ["replay-inputs/1_default: ", "StopNormal"],
        [
            "replay-inputs/2_pagefault: ",
            r#"{"Exception":{"number":14}}"#,
        ],
        ["replay-inputs/3_timeout: ", "Timeout"],
        [
            "replay-inputs/4_invalid_opcode: ",
            r#"{"Exception":{"number":6}}"#,
        ],
        // The start the restored snapshot reaches again does not end the run
        ["directory ended", ""],
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

    Ok(())
}
