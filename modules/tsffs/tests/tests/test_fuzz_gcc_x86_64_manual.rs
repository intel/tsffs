//! Test that we can load TSFFS in a new project

use std::process::Command;

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use tests::{Architecture, TestEnvSpec};

const BOOT_DISK: &[u8] = include_bytes!("../rsrc/minimal_boot_disk.craff");
const TEST_UEFI: &[u8] = include_bytes!("../targets/minimal-x86_64/test.efi");

#[test]
fn test_fuzz_gcc_x86_64_manual() -> Result<()> {
    let script = indoc! {r#"
        import simics
        import cli

        simics.SIM_load_module("tsffs")

        tsffs = simics.SIM_create_object(
            simics.SIM_get_class("tsffs"),
            "tsffs",
            []
        )
        simics.SIM_set_log_level(tsffs, 1)
        tsffs.iface.tsffs.set_start_on_harness(False)
        tsffs.iface.tsffs.set_stop_on_harness(False)
        tsffs.iface.tsffs.set_timeout(3.0)
        tsffs.iface.tsffs.add_exception_solution(14)
        tsffs.iface.tsffs.set_generate_random_corpus(True)
        tsffs.iface.tsffs.set_iterations(1000)
        tsffs.iface.tsffs.set_use_snapshots(False)

        simics.SIM_load_target(
            "qsp-x86/uefi-shell", # Target
            "qsp", # Namespace
            [],  # Presets
            [ # Cmdline args
                ["machine:hardware:storage:disk0:image", "minimal_boot_disk.craff"],
                ["machine:hardware:processor:class", "x86-goldencove-server"]
            ]
        )

        qsp = simics.SIM_get_object("qsp")
        
        def on_magic(o, e, r):
            # Wait for magic stop -- in reality this could wait for any stop
            # condition, but we make it easy on ourselves for testing purposes
            if r == 2:
                print("Got magic stop...")
                tsffs.iface.tsffs.stop()
        
        def start_script_branch():
            # Wait for magic start -- in reality this could wait for any
            # start condition, but we make it easy on ourselves for testing purposes
            print("Waiting for magic start...")
            conf.bp.magic.cli_cmds.wait_for(number=1)
            print("Got magic start...")

            # In reality, you probably have a known buffer in mind to fuzz
            testcase_address_regno = conf.qsp.mb.cpu0.core[0][0].iface.int_register.get_number("rdi")
            print("testcase address regno: ", testcase_address_regno)
            testcase_address = conf.qsp.mb.cpu0.core[0][0].iface.int_register.read(testcase_address_regno)
            print("testcase address: ", testcase_address)
            size_regno = conf.qsp.mb.cpu0.core[0][0].iface.int_register.get_number("rsi")
            print("size regno: ", size_regno)
            size_address = conf.qsp.mb.cpu0.core[0][0].iface.int_register.read(size_regno)
            print("size address: ", size_address)
            virt = False

            print(
                "Starting with testcase address",
                hex(testcase_address),
                "size address",
                hex(size_address),
                "virt",
                virt
            )
            
            tsffs.iface.tsffs.start(
                conf.qsp.mb.cpu0.core[0][0],
                testcase_address,
                size_address,
                virt
            )



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

        simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
        cli.sb_create(start_script_branch)
        cli.sb_create(startup_script_branch)

        simics.SIM_continue(0)
        simics.SIM_main_loop()
    "#};

    let env = TestEnvSpec::builder()
        .name("fuzz_gcc_x86_64_manual")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .files(vec![
            ("test.py".to_string(), script.as_bytes().to_vec()),
            ("test.efi".to_string(), TEST_UEFI.to_vec()),
            ("minimal_boot_disk.craff".to_string(), BOOT_DISK.to_vec()),
        ])
        .arch(Architecture::X86)
        .extra_packages([ProjectPackage::builder()
            .package_number(1030)
            .version("latest")
            .build()])
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