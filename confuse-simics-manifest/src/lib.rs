//! This library implements utilities for reading and manipulating SIMICS Manifests. SIMICS
//! includes manifest files for each installation in .smf format (AKA YAML) and in a pseudo-yaml
//! format with each package.

use anyhow::{bail, Context, Error, Result};
use chrono::NaiveDate;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_yaml::from_reader;
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};
extern crate num_traits;
#[macro_use]
extern crate num_derive;

#[derive(
    Hash,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Deserialize_repr,
    Serialize_repr,
    Debug,
    FromPrimitive,
)]
#[repr(u64)]
/// Package Identifiers for Simics Manifest
///
/// If you have or need a package not listed here, just add it!
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
    Error = 0,
}

impl TryFrom<u64> for PackageNumber {
    type Error = Error;
    /// Try to convert a u64 to a PackageNumber and fail if the PackageNumber is unknown
    fn try_from(value: u64) -> Result<Self> {
        num::FromPrimitive::from_u64(value).context("Could not convert to PackageNumber")
    }
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
/// A SIMICS manifest, the top-level manifest for an installation in YAML format
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

/// Parse a simics manifest file into a `SimicsManifest` object
pub fn parse_simics_manifest(manifest: &Path) -> Result<SimicsManifest> {
    let manifest_content = read_to_string(manifest)?;
    let manifest: SimicsManifest = from_reader(manifest_content.as_bytes())?;
    Ok(manifest)
}

/// Parse all SIMICS manifest(s) in the installation to determine the latest simics version and
/// return its manifest
pub fn simics_latest<P: AsRef<Path>>(simics_home: P) -> Result<SimicsManifest> {
    let manifest_dir = simics_home.as_ref().join("manifests");
    let mut manifests = read_dir(manifest_dir)?
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
/// Information about a package. This package info is located in the packageinfo subdirectory of
/// a simics package, for example SIMICS_HOME/simics-6.0.157/packageinfo/Simics-Base-linux64
/// and is not *quite* YAML but is close.
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

impl Default for PackageInfo {
    /// A default, blank, package info structure
    fn default() -> Self {
        Self {
            name: "".to_string(),
            description: "".to_string(),
            version: "".to_string(),
            extra_version: "".to_string(),
            host: "".to_string(),
            confidentiality: "".to_string(),
            package_name: "".to_string(),
            package_number: PackageNumber::Error,
            build_id: 0,
            build_id_namespace: "".to_string(),
            typ: "".to_string(),
            package_name_full: "".to_string(),
            files: vec![],
        }
    }
}

impl PackageInfo {
    /// Get the path to a package relative to the simics home installation directory
    pub fn get_package_path<P: AsRef<Path>>(&self, simics_home: P) -> Result<PathBuf> {
        Ok(simics_home.as_ref().to_path_buf().join(
            self.files
                .iter()
                .take(1)
                .next()
                .context("No files in package.")?
                .split('/')
                .take(1)
                .next()
                .context("No base path.")?,
        ))
    }
}

/// Get all the package information of all packages in the simics home installation directory
pub fn package_infos<P: AsRef<Path>>(
    simics_home: P,
) -> Result<HashMap<PackageNumber, PackageInfo>> {
    let infos: Vec<PackageInfo> = read_dir(&simics_home)?
        .filter_map(|d| {
            d.map_err(|e| error!("Could not read directory entry: {}", e))
                .ok()
        })
        .filter_map(|d| match d.path().join("packageinfo").is_dir() {
            true => Some(d.path().join("packageinfo")),
            false => {
                warn!(
                    "Package info path {:?} is not a directory",
                    d.path().join("packageinfo")
                );
                None
            }
        })
        .filter_map(|pid| match read_dir(&pid) {
            Ok(rd) => rd.into_iter().take(1).next().or_else(|| {
                warn!("No contents of packageinfo directory {:?}", pid);
                None
            }),
            Err(_) => None,
        })
        .filter_map(|pi| {
            pi.map_err(|e| {
                error!("Could not get directory entry: {}", e);
                e
            })
            .ok()
        })
        .filter_map(|pi| {
            read_to_string(pi.path())
                .map_err(|e| {
                    error!("Could not read file {:?} to string: {}", pi.path(), e);
                    e
                })
                .ok()
        })
        .map(|pis| {
            // TODO: This should be worked out with a real parser if possible
            // We're parsing it bespoke because...it's not yaml! yay
            let mut package_info = PackageInfo::default();
            pis.lines().for_each(|l| {
                if l.trim_start() != l {
                    // There is some whitespace at the front
                    package_info.files.push(l.trim().to_string());
                } else {
                    let kv: Vec<&str> = l.split(':').map(|lp| lp.trim()).collect();
                    if let Some(k) = kv.first() {
                        if let Some(v) = kv.get(1) {
                            match k.to_string().as_str() {
                                "name" => package_info.name = v.to_string(),
                                "description" => package_info.description = v.to_string(),
                                "version" => package_info.version = v.to_string(),
                                "extra-version" => package_info.extra_version = v.to_string(),
                                "host" => package_info.host = v.to_string(),
                                "confidentiality" => package_info.confidentiality = v.to_string(),
                                "package-name" => package_info.package_name = v.to_string(),
                                "package-number" => {
                                    package_info.package_number = v
                                        .to_string()
                                        .parse()
                                        .unwrap_or(0)
                                        .try_into()
                                        .unwrap_or(PackageNumber::Error)
                                }
                                "build-id" => {
                                    package_info.build_id = v.to_string().parse().unwrap_or(0)
                                }
                                "build-id-namespace" => {
                                    package_info.build_id_namespace = v.to_string()
                                }
                                "type" => package_info.typ = v.to_string(),
                                "package-name-full" => {
                                    package_info.package_name_full = v.to_string()
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
            package_info
        })
        .collect();

    Ok(infos
        .iter()
        .map(|pi| (pi.package_number, pi.clone()))
        .collect())
}
