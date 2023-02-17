use anyhow::{bail, Result};
use dotenvy_macro::dotenv;
use serde::Deserialize;
use serde_yaml::from_reader;
use std::{
    collections::HashMap,
    env::var,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};
use version_compare::Version;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct SimicsManifestProfile {
    description: String,
    name: String,
    platform_script: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct SimicsManifestPackage {
    description: Option<String>,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct SimicsManifest {
    manifest_format: u64,
    name: String,
    group: String,
    version: String,
    date: String,
    description: String,
    profiles: HashMap<String, SimicsManifestProfile>,
    packages: HashMap<u64, SimicsManifestPackage>,
}

/// SIMICS_HOME must be provided containing a working SIMICS installation
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
const SIMICS_BASE_KEY: &u64 = &1000u64;
const SIMICS_QSP_X86_KEY: &u64 = &2096u64;

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

fn parse_simics_manifest(manifest: &Path) -> Result<SimicsManifest> {
    let manifest_content = read_to_string(manifest)?;
    let manifest: SimicsManifest = from_reader(manifest_content.as_bytes())?;
    Ok(manifest)
}

/// Parse SIMICS manifest(s) to determine the latest simics version and return its manifest
fn simics_latest() -> Result<SimicsManifest> {
    let manifest_dir = simics_home()?.join("manifests");
    let mut manifests = read_dir(&manifest_dir)?
        .filter_map(|de| match de.ok() {
            Some(de) => parse_simics_manifest(&de.path()).ok(),
            None => None,
        })
        .collect::<Vec<_>>();

    manifests.sort_by(|a, b| {
        let aver = Version::from(&a.packages[SIMICS_BASE_KEY].version)
            .expect("Missing manifest entry for base key.");
        let bver = Version::from(&b.packages[SIMICS_BASE_KEY].version)
            .expect("Missing manifest entry for base key.");
        aver.compare(bver).ord().expect("No ordering found")
    });

    match manifests.last() {
        Some(m) => Ok(m.clone()),
        None => bail!("No highest version manifest found."),
    }
}

/// Set up the SIMICS simulator project
///
/// Expects SIMICS_HOME to be set, and errors if it is not. We can ostensibly download a fresh
/// copy of simics, but it's quite large (2G so this should be avoided).
fn setup_simics() -> Result<()> {
    let confuse_simics_project_dir = out_dir()?.join("simics");
    let latest_simics_manifest = simics_latest()?;
    let simics_base_dir = simics_home()?.join(format!(
        "simics-{}",
        latest_simics_manifest.packages[SIMICS_BASE_KEY].version
    ));
    let simics_qsp_x86_dir = simics_home()?.join(format!(
        "simics-qsp-x86-{}",
        latest_simics_manifest.packages[SIMICS_QSP_X86_KEY].version
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
