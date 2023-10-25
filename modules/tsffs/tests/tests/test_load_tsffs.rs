//! Test that we can load TSFFS in a new project

use anyhow::Result;
use command_ext::CommandExtCheck;
use indoc::indoc;
use std::process::Command;
use tests::{Architecture, TestEnvSpec};

#[test]
fn test_load_tsffs() -> Result<()> {
    let script = indoc! {r#"
        load-module tsffs
        @tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
        @tsffs.iface.tsffs.set_start_on_harness(True)
        @tsffs.iface.tsffs.set_stop_on_harness(True)
        @tsffs.iface.tsffs.set_use_snapshots(True)
        @tsffs.iface.tsffs.set_timeout(60.0)
        @tsffs.iface.tsffs.add_exception_solution(6)
        @tsffs.iface.tsffs.add_exception_solution(14)
        @tsffs.iface.tsffs.remove_exception_solution(6)
        @tsffs.iface.tsffs.set_all_exceptions_are_solutions(True)
        @tsffs.iface.tsffs.set_all_exceptions_are_solutions(False)
        @tsffs.iface.tsffs.add_breakpoint_solution(0)
        @tsffs.iface.tsffs.add_breakpoint_solution(1)
        @tsffs.iface.tsffs.remove_breakpoint_solution(0)
        @tsffs.iface.tsffs.set_all_breakpoints_are_solutions(True)
        @tsffs.iface.tsffs.set_all_breakpoints_are_solutions(False)
        @tsffs.iface.tsffs.set_tracing_mode("once")
        @tsffs.iface.tsffs.set_cmplog_enabled(False)
        # tsffs.iface.tsffs.set_corpus_directory("%simics%/corpus/")
        # tsffs.iface.tsffs.set_solutions_directory("%simics%/solutions")
        @tsffs.iface.tsffs.set_generate_random_corpus(True)
        @tsffs_config = tsffs.iface.tsffs.get_configuration()
        @print(tsffs_config)
    "#};

    let env = TestEnvSpec::builder()
        .name("load")
        .cargo_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .arch(Architecture::X86)
        .files(vec![(
            "test.simics".to_string(),
            script.as_bytes().to_vec(),
        )])
        .build()
        .to_env()?;

    Command::new("./simics")
        .current_dir(env.project_dir())
        .arg("--batch-mode")
        .arg("-no-gui")
        .arg("--no-win")
        .arg("test.simics")
        .check()?;

    Ok(())
}
