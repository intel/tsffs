// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Runs a short magic-harness campaign and checks the AFL++-compatible files it leaves behind.

use anyhow::{Context, Result};
use indoc::formatdoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf};

const ITERATION_LIMIT: u64 = 100;
const INSTANCE_NAME: &str = "afl-stats-test";

fn parse_fuzzer_stats(contents: &str) -> Result<HashMap<String, String>> {
    contents
        .lines()
        .map(|line| {
            let (key, value) = line
                .split_once(": ")
                .with_context(|| format!("Malformed fuzzer_stats line: {line:?}"))?;
            Ok((key.trim_end().to_string(), value.to_string()))
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_magic_afl_stats_latest() -> Result<()> {
    let output_directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("test_afl_stats_output");
    let _ = std::fs::remove_dir_all(&output_directory);
    // Simics scripts accept forward slashes on every platform
    let output_directory_str = output_directory.to_string_lossy().replace('\\', "/");

    let script = formatdoc! {r#"
        load-module tsffs
        init-tsffs

        @tsffs.log_level = 2
        @tsffs.start_on_harness = True
        @tsffs.stop_on_harness = True
        @tsffs.timeout = 3.0
        @tsffs.exceptions = [14]
        @tsffs.generate_random_corpus = True
        @tsffs.iteration_limit = {ITERATION_LIMIT}
        @tsffs.output_directory = "{output_directory_str}"
        @tsffs.instance_name = "{INSTANCE_NAME}"
        @tsffs.coverage_reporting = True
        @tsffs.heartbeat = True
        @tsffs.heartbeat_interval = 1
        @tsffs.log_path = "{output_directory_str}/log.json"

        load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

        script-branch {{
            bp.time.wait-for seconds = 15
            qsp.serconsole.con.input "\n"
            bp.time.wait-for seconds = .5
            qsp.serconsole.con.input "FS0:\n"
            bp.time.wait-for seconds = .5
            local $manager = (start-agent-manager)
            qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test.efi") + "\n")
            bp.time.wait-for seconds = .5
            qsp.serconsole.con.input "test.efi\n"
        }}

        script-branch {{
            bp.time.wait-for seconds = 240
            quit 1
        }}

        run
    "#};

    let output = TestEnvSpec::builder()
        .name("test_x86_64_magic_afl_stats_latest")
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
        .test(script)?;

    println!("{}", String::from_utf8_lossy(&output.stdout));

    let instance = output_directory.join(INSTANCE_NAME);

    let stats = parse_fuzzer_stats(
        &read_to_string(instance.join("fuzzer_stats")).context("Failed to read fuzzer_stats")?,
    )?;

    // The final write at shutdown makes the last numbers match the end of the campaign
    assert_eq!(
        stats["execs_done"],
        ITERATION_LIMIT.to_string(),
        "execs_done must equal the iteration limit exactly: {stats:?}"
    );
    assert_eq!(stats["afl_banner"], INSTANCE_NAME);
    assert_eq!(stats["target_mode"], "tsffs");
    assert!(stats["afl_version"].starts_with("tsffs-"));
    assert_eq!(stats["fuzzer_pid"].parse::<u32>().map(|p| p > 0), Ok(true));

    // Edges are occupied coverage-map slots, whether or not coverage reporting is enabled
    let edges_found = stats["edges_found"].parse::<u64>()?;
    assert!(edges_found > 0, "edges_found must be positive: {stats:?}");
    assert!(
        stats["corpus_count"].parse::<u64>()? >= 1,
        "corpus_count must be at least 1: {stats:?}"
    );
    assert_eq!(stats["total_edges"], "131072");

    // Coverage reporting is on, so Branch targets are reported, separately from Edges
    assert!(
        stats["tsffs_branch_targets"].parse::<u64>()? > 0,
        "tsffs_branch_targets must be positive: {stats:?}"
    );

    // log.json keeps its startup and find records but no longer carries stats records
    let log =
        read_to_string(output_directory.join("log.json")).context("Failed to read log.json")?;
    let record_kinds = log
        .lines()
        .map(|line| {
            let record: serde_json::Value = serde_json::from_str(line)?;
            Ok(record
                .as_object()
                .and_then(|o| o.keys().next().cloned())
                .unwrap_or_default())
        })
        .collect::<Result<Vec<String>>>()?;
    assert!(
        record_kinds.iter().any(|k| k == "Startup"),
        "{record_kinds:?}"
    );
    assert!(
        !record_kinds
            .iter()
            .any(|k| k == "Heartbeat" || k == "Message"),
        "log.json must not contain heartbeat or message records: {record_kinds:?}"
    );
    for key in [
        "cycles_done",
        "cycles_wo_finds",
        "pending_total",
        "pending_favs",
        "cur_item",
        "max_depth",
    ] {
        assert!(stats.contains_key(key), "missing {key}: {stats:?}");
    }
    let bitmap_cvg = stats["bitmap_cvg"].trim_end_matches('%').parse::<f64>()?;
    assert!(bitmap_cvg > 0.0 && bitmap_cvg <= 100.0, "{bitmap_cvg}");

    Ok(())
}
