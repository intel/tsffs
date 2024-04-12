// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_manual_latest() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_manual_latest")
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
            .join("x86_64-uefi")])
        .build()
        .to_env()?
        .test_python(indoc! {r#"
            import cli
            import simics

            simics.SIM_load_module("tsffs")

            tsffs = simics.SIM_create_object(simics.SIM_get_class("tsffs"), "tsffs", [])
            simics.SIM_set_log_level(tsffs, 4)
            tsffs.start_on_harness = False
            tsffs.stop_on_harness = False
            tsffs.timeout = 3.0
            tsffs.exceptions = [14]
            tsffs.generate_random_corpus = True
            tsffs.iteration_limit = 100

            simics.SIM_load_target(
                "qsp-x86/uefi-shell",  # Target
                "qsp",  # Namespace
                [],  # Presets
                [  # Cmdline args
                    ["machine:hardware:storage:disk0:image", "minimal_boot_disk.craff"],
                    ["machine:hardware:processor:class", "x86-goldencove-server"],
                ],
            )

            qsp = simics.SIM_get_object("qsp")


            def on_magic(o, e, r):
                if r == 2:
                    print("Got magic stop...")
                    tsffs.iface.fuzz.stop()


            def start_script_branch():
                # Wait for magic start -- in reality this could wait for any
                # start condition, but we make it easy on ourselves for testing purposes
                print("Waiting for magic start...")
                conf.bp.magic.cli_cmds.wait_for(number=1)
                print("Got magic start...")

                # In reality, you probably have a known buffer in mind to fuzz
                testcase_address_regno = conf.qsp.mb.cpu0.core[0][0].iface.int_register.get_number(
                    "rsi"
                )
                print("testcase address regno: ", testcase_address_regno)
                testcase_address = conf.qsp.mb.cpu0.core[0][0].iface.int_register.read(
                    testcase_address_regno
                )
                print("testcase address: ", testcase_address)
                size_regno = conf.qsp.mb.cpu0.core[0][0].iface.int_register.get_number("rdx")
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
                    virt,
                )

                tsffs.iface.fuzz.start_with_buffer_ptr_size_ptr(
                    conf.qsp.mb.cpu0.core[0][0],
                    testcase_address,
                    size_address,
                    True,
                )


            def startup_script_branch():
                cli.global_cmds.wait_for_global_time(seconds=15.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("\n")
                cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("FS0:\n")
                cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
                cli.global_cmds.start_agent_manager()
                qsp.serconsole.con.iface.con_input.input_str(
                    "SimicsAgent.efi --download "
                    + simics.SIM_lookup_file("%simics%/test.efi")
                    + "\n"
                )
                cli.global_cmds.wait_for_global_time(seconds=3.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("test.efi\n")


            def exit_script_branch():
                cli.global_cmds.wait_for_global_time(seconds=240.0, _relative=True)
                simics.SIM_quit(1)

            def on_magic(o, e, r):
                if r == 2:
                    print("Got magic stop...")
                    tsffs.iface.fuzz.stop()

            simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
            cli.sb_create(start_script_branch)
            cli.sb_create(startup_script_branch)
            cli.sb_create(exit_script_branch)

            simics.SIM_continue(0)
            # NOTE: If running from CLI, omit this!
            simics.SIM_main_loop()
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())
}
