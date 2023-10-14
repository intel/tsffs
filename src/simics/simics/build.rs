use anyhow::{anyhow, Result};
use std::{env::var, fs::write, path::PathBuf};

/// The name of the environment variable set by cargo containing the path to the out directory
/// for intermediate build results
const OUT_DIR_ENV: &str = "OUT_DIR";

use simics_api_sys::SIMICS_API_BINDINGS;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    {
        let sys_bindings_path = out_dir.join("bindings.rs");
        write(sys_bindings_path, SIMICS_API_BINDINGS)?;
    }

    Ok(())
}
