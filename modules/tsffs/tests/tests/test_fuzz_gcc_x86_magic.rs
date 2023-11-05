//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use tests::{Architecture, TestEnvSpec};

const TEST_USER: &[u8] = include_bytes!("../targets/minimal-x86-user/test");

#[test]
fn test_fuzz_gcc_x86_magic() -> Result<()> {
    let script = indoc! {r#"
        import simics
        import cli

        simics.SIM_load_module("tsffs")

        tsffs = simics.SIM_create_object(
            simics.SIM_get_class("tsffs"),
            "tsffs",
            []
        )
        simics.SIM_set_log_level(tsffs, 4)
        tsffs.iface.tsffs.set_start_on_harness(True)
        tsffs.iface.tsffs.set_stop_on_harness(True)
        tsffs.iface.tsffs.set_timeout(3.0)
        tsffs.iface.tsffs.set_generate_random_corpus(True)
        tsffs.iface.tsffs.set_iterations(1000)
        tsffs.iface.tsffs.set_use_snapshots(True)

        simics.SIM_load_target(
            "qsp-x86/clear-linux", # Target
            "qsp", # Namespace
            [],  # Presets
            [["machine:hardware:storage:disk1:image", "test.fs.craff"]],
        )

        qsp = simics.SIM_get_object("qsp")

        tsffs.iface.tsffs.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")

        # When we're running userspace code, we don't want to catch exeptions until
        # we actually start fuzzing, including GPFs on other code. We can wait to
        # enable the exception until later (we could even toggle it on and off per
        # iteration)
        def on_magic(o, e, r):
            # Wait for magic stop -- in reality this could wait for any stop
            # condition, but we make it easy on ourselves for testing purposes
            if r == 1:
                tsffs.iface.tsffs.add_exception_solution(13)

        def startup_script_branch():
            cli.global_cmds.wait_for_global_time(seconds=20.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("sudo mkdir /disk0/\n")
            cli.global_cmds.wait_for_global_time(seconds=1.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("sudo mount /dev/sdb /disk0/\n")
            cli.global_cmds.wait_for_global_time(seconds=1.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("ls /disk0\n")
            cli.global_cmds.wait_for_global_time(seconds=1.0, _relative = True)
            qsp.serconsole.con.iface.con_input.input_str("/disk0/test\n")


        def exit_script_branch():
            cli.global_cmds.wait_for_global_time(seconds=240.0, _relative = True)
            simics.SIM_quit(1)

        simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
        cli.sb_create(startup_script_branch)
        cli.sb_create(exit_script_branch)

        simics.SIM_continue(0)
        # NOTE: If running from CLI, omit this!
        simics.SIM_main_loop()
    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_x86_magic")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .files(vec![
            ("test.py".to_string(), script.as_bytes().to_vec()),
            ("test".to_string(), TEST_USER.to_vec()),
        ])
        .arch(Architecture::X86)
        .extra_packages([
            ProjectPackage::builder()
                .package_number(1030)
                .version("latest")
                .build(),
            ProjectPackage::builder()
                .package_number(4094)
                .version("latest")
                .build(),
        ])
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
        .arg(env.project_dir().join("test"))
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
        .arg("test.py")
        .check()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    Ok(())
}
