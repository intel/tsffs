use anyhow::{anyhow, Result};
use simics_codegen::simics_tests;
use std::{env::var, fs::write, path::PathBuf};

const OUT_DIR_ENV: &str = "OUT_DIR";
const TESTS_FILE: &str = "tests.rs";

fn main() -> Result<()> {
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );
    let tests_out_file = out_dir.join(TESTS_FILE);

    let tests_tokens = simics_tests("../../../");
    write(&tests_out_file, tests_tokens.to_string())?;

    Ok(())
}
