use anyhow::{bail, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_yaml::from_reader;
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::Path,
};

#[derive(
    Hash, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize_repr, Serialize_repr, Debug,
)]
#[repr(u64)]
/// Package Identifiers for Simics Manifest
///
/// If you have a package not listed here, you need to add it via PR
pub enum PackageNumber {
    Base = 1000,
    OSSSources = 1020,
    QuickStartPlatform = 2096,
    QuickStartPlatformClearLinux = 4094,
    SimicsTraining = 6010,
    SimicsDoceaBase = 7801,
    QuickStartPlatformCpu = 8112,
    SimicsViewer = 8126,
    QuickStartPlatformISim = 8144,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// The profile for a SIMICS manifest
pub struct SimicsManifestProfile {
    pub description: String,
    pub name: String,
    pub platform_script: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// A package description in a SIMICS manifest
pub struct SimicsManifestPackage {
    pub description: Option<String>,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimicsManifest {
    pub manifest_format: u64,
    pub name: String,
    pub group: String,
    pub version: String,
    pub date: NaiveDate,
    pub description: String,
    pub profiles: HashMap<String, SimicsManifestProfile>,
    pub packages: HashMap<PackageNumber, SimicsManifestPackage>,
}

pub fn parse_simics_manifest(manifest: &Path) -> Result<SimicsManifest> {
    let manifest_content = read_to_string(manifest)?;
    let manifest: SimicsManifest = from_reader(manifest_content.as_bytes())?;
    Ok(manifest)
}

/// Parse SIMICS manifest(s) to determine the latest simics version and return its manifest
pub fn simics_latest<P: AsRef<Path>>(simics_home: P) -> Result<SimicsManifest> {
    let manifest_dir = simics_home.as_ref().join("manifests");
    let mut manifests = read_dir(&manifest_dir)?
        .filter_map(|de| match de.ok() {
            Some(de) => parse_simics_manifest(&de.path()).ok(),
            None => None,
        })
        .collect::<Vec<_>>();

    manifests.sort_by(|a, b| a.date.cmp(&b.date));

    match manifests.last() {
        Some(m) => Ok(m.clone()),
        None => bail!("No highest version manifest found."),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageInfo {
    name: String,
    description: String,
    version: String,
    #[serde(rename = "extra-version")]
    extra_version: String,
    host: String,
    confidentiality: String,
    #[serde(rename = "package-name")]
    package_name: String,
    #[serde(rename = "package-number")]
    package_number: PackageNumber,
    #[serde(rename = "build-id")]
    build_id: u64,
    #[serde(rename = "build-id-namespace")]
    build_id_namespace: String,
    #[serde(rename = "type")]
    typ: String,
    #[serde(rename = "package-name-full")]
    package_name_full: String,
    files: Vec<String>,
}
