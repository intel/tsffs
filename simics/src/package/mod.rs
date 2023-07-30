//! Utilities for managing SIMICS packages, reading and writing their metadata, and creating
//! packages from a specification

use crate::simics::home::simics_home;
use anyhow::{anyhow, bail, Error, Result};
use derive_builder::Builder;
use itertools::Itertools;
use num::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::{error, warn};
use version_tools::VersionConstraint;
use versions::Versioning;

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

pub fn parse_packageinfo<P: AsRef<Path>>(package_path: P) -> Result<Package> {
    let package_path = package_path.as_ref().to_path_buf();

    if !package_path.is_dir() {
        bail!(
            "Package path {} does not exist or is not a directory",
            package_path.display()
        );
    }

    let packageinfo_path = package_path.join("packageinfo");

    if !packageinfo_path.is_dir() {
        bail!(
            "Package info path {} does not exist or is not a directory",
            packageinfo_path.display()
        );
    }

    let package_home = package_path
        .parent()
        .ok_or_else(|| anyhow!("No parent of package path {}", package_path.display()))?
        .to_path_buf();

    read_dir(&packageinfo_path)
        .map_err(|e| {
            anyhow!(
                "Failed to read packageinfo directory {}: {}",
                packageinfo_path.display(),
                e
            )
        })
        .and_then(|packageinfo_entries| {
            packageinfo_entries
                .into_iter()
                .take(1)
                .next()
                .ok_or_else(|| {
                    anyhow!(
                        "No entries in packageinfo directory {}",
                        packageinfo_path.display()
                    )
                })
                .map(|packageinfo_manifest| {
                    packageinfo_manifest
                        .map(|packageinfo_manifest| packageinfo_manifest.path())
                        .map_err(|e| anyhow!("Couldn't get entry for manifest: {}", e))
                })
        })?
        .and_then(|packageinfo_manifest| {
            read_to_string(&packageinfo_manifest)
                .map_err(|e| {
                    anyhow!(
                        "Failed to read manifest {}: {}",
                        packageinfo_manifest.display(),
                        e
                    )
                })
                .map(|packageinfo_contents| {
                    packageinfo_contents.parse().map(|mut package: Package| {
                        package.home = package_home.clone();
                        package.path = package_path.clone();
                        package
                    })
                })
        })?
}

/// Get all the package information of all packages in the `simics_home` installation directory as
/// a mapping between the package number and a nested mapping of package version to the package
/// info for the package
pub fn packages<P: AsRef<Path>>(
    home: P,
) -> Result<HashMap<PackageNumber, HashMap<PackageVersion, Package>>> {
    let infos: Vec<Package> = read_dir(&home)?
        .filter_map(|home_dir_entry| {
            home_dir_entry
                .map_err(|e| error!("Could not read directory entry: {}", e))
                .ok()
        })
        .filter_map(|home_dir_entry| {
            let package_path = home_dir_entry.path();
            parse_packageinfo(&package_path)
                .map_err(|e| {
                    warn!(
                        "Could not parse package info from package at {}: {}",
                        package_path.display(),
                        e
                    )
                })
                .ok()
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

#[derive(Builder, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[builder(setter(skip), build_fn(skip))]
pub struct Package {
    #[serde(skip)]
    #[builder(setter(into))]
    /// The SIMICS Home directory. You should never need to manually specify this.
    pub home: PathBuf,
    #[serde(skip)]
    #[builder(setter(into, name = "version"))]
    /// The version string for the package
    pub version_constraint: VersionConstraint,
    #[serde(skip)]
    pub path: PathBuf,
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
    #[builder(setter(into))]
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

impl TryFrom<PathBuf> for Package {
    type Error = Error;
    fn try_from(value: PathBuf) -> Result<Self> {
        let mut package = parse_packageinfo(&value)?;
        package.home = value
            .parent()
            .ok_or_else(|| anyhow!("No parent directory for package path {}", value.display()))?
            .to_path_buf();
        package.path = value;
        Ok(package)
    }
}

impl Debug for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Package")
            .field("home", &self.home)
            .field("version_constraint", &self.version_constraint)
            .field("path", &self.path)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("version", &self.version)
            .field("extra_version", &self.extra_version)
            .field("host", &self.host)
            .field("confidentiality", &self.confidentiality)
            .field("package_name", &self.package_name)
            .field("package_number", &self.package_number)
            .field("build_id", &self.build_id)
            .field("build_id_namespace", &self.build_id_namespace)
            .field("typ", &self.typ)
            .field("package_name_full", &self.package_name_full)
            .field("files", &"[...]")
            .finish()
    }
}

impl PackageBuilder {
    pub fn build(&mut self) -> Result<Package> {
        let home = self.home.as_ref().cloned().unwrap_or(simics_home()?);

        let package_number = self
            .package_number
            .ok_or_else(|| anyhow!("No package number set"))?;

        let packages = packages(&home)?;
        let packages_for_number = packages.get(&package_number).ok_or_else(|| {
            anyhow!(
                "No package found with number {} in {}",
                package_number,
                home.display()
            )
        })?;

        let package_version = self
            .version_constraint
            .as_ref()
            .cloned()
            .unwrap_or("*".parse()?);

        let version = packages_for_number
            .keys()
            .filter_map(|k| Versioning::new(k))
            .filter(|v| package_version.matches(v))
            .max()
            .ok_or_else(|| anyhow!("No version found"))?;

        packages_for_number
            .get(&version.to_string())
            .ok_or_else(|| {
                anyhow!(
                    "No version {} found for package {} in {}",
                    version,
                    package_number,
                    home.display()
                )
            })
            .cloned()
    }
}

impl Package {
    /// A default, blank, package info structure
    fn try_default() -> Result<Self> {
        Ok(Self::blank_in_at(simics_home()?, PathBuf::from("")))
    }

    fn blank_in_at(home: PathBuf, path: PathBuf) -> Self {
        Self {
            home,
            path,
            version_constraint: VersionConstraint::default(),
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

impl FromStr for Package {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut package = Package::try_default()?;

        s.lines().for_each(|l| {
            if l.trim_start() != l {
                // There is some whitespace at the front
                package.files.push(l.trim().to_string());
            } else {
                let kv: Vec<&str> = l.split(':').map(|lp| lp.trim()).collect();
                if let Some(k) = kv.first() {
                    if let Some(v) = kv.get(1) {
                        match k.to_string().as_str() {
                            "name" => package.name = v.to_string(),
                            "description" => package.description = v.to_string(),
                            "version" => package.version = v.to_string(),
                            "extra-version" => package.extra_version = v.to_string(),
                            "host" => package.host = v.to_string(),
                            "confidentiality" => package.confidentiality = v.to_string(),
                            "package-name" => package.package_name = v.to_string(),
                            "package-number" => {
                                package.package_number =
                                    v.to_string().parse().unwrap_or(0).try_into().unwrap_or(-1)
                            }
                            "build-id" => package.build_id = v.to_string().parse().unwrap_or(0),
                            "build-id-namespace" => package.build_id_namespace = v.to_string(),
                            "type" => package.typ = v.to_string(),
                            "package-name-full" => package.package_name_full = v.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(package)
    }
}
