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

pub type PackageVersion = String;
pub type PackageNumber = i64;

#[derive(Hash, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, FromPrimitive, ToPrimitive)]
#[repr(i64)]
/// Numbers for public SIMICS packages. These numbers can be used to conveniently specify package
/// numbers
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Information about a package. This package info is located in the packageinfo subdirectory of
/// a simics package, for example SIMICS_HOME/simics-6.0.157/packageinfo/Simics-Base-linux64
/// and is not *quite* YAML but is close.
pub struct PackageInfo {
    /// The package name
    pub name: String,
    /// The package description
    pub description: String,
    /// The version string for the package
    pub version: String,
    #[serde(rename = "extra-version")]
    /// The extra version string for the package, usually blank
    pub extra_version: String,
    //// Host type, e.g. `linux64`
    pub host: String,
    /// Whether the package is public or private
    pub confidentiality: String,
    #[serde(rename = "package-name")]
    /// The name of the package, again (this field is typically the same as `name`)
    pub package_name: String,
    #[serde(rename = "package-number")]
    /// The package number
    pub package_number: PackageNumber,
    #[serde(rename = "build-id")]
    /// A monotonically increasing build ID for the package number
    pub build_id: u64,
    #[serde(rename = "build-id-namespace")]
    /// Namespace for build IDs, `simics` for public/official packages
    pub build_id_namespace: String,
    #[serde(rename = "type")]
    /// The type of package, typically either `base` or `addon`
    pub typ: String,
    #[serde(rename = "package-name-full")]
    /// Long package name
    pub package_name_full: String,
    /// Complete list of files in the package
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
    /// Get the path to a package relative to the `simics_home` installation directory
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

/// Get all the package information of all packages in the `simics_home` installation directory as
/// a mapping between the package number and a nested mapping of package version to the package
/// info for the package
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
