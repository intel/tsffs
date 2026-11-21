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

/// The stop reason TSFFS reports for a page fault, as `last_stop_reason` serializes it
const PAGE_FAULT: &str = r#"{"Solution":{"kind":{"Exception":{"number":14}}}}"#;

/// The stop reason TSFFS reports for an invalid opcode
const INVALID_OPCODE: &str = r#"{"Solution":{"kind":{"Exception":{"number":6}}}}"#;

/// Replays a directory of inputs with `repro` when the harness starts with
/// `start_without_buffer`: every input is handed to the harness and runs from the
/// snapshot, and one resume per input ends at that input's own stop.
#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_repro_no_buffer_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_repro_no_buffer_6_0_185")
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
        // invalid opcode, 'B' a hang, 'C' a page fault, anything else returns normally.
        .files([
            (
                "replay.py".to_string(),
                include_bytes!("../examples/source-coverage/replay.py").to_vec(),
            ),
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
            @tsffs.generate_random_corpus = True

            run-script "%simics%/replay.py"
            run-script "%simics%/no_buffer_harness.py"
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

            # Ends at the harness start: no input has run yet.
            run
            @print(f"TSFFS_ARMED {tsffs.last_stop_reason}")

            @tsffs_replay("%simics%/replay-inputs")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    // With `repro_auto_continue = False`, which the helper sets, the first run stops at
    // the harness start like the other harnesses do, before the first input runs.
    assert!(
        output_str.contains(r#"TSFFS_ARMED {"ManualStartWithoutBuffer":{}}"#),
        "The run that reaches the harness did not end at the start_without_buffer stop"
    );

    for (input, expected) in [
        ("1/4 1_default", "StopNormal"),
        ("2/4 2_pagefault", PAGE_FAULT),
        ("3/4 3_timeout", "Timeout"),
        ("4/4 4_invalid_opcode", INVALID_OPCODE),
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
        output_str.contains("TSFFS_REPLAY done 4"),
        "The replay did not run to the end of the directory"
    );

    Ok(())
}
