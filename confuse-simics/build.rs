use anyhow::{bail, Result};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use dotenvy_macro::dotenv;
use std::{env::var, path::PathBuf, process::Command};

/// SIMICS_HOME must be provided containing a working SIMICS installation
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

/// Return the OUT_DIR build directory as a PathBuf
fn out_dir() -> Result<PathBuf> {
    match var("OUT_DIR") {
        Ok(out_dir) => Ok(PathBuf::from(out_dir)),
        Err(e) => Err(e.into()),
    }
}

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

/// Set up the SIMICS simulator project
///
/// Expects SIMICS_HOME to be set, and errors if it is not. We can ostensibly download a fresh
/// copy of simics, but it's quite large (2G so this should be avoided).
fn setup_simics() -> Result<()> {
    let confuse_simics_project_dir = out_dir()?.join("simics");
    let latest_simics_manifest = simics_latest(simics_home()?)?;
    let simics_base_dir = simics_home()?.join(format!(
        "simics-{}",
        latest_simics_manifest.packages[&PackageNumber::Base].version
    ));
    let simics_qsp_x86_dir = simics_home()?.join(format!(
        "simics-qsp-x86-{}",
        latest_simics_manifest.packages[&PackageNumber::QuickStartPlatform].version
    ));

    assert!(
        simics_base_dir.exists(),
        "Simics base directory does not exist. Is install broken?"
    );
    assert!(
        simics_qsp_x86_dir.exists(),
        "Simics QSP directory does not exist. Is install broken?"
    );

    let simics_base_project_setup = simics_base_dir.join("bin").join("project-setup");

    assert!(
        simics_base_project_setup.exists(),
        "Simics project-setup tool not found."
    );

    Command::new(simics_base_project_setup)
        .arg("--ignore-existing-files")
        .arg(&confuse_simics_project_dir)
        .output()?;

    let _simics_project_project_setup =
        confuse_simics_project_dir.join("bin").join("project-setup");

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    setup_simics()?;
    Ok(())
}
