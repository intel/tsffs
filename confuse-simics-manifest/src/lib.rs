//! This library implements utilities for reading and manipulating SIMICS Manifests. SIMICS
//! includes manifest files for each installation in .smf format (AKA YAML) and in a pseudo-yaml
//! format with each package.

use anyhow::{Context, Result};
use itertools::Itertools;
use log::{error, warn};
use num::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};
use versions::Versioning;

extern crate num_traits;
#[macro_use]
extern crate num_derive;

pub type PackageNumber = i64;
pub type PackageVersion = String;

#[derive(Hash, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, FromPrimitive, ToPrimitive)]
#[repr(i64)]
pub enum PublicPackageNumber {
    QspClearLinux = 4094,
    QspCpu = 8112,
    QspIsim = 8144,
    DoceaBase = 7801,
    OssSources = 1020,
    Training = 6010,
    Viewer = 8126,
    QspX86 = 2096,
    Base = 1000,
    Error = -1,
}

impl From<i64> for PublicPackageNumber {
    fn from(value: i64) -> Self {
        FromPrimitive::from_i64(value).unwrap_or(PublicPackageNumber::Error)
    }
}

impl From<PublicPackageNumber> for i64 {
    fn from(val: PublicPackageNumber) -> Self {
        ToPrimitive::to_i64(&val).expect("Invalid conversion to i64")
    }
}

/// Parse all SIMICS manifest(s) in the installation to determine the latest simics version and
/// return its manifest
pub fn simics_latest<P: AsRef<Path>>(simics_home: P) -> Result<PackageInfo> {
    let infos = package_infos(simics_home)?[&1000].clone();

    let max_base = infos
        .into_iter()
        .max_by_key(|k| Versioning::new(&k.0).expect("Invalid version string"))
        .context("No versions for base")?;

    Ok(max_base.1)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Information about a package. This package info is located in the packageinfo subdirectory of
/// a simics package, for example SIMICS_HOME/simics-6.0.157/packageinfo/Simics-Base-linux64
/// and is not *quite* YAML but is close.
pub struct PackageInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "extra-version")]
    pub extra_version: String,
    pub host: String,
    pub confidentiality: String,
    #[serde(rename = "package-name")]
    pub package_name: String,
    #[serde(rename = "package-number")]
    pub package_number: PackageNumber,
    #[serde(rename = "build-id")]
    pub build_id: u64,
    #[serde(rename = "build-id-namespace")]
    pub build_id_namespace: String,
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(rename = "package-name-full")]
    pub package_name_full: String,
    pub files: Vec<String>,
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
            package_number: -1,
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
) -> Result<HashMap<PackageNumber, HashMap<PackageVersion, PackageInfo>>> {
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
                                    package_info.package_number =
                                        v.to_string().parse().unwrap_or(0).try_into().unwrap_or(-1)
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
        .group_by(|p| p.package_number)
        .into_iter()
        .map(|(k, g)| {
            let g: Vec<_> = g.collect();
            (
                k,
                g.iter()
                    .map(|p| (p.version.clone(), (*p).clone()))
                    .collect(),
            )
        })
        .collect())
}
