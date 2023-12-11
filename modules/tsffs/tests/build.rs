use anyhow::{anyhow, Result};
use command_ext::CommandExtCheck;
use ispm_wrapper::ispm::{self, GlobalOptions};
use simics_codegen::simics_tests;
use std::{env::var, fs::write, path::PathBuf, process::Command};

const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
const OUT_DIR_ENV: &str = "OUT_DIR";
const TESTS_FILE: &str = "tests.rs";

fn main() -> Result<()> {
    let packages = ispm::packages::list(&GlobalOptions::default())?;
    let base = packages
        .installed_packages_ref()
        .as_ref()
        .ok_or_else(|| anyhow!("No installed packages"))?
        .iter()
        .find(|p| p.package_number_deref() == 1000isize)
        .ok_or_else(|| anyhow!("No base in installed packages"))?;

    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );
    let tests_out_file = out_dir.join(TESTS_FILE);

    let tests_tokens = simics_tests("../../../");
    write(tests_out_file, tests_tokens.to_string())?;

    let manifest_dir = PathBuf::from(
        var(CARGO_MANIFEST_DIR)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );
    let targets_dir = manifest_dir.join("../../../examples/tests/");

    println!("cargo:rerun-if-changed={}", targets_dir.display());

    if var("TSFFS_TESTS_SKIP_BUILD").is_ok() {
        println!("Skipping test build");
    } else {
        Command::new(targets_dir.join("build.sh"))
            .current_dir(targets_dir)
            .env(
                "SIMICS_BASE",
                base.paths_ref()
                    .first()
                    .ok_or_else(|| anyhow!("No path to base package"))?
                    .to_string_lossy()
                    .to_string(),
            )
            .check()
            .expect("failed to build");
        println!("cargo:rerun-if-changed=build.rs");
    }

    Ok(())
}
