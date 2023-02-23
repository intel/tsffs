use anyhow::{bail, Context, Result};
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
    Error = 0,
}

impl From<u64> for PackageNumber {
    fn from(value: u64) -> Self {
        match value {
            x if x == PackageNumber::Base as u64 => PackageNumber::Base,
            x if x == PackageNumber::OSSSources as u64 => PackageNumber::OSSSources,
            x if x == PackageNumber::QuickStartPlatform as u64 => PackageNumber::QuickStartPlatform,
            x if x == PackageNumber::QuickStartPlatformClearLinux as u64 => {
                PackageNumber::QuickStartPlatformClearLinux
            }

            x if x == PackageNumber::SimicsTraining as u64 => PackageNumber::SimicsTraining,
            x if x == PackageNumber::SimicsDoceaBase as u64 => PackageNumber::SimicsDoceaBase,
            x if x == PackageNumber::QuickStartPlatformCpu as u64 => {
                PackageNumber::QuickStartPlatformCpu
            }
            x if x == PackageNumber::SimicsViewer as u64 => PackageNumber::SimicsViewer,
            x if x == PackageNumber::QuickStartPlatformISim as u64 => {
                PackageNumber::QuickStartPlatformISim
            }
            _ => PackageNumber::Error,
        }
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

impl Default for PackageInfo {
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
    pub fn get_package_path<P: AsRef<Path>>(&self, simics_home: P) -> Result<PathBuf> {
        Ok(simics_home.as_ref().to_path_buf().join(
            self.files
                .iter()
                .take(1)
                .next()
                .context("No files in package.")?
                .split("/")
                .take(1)
                .next()
                .context("No base path.")?,
        ))
    }
}

pub fn package_infos<P: AsRef<Path>>(
    simics_home: P,
) -> Result<HashMap<PackageNumber, PackageInfo>> {
    let infos: Vec<PackageInfo> = read_dir(&simics_home)?
        .into_iter()
        .filter_map(|d| {
            d.map_err(|e| error!("Could not read directory entry: {}", e))
                .ok()
        })
        .filter_map(|d| match d.path().join("packageinfo").is_dir() {
            true => Some(d.path().to_path_buf().join("packageinfo")),
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
                    let kv: Vec<&str> = l.split(":").map(|lp| lp.trim()).collect();
                    match kv.get(0) {
                        Some(k) => match kv.get(1) {
                            Some(v) => match k.to_string().as_str() {
                                "name" => package_info.name = v.to_string(),
                                "description" => package_info.description = v.to_string(),
                                "version" => package_info.version = v.to_string(),
                                "extra-version" => package_info.extra_version = v.to_string(),
                                "host" => package_info.host = v.to_string(),
                                "confidentiality" => package_info.confidentiality = v.to_string(),
                                "package-name" => package_info.package_name = v.to_string(),
                                "package-number" => {
                                    package_info.package_number =
                                        v.to_string().parse().unwrap_or_else(|_| 0).into()
                                }
                                "build-id" => {
                                    package_info.build_id =
                                        v.to_string().parse().unwrap_or_else(|_| 0)
                                }
                                "build-id-namespace" => {
                                    package_info.build_id_namespace = v.to_string()
                                }
                                "type" => package_info.typ = v.to_string(),
                                "package-name-full" => {
                                    package_info.package_name_full = v.to_string()
                                }
                                _ => {}
                            },
                            None => {}
                        },
                        None => {}
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
