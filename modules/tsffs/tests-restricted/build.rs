use anyhow::{anyhow, Result};
use command_ext::CommandExtCheck;
use simics_codegen::simics_tests;
use std::{
    env::var,
    fs::{read_dir, write},
    path::PathBuf,
    process::Command,
};

const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
const OUT_DIR_ENV: &str = "OUT_DIR";
const TESTS_FILE: &str = "tests.rs";

fn main() -> Result<()> {
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );
    let tests_out_file = out_dir.join(TESTS_FILE);

    let tests_tokens = simics_tests("../../../");
    write(tests_out_file, tests_tokens.to_string())
        .map_err(|e| anyhow!("Failed to write tests out file: {e}"))?;

    let manifest_dir = PathBuf::from(
        var(CARGO_MANIFEST_DIR)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    let targets_dir = manifest_dir.join("targets");

    if targets_dir.is_dir() {
        read_dir(targets_dir)?
            .filter_map(|d| d.ok())
            .filter(|d| d.path().is_dir())
            .map(|d| d.path())
            .for_each(|d| {
                Command::new("ninja")
                    .current_dir(&d)
                    .check()
                    .expect("failed to build");
                println!("cargo:rerun-if-changed={}", d.display());
            });
    }

    Ok(())
}
