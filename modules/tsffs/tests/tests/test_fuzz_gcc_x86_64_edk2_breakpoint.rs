// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test fuzzing a UEFI firmware built with EDK2, using a write breakpoint on a region of
//! memory as the solution condition
//!
//! X86-64 architecture

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use std::process::Command;
use tests::{Architecture, TestEnvSpec};

const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const TEST_UEFI: &[u8] = include_bytes!("../targets/minimal-x86_64-breakpoint-edk2/test.efi");

#[test]
fn test_fuzz_gcc_x86_64_edk2_breakpoint() -> Result<()> {
    let script = indoc! {r#"
        load-module tsffs

        @tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
        tsffs.log-level 4
        @tsffs.iface.tsffs.set_start_on_harness(True)
        @tsffs.iface.tsffs.set_stop_on_harness(True)
        @tsffs.iface.tsffs.set_timeout(3.0)
        @tsffs.iface.tsffs.add_exception_solution(14)
        @tsffs.iface.tsffs.add_exception_solution(6)
        @tsffs.iface.tsffs.set_generate_random_corpus(True)
        @tsffs.iface.tsffs.set_iterations(1000)
        @tsffs.iface.tsffs.set_use_snapshots(True)
        @tsffs.iface.tsffs.set_all_breakpoints_are_solutions(True)

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
            local $BP_BUFFER_ADDRESS = 0x4000000
            local $MAGIC_START = 1
            bp.magic.wait-for $MAGIC_START
            local $ctx = (new-context)
            qsp.mb.cpu0.core[0][0].set-context $ctx
            $ctx.break -w $BP_BUFFER_ADDRESS 0x1000
        }

        script-branch {
            bp.time.wait-for seconds = 240
            quit 1
        }

        run

    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_x86_64-edk2-breakpoint")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .files(vec![
            ("test.simics".to_string(), script.as_bytes().to_vec()),
            ("test.efi".to_string(), TEST_UEFI.to_vec()),
            ("minimal_boot_disk.craff".to_string(), BOOT_DISK.to_vec()),
        ])
        .arch(Architecture::X86)
        .build()
        .to_env()?;

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test.simics")
        .check()?;

    let _output_str = String::from_utf8_lossy(&output.stdout);

    Ok(())
}