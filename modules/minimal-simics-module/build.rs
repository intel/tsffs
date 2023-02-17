use anyhow::{bail, Result};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use dotenvy_macro::dotenv;
use std::path::PathBuf;

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

/// Return the SIMICS_HOME directory as a PathBuf
fn simics_home() -> Result<PathBuf> {
    let simics_home = PathBuf::from(SIMICS_HOME);
    match simics_home.exists() {
        true => Ok(simics_home),
        false => {
            bail!(
                "SIMICS_HOME is defined, but {} does not exist.",
                SIMICS_HOME
            )
        }
    }
}

fn main() -> Result<()> {
    let simics_bin_dir = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("bin");

    println!(
        "cargo:rustc-link-search=native={}",
        simics_bin_dir.display()
    );
    println!("cargo:rustc-link-lib=simics-common");
    println!("cargo:rustc-link-lib=vtutils");
    Ok(())
}
