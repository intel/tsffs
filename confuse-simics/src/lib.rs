pub mod api;

use std::path::Path;

use anyhow::Result;
use dotenvy_macro::dotenv;
use include_dir::{include_dir, Dir};

use confuse_simics_manifest::PackageNumber;

const OUT_DIR: &str = env!("OUT_DIR");
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

static SIMICS_PROJECT_DIR: Dir<'_> = include_dir!("$OUT_DIR/simics");

/// Set up a SIMICs project with a specified set of packages
pub fn setup_simics_project<P: AsRef<Path>>(
    base_path: P,
    packages: Vec<PackageNumber>,
) -> Result<()> {
    SIMICS_PROJECT_DIR.extract(base_path)?;

    Ok(())
}
