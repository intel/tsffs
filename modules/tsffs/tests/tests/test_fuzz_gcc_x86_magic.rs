//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use tests::{Architecture, TestEnvSpec};

const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const TEST_UEFI: &[u8] = include_bytes!("../targets/minimal-x86/test.efi");

#[test]
fn test_fuzz_gcc_x86_magic() -> Result<()> {
    let script = indoc! {r#"
        import simics
        import cli

        simics.SIM_load_module("tsffs")
        simics.SIM_load_module("x86-intel64")

        tsffs = simics.SIM_create_object(
            simics.SIM_get_class("tsffs"),
            "tsffs",
            []
        )
        simics.SIM_set_log_level(tsffs, 4)
        tsffs.iface.tsffs.set_start_on_harness(True)
        tsffs.iface.tsffs.set_stop_on_harness(True)
        tsffs.iface.tsffs.set_timeout(3.0)
        tsffs.iface.tsffs.add_exception_solution(14)
        tsffs.iface.tsffs.set_generate_random_corpus(True)
        tsffs.iface.tsffs.set_iterations(1000)
        tsffs.iface.tsffs.set_use_snapshots(True)

        simics.SIM_load_target(
            "qsp-x86/uefi-shell", # Target
            "qsp", # Namespace
            [],  # Presets
            [ # Cmdline args
                ["machine:hardware:storage:disk0:image", "minimal_boot_disk.craff"],
                ["machine:hardware:processor:class", "x86-nehalem"]
            ]
        )

        qsp = simics.SIM_get_object("qsp")

        def startup_script_branch():
            cli.global_cmds.wait_for_global_time(seconds=15.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("\n")
            cli.global_cmds.wait_for_global_time(seconds=1.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("FS0:\n")
            cli.global_cmds.wait_for_global_time(seconds=1.0, _relative = True)
            cli.global_cmds.start_agent_manager()
            qsp.serconsole.con.iface.con_input.input_str(
                "SimicsAgent.efi --download " + simics.SIM_lookup_file("%simics%/test.efi") + "\n"
            )
            cli.global_cmds.wait_for_global_time(seconds=3.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("test.efi\n")

        cli.sb_create(startup_script_branch)

        simics.SIM_continue(0)
        simics.SIM_main_loop()
    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_x86_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .files(vec![
            ("test.py".to_string(), script.as_bytes().to_vec()),
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
        .arg("test.py")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    Ok(())
}
