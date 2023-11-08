// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Test for fuzzing a kernel module using a harness directly in the kernel module
//! RISC-V architecture

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use std::process::Command;
use tests::{Architecture, TestEnvSpec};

// const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const IMAGE: &[u8] = include_bytes!("../targets/minimal-riscv-64/Image");
const ROOTFS: &[u8] = include_bytes!("../targets/minimal-riscv-64/rootfs.ext2");
const FW_JUMP: &[u8] = include_bytes!("../targets/minimal-riscv-64/fw_jump.elf");
const TEST_MOD: &[u8] = include_bytes!("../targets/minimal-riscv-64/test-mod");
const TEST_KO: &[u8] = include_bytes!("../targets/minimal-riscv-64/test-mod.ko");

#[test]
#[cfg_attr(miri, ignore)]
fn test_fuzz_gcc_riscv_64_kernel_magic() -> Result<()> {
    let script = indoc! {r#"
        load-module tsffs

        @tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
        tsffs.log-level 4
        @tsffs.iface.tsffs.set_start_on_harness(True)
        @tsffs.iface.tsffs.set_stop_on_harness(True)
        @tsffs.iface.tsffs.set_timeout(3.0)
        @tsffs.iface.tsffs.add_exception_solution(14)
        @tsffs.iface.tsffs.set_generate_random_corpus(True)
        @tsffs.iface.tsffs.set_iterations(1000)
        @tsffs.iface.tsffs.set_use_snapshots(True)

        load-target "risc-v-simple/linux" namespace = riscv machine:hardware:storage:disk1:image = "test.fs.craff"

        script-branch {
            bp.time.wait-for seconds = 15
            board.console.con.input "mkdir /mnt/disk0\r\n"
            bp.time.wait-for seconds = 1.0
            board.console.con.input "mount /dev/vdb /mnt/disk0\r\n"
            bp.time.wait-for seconds = 1.0
            board.console.con.input "insmod /mnt/disk0/test-mod.ko\r\n"
            bp.time.wait-for seconds = 1.0
            board.console.con.input "/mnt/disk0/test-mod\r\n"
        }

        script-branch {
            bp.time.wait-for seconds = 240
            quit 1
        }

        run
    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_riscv_64_kernel_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .files([
            ("test.simics".to_string(), script.as_bytes().to_vec()),
            (
                "targets/risc-v-simple/images/linux/Image".to_string(),
                IMAGE.to_vec(),
            ),
            (
                "targets/risc-v-simple/images/linux/rootfs.ext2".to_string(),
                ROOTFS.to_vec(),
            ),
            (
                "targets/risc-v-simple/images/linux/fw_jump.elf".to_string(),
                FW_JUMP.to_vec(),
            ),
            (
                "targets/risc-v-simple/images/linux/test-mod".to_string(),
                TEST_MOD.to_vec(),
            ),
            (
                "targets/risc-v-simple/images/linux/test-mod.ko".to_string(),
                TEST_KO.to_vec(),
            ),
        ])
        .arch(Architecture::Riscv)
        .build()
        .to_env()?;

    let base = env.simics_base_dir()?;
    let craff = base.join("linux64").join("bin").join("craff");

    Command::new("dd")
        .arg("if=/dev/zero")
        .arg(format!(
            "of={}",
            env.project_dir().join("test.fs").display()
        ))
        // Create a 128MB disk
        .arg("bs=1024")
        .arg("count=131072")
        .check()?;
    Command::new("mkfs.fat")
        .arg(env.project_dir().join("test.fs"))
        .check()?;
    Command::new("mcopy")
        .arg("-i")
        .arg(env.project_dir().join("test.fs"))
        .arg(
            env.project_dir()
                .join("targets/risc-v-simple/images/linux/test-mod"),
        )
        .arg("::test-mod")
        .check()?;
    Command::new("mcopy")
        .arg("-i")
        .arg(env.project_dir().join("test.fs"))
        .arg(
            env.project_dir()
                .join("targets/risc-v-simple/images/linux/test-mod.ko"),
        )
        .arg("::test-mod.ko")
        .check()?;
    Command::new(craff)
        .arg("-o")
        .arg(env.project_dir().join("test.fs.craff"))
        .arg(env.project_dir().join("test.fs"))
        .check()?;

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
