//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use tests::{Architecture, TestEnvSpec};

// const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const IMAGE: &[u8] = include_bytes!("../targets/minimal-riscv-64/Image");
const ROOTFS: &[u8] = include_bytes!("../targets/minimal-riscv-64/rootfs.ext2");
const FW_JUMP: &[u8] = include_bytes!("../targets/minimal-riscv-64/fw_jump.elf");
const TEST: &[u8] = include_bytes!("../targets/minimal-riscv-64/test");

#[test]
fn test_fuzz_gcc_riscv_64_magic() -> Result<()> {
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
            board.console.con.capture-start out.txt
            board.console.con.input "/mnt/disk0/test\r\n"
        }

        script-branch {
            bp.time.wait-for seconds = 30
            echo "Exiting..."
            !cat out.txt
            exit 0
        }

        run
    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_riscv_64_magic")
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
                "targets/risc-v-simple/images/linux/test".to_string(),
                TEST.to_vec(),
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
        .arg("bs=1024")
        .arg("count=4194304")
        .check()?;
    Command::new("mkfs.fat")
        .arg(env.project_dir().join("test.fs"))
        .check()?;
    Command::new("mcopy")
        .arg("-i")
        .arg(env.project_dir().join("test.fs"))
        .arg(
            env.project_dir()
                .join("targets/risc-v-simple/images/linux/test"),
        )
        .arg("::test")
        .check()?;
    Command::new(craff)
        .arg("-o")
        .arg(env.project_dir().join("test.fs.craff"))
        .arg(env.project_dir().join("test.fs"))
        .check()?;

    // NOTE:
    // You can connect to the qsp-x86/uefi-shell
    // machine by running `qsp.serconsole.con.telnet-setup /path/to/telnet.sock
    // then connect with
    // socat -,rawer,escape=0x1d unix-connect:/path/to/telnet.sock
    //
    // An empty FAT fs craff can be created with:
    // dd if=/dev/zero of=fat.fs bs=1024 count=4096
    // mkfs.fat fat.fs
    // /path/to/craff -o fat.fs.craff fat.fs
    //

    let output = Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test.simics")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    Ok(())
}
