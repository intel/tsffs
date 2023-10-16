use anyhow::{anyhow, Result};
use std::{env::var, fs::write, path::PathBuf, str::FromStr};
use version_tools::{VersionConstraint, Versioning};

/// The name of the environment variable set by cargo containing the path to the out directory
/// for intermediate build results
const OUT_DIR_ENV: &str = "OUT_DIR";

use simics_api_sys::{SIMICS_API_BINDINGS, SIMICS_VERSION};
use simics_codegen::{simics_hap_codegen, simics_interface_codegen};

const INTERFACES_FILE: &str = "interfaces.rs";
const HAPS_FILE: &str = "haps.rs";

fn main() -> Result<()> {
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    // Write intermediate auto-generated high level bindings for interfaces and haps

    let interfaces_out_file = out_dir.join(INTERFACES_FILE);
    let haps_out_file = out_dir.join(HAPS_FILE);

    let interfaces_tokens = simics_interface_codegen(SIMICS_API_BINDINGS);
    let haps_tokens = simics_hap_codegen(SIMICS_API_BINDINGS);

    write(interfaces_out_file, interfaces_tokens.to_string())?;
    write(haps_out_file, haps_tokens.to_string())?;

    // Set configurations to conditionally enable experimental features that aren't
    // compatible with all supported SIMICS versions, based on the SIMICS version of the
    // low level bindings.

    let simics_api_version = Versioning::new(SIMICS_VERSION)
        .ok_or_else(|| anyhow!("Invalid version {}", SIMICS_VERSION))?;

    if VersionConstraint::from_str("<6.0.163")?.matches(&simics_api_version) {
        // Bail out if we are targeting a version before 6.0.163. We don't test any earlier than
        // this.
        panic!("Target SIMICS API version is too old. The minimum version supported is 6.0.163.");
    }

    if VersionConstraint::from_str(">=6.0.173")?.matches(&simics_api_version) {
        // Enable the experimental snapshots api for versions over 6.0.173 (where the API first
        // appears)
        println!("cargo:rustc-cfg=simics_experimental_api_snapshots");
    }

    Ok(())
}
