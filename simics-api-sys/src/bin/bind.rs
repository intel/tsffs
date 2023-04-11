//! Bindings build script for SIMICS sys API
//!
//! This script will download every version of the SIMICS 6 Base package and generate an API
//! bindings file for each one. It will output the bindings to the `src/bindings` directory of
//! this crate, where they will be included by feature flag with the `src/bindings/mod.rs` file.

use anyhow::Result;
use bytes::Buf;
use reqwest::blocking::get;
use std::{
    fs::File,
    io::{copy, BufReader},
    str::Split,
};
use tempfile::Builder;

/// Path to ISPM (this is kept up to date)
const ISPM_URL: &str = "https://af02p-or.devtools.intel.com/artifactory/simics-local/pub/simics-installer/external/intel-simics-package-manager-cli-latest-linux64.tar.gz";

/// Base path to where the SIMICS packages can be downloaded from
const SIMICS_PACKAGE_REPO_URL: &str =
    "https://af02p-or.devtools.intel.com/ui/native/simics-local/pub/simics-6/linux64";

/// Known versions for SIMICS-Base package. This should be updated correspondingly with a feature
/// flag addition in the library for this crate
const SIMICS_BASE_VERSIONS: &[&str] = &[
    "6.0.14", "6.0.15", "6.0.20", "6.0.21", "6.0.28", "6.0.31", "6.0.33", "6.0.34", "6.0.35",
    "6.0.36", "6.0.38", "6.0.39", "6.0.40", "6.0.41", "6.0.42", "6.0.43", "6.0.44", "6.0.45",
    "6.0.46", "6.0.47", "6.0.48", "6.0.49", "6.0.50", "6.0.51", "6.0.52", "6.0.53", "6.0.54",
    "6.0.55", "6.0.56", "6.0.57", "6.0.58", "6.0.59", "6.0.60", "6.0.61", "6.0.62", "6.0.63",
    "6.0.64", "6.0.65", "6.0.66", "6.0.67", "6.0.68", "6.0.69", "6.0.70", "6.0.71", "6.0.72",
    "6.0.73", "6.0.74", "6.0.75", "6.0.76", "6.0.77", "6.0.78", "6.0.79", "6.0.80", "6.0.81",
    "6.0.82", "6.0.83", "6.0.84", "6.0.85", "6.0.86", "6.0.87", "6.0.88", "6.0.89", "6.0.90",
    "6.0.91", "6.0.92", "6.0.93", "6.0.94", "6.0.95", "6.0.96", "6.0.97", "6.0.98", "6.0.99",
    "6.0.100", "6.0.101", "6.0.102", "6.0.103", "6.0.104", "6.0.105", "6.0.106", "6.0.107",
    "6.0.108", "6.0.109", "6.0.110", "6.0.111", "6.0.112", "6.0.113", "6.0.114", "6.0.115",
    "6.0.116", "6.0.117", "6.0.118", "6.0.119", "6.0.120", "6.0.121", "6.0.122", "6.0.123",
    "6.0.124", "6.0.125", "6.0.126", "6.0.127", "6.0.128", "6.0.129", "6.0.130", "6.0.131",
    "6.0.132", "6.0.133", "6.0.134", "6.0.135", "6.0.136", "6.0.137", "6.0.138", "6.0.139",
    "6.0.140", "6.0.141", "6.0.142", "6.0.143", "6.0.144", "6.0.145", "6.0.146", "6.0.147",
    "6.0.148", "6.0.149", "6.0.150", "6.0.151", "6.0.152", "6.0.153", "6.0.154", "6.0.155",
    "6.0.156", "6.0.157", "6.0.158", "6.0.159", "6.0.160", "6.0.161", "6.0.162",
];

fn main() -> Result<()> {
    let download_dir = Builder::new().prefix("simics-packages").tempdir()?;
    let download_dir = download_dir.into_path();

    for version in SIMICS_BASE_VERSIONS {
        let package_url = format!(
            "{}/simics-pkg-1000-{}-linux64.tar",
            SIMICS_PACKAGE_REPO_URL, version
        );
        println!("Downloading {}", package_url);
        let response = get(package_url)?;
        let fname = response
            .url()
            .path_segments()
            .and_then(Split::last)
            .expect("No file component of downloaded response");
        let fpath = download_dir.join(fname);

        let mut dest = File::create(&fpath)?;
        copy(&mut response.bytes()?.reader(), &mut dest)?;

        println!("Wrote downloaded file to {}", fpath.display());
    }

    Ok(())
}
