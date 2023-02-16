use anyhow::{Context, Result};
use const_format::formatcp;
use flate2::read::GzDecoder;
use reqwest::blocking::get;
use std::{
    env::var,
    fs::{create_dir_all, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};
use tar::Archive;

const SIMICS_ISPM_URL: &str = "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/708028d9-b710-45ea-baab-3b9c78c32cfc/intel-simics-package-manager-1.5.3-linux64.tar.gz";
const SIMICS_PKGS_BUNDLE: &str = "simics-6-packages-2022-49-linux64.ispm";
const SIMICS_PKGS_URL: &str = formatcp!(
    "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/708028d9-b710-45ea-baab-3b9c78c32cfc/{}",
    SIMICS_PKGS_BUNDLE
);

fn out_dir() -> PathBuf {
    let out_dir =
        var("OUT_DIR").expect("OUT_DIR not defined. Is build.rs running under cargo build?");

    PathBuf::from(out_dir)
}

fn download_and_unpack_tgz(url: &str, dest: &Path) -> Result<()> {
    let response = get(url)?;
    let data = response.bytes()?.to_vec();
    let buf_reader = BufReader::new(data.as_slice());
    let gz_reader = GzDecoder::new(buf_reader);
    let mut tar = Archive::new(gz_reader);

    tar.unpack(dest)
        .context(format!("Could not unpack tarball to {}", dest.display()))?;

    Ok(())
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = get(url)?;
    let data = response.bytes()?.to_vec();
    let buf_reader = BufReader::new(data.as_slice());
    let mut outfile =
        File::create(dest).context(format!("Could not create output file {}", dest.display()))?;

    outfile.write_all(buf_reader.buffer())?;

    Ok(())
}

/// Set up the SIMICS simulator
///
/// Expects SIMICS_HOME to be set, and errors if it is not. We can ostensibly download a fresh
/// copy of simics, but it's quite large (2G so this should be avoided).
fn setup_simics() -> Result<()> {
    let simics_home = match var("SIMICS_HOME") {
        Ok(simics_home) => {
            let simics_home = PathBuf::from(simics_home);
            assert!(
                simics_home.exists(),
                "SIMICS_HOME is defined, but does not exist."
            );
            simics_home
        }
        Err(_) => {
            // If SIMICS_HOME isn't set, we'll obtain a local copy and create a simics install
            let simics_home = out_dir().join("simics");
            let simics_bundle_file = out_dir().join(SIMICS_PKGS_BUNDLE);
            let simics_ispm_dir = out_dir().join("ispm");

            if !simics_home.exists() {
                create_dir_all(&simics_home).expect(&format!(
                    "Could not create directory {}",
                    simics_home.display()
                ));
            }

            if !simics_ispm_dir.exists() {
                download_and_unpack_tgz(SIMICS_ISPM_URL, &simics_ispm_dir)?;
            }

            if !simics_bundle_file.exists() {
                download_file(SIMICS_PKGS_URL, &simics_bundle_file)?;
            }

            simics_home
        }
    };

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    setup_simics()?;
    Ok(())
}
