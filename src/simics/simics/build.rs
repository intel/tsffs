use anyhow::{anyhow, Result};
use std::{env::var, fs::write, path::PathBuf};
use version_tools::{VersionConstraint, Versioning};

/// The name of the environment variable set by cargo containing the path to the out directory
/// for intermediate build results
const OUT_DIR_ENV: &str = "OUT_DIR";

use simics_api_sys::{SIMICS_API_BINDINGS, SIMICS_VERSION};

fn main() -> Result<()> {
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    {
        let sys_bindings_path = out_dir.join("bindings.rs");
        write(sys_bindings_path, SIMICS_API_BINDINGS)?;
    }

    // Set configurations to conditionally enable experimental features that aren't
    // compatible with all supported SIMICS versions, based on the SIMICS version of the
    // low level bindings.

    let simics_api_version = Versioning::new(SIMICS_VERSION)
        .ok_or_else(|| anyhow!("Invalid version {}", SIMICS_VERSION))?;

    // The minimum version required to enable the experimental snapshots API
    let snapshots_minimum_version: VersionConstraint = ">=6.0.173".parse()?;

    if snapshots_minimum_version.matches(&simics_api_version) {
        println!("cargo:rustc-cfg=simics_experimental_api_snapshots");
    }

    Ok(())
}
