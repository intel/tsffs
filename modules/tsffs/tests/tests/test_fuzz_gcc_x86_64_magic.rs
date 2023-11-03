//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use tests::{Architecture, TestEnvSpec};

const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const TEST_UEFI: &[u8] = include_bytes!("../targets/minimal-x86_64/test.efi");

#[test]
fn test_fuzz_gcc_x86_64_magic() -> Result<()> {
    let script = indoc! {r#"
        load-module tsffs

        @tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
        tsffs.log-level 1
        @tsffs.iface.tsffs.set_start_on_harness(True)
        @tsffs.iface.tsffs.set_stop_on_harness(True)
        @tsffs.iface.tsffs.set_timeout(3.0)
        @tsffs.iface.tsffs.add_exception_solution(14)
        @tsffs.iface.tsffs.set_generate_random_corpus(True)
        @tsffs.iface.tsffs.set_iterations(1000)
        @tsffs.iface.tsffs.set_use_snapshots(True)

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

    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_x86_64_magic")
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
