// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_userspace_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_userspace_6_0_185")
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
            ProjectPackage::builder()
                .package_number(1030)
                .version("6.0.8")
                .build(),
            ProjectPackage::builder()
                .package_number(4094)
                .version("6.0.15")
                .build(),
        ])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86-user")])
        .build()
        .to_env()?
        .test_python(indoc! {r#"
            import cli
            import simics

            simics.SIM_load_module("tsffs")

            tsffs = simics.SIM_create_object(simics.SIM_get_class("tsffs"), "tsffs", [])
            simics.SIM_set_log_level(tsffs, 2)
            tsffs.start_on_harness = True
            tsffs.stop_on_harness = True
            tsffs.timeout = 3.0
            tsffs.generate_random_corpus = True
            tsffs.iteration_limit = 100

            simics.SIM_load_target(
                "qsp-x86/clear-linux",  # target
                "qsp",  # namespace
                [],  # presets
                [["machine:hardware:storage:disk1:image", "test.fs.craff"]],
            )

            qsp = simics.SIM_get_object("qsp")

            tsffs.iface.config.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")


            # when we're running userspace code, we don't want to catch exeptions until
            # we actually start fuzzing, including gpfs on other code. we can wait to
            # enable the exception until later (we could even toggle it on and off per
            # iteration)
            def on_magic(o, e, r):
                # wait for magic stop -- in reality this could wait for any stop
                # condition, but we make it easy on ourselves for testing purposes
                if r == 1:
                    tsffs.exceptions = [13]


            def startup_script_branch():
                cli.global_cmds.wait_for_global_time(seconds=20.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("sudo mkdir /disk0/\n")
                cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("sudo mount /dev/sdb /disk0/\n")
                cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("ls /disk0\n")
                cli.global_cmds.wait_for_global_time(seconds=1.0, _relative=True)
                qsp.serconsole.con.iface.con_input.input_str("/disk0/test\n")


            def exit_script_branch():
                cli.global_cmds.wait_for_global_time(seconds=240.0, _relative=True)
                simics.SIM_quit(1)


            simics.SIM_hap_add_callback("Core_Magic_Instruction", on_magic, None)
            cli.sb_create(startup_script_branch)
            cli.sb_create(exit_script_branch)

            simics.SIM_continue(0)
            # note: if running from cli, omit this!
            simics.SIM_main_loop()
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    Ok(())
}
