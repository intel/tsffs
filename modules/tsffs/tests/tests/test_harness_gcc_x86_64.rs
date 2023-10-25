//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use tests::{Architecture, TestEnvSpec};

const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const TEST_UEFI: &[u8] = include_bytes!("../targets/minimal-x86_64/test.efi");

#[test]
fn test_harness_gcc_x86_64() -> Result<()> {
    let script = indoc! {r#"
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
            local $MAGIC_START = 1
            local $MAGIC_STOP = 2

            echo "Waiting for MAGIC_START"
            bp.magic.wait-for $MAGIC_START
            echo "Got magic start"

            local $rsi = (qsp.mb.cpu0.core[0][0].read-reg rsi)
            local $rdi = (qsp.mb.cpu0.core[0][0].read-reg rdi)

            echo "Buffer: "
            print -x $rsi
            x %rsi 8
            echo "Size: "
            print -x $rdi
            # This should be the size of the buffer (8 bytes)
            x %rdi 8

            echo "Waiting for MAGIC_STOP"

            bp.magic.wait-for $MAGIC_STOP

            echo "Got magic stop"
            
            exit 0
        }


        script-branch {
            bp.time.wait-for seconds = 30
            exit 1
        }

        run

    "#};

    let env = TestEnvSpec::builder()
        .name("harness_gcc_x86_64")
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

    assert!(
        output_str.contains("4141 4141 4141 4141"),
        "Output does not contain initial buffer"
    );
    assert!(
        output_str.contains("0800 0000 0000 0000"),
        "Output does not contain initial size"
    );

    Ok(())
}
