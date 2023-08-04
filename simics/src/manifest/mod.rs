//! Parse package manifest files and extract package specifications containing information about
//! files in the package, package version, etc.

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use version_tools::VersionConstraint;
use versions::Versioning;

use crate::package::{packages, Package, PackageNumber};

/// Parse all SIMICS manifest(s) in the installation to determine the latest simics version and
/// return its manifest
pub fn package_latest<P>(simics_home: P, package_number: PackageNumber) -> Result<Package>
where
    P: AsRef<Path>,
{
    let infos = packages(simics_home)?[&package_number].clone();

    let max_base = infos
        .into_iter()
        .max_by_key(|k| Versioning::new(&k.0).expect("Invalid version string"))
        .context("No versions for base")?;

    Ok(max_base.1)
}

/// Find the latest version of the Simics Base package with a particular constraint.
pub fn package_version<P>(
    simics_home: P,
    package_number: PackageNumber,
    version_constraint: VersionConstraint,
) -> Result<Package>
where
    P: AsRef<Path>,
{
    let infos = packages(simics_home.as_ref())?[&package_number].clone();
    let version = infos
        .keys()
        .filter_map(|k| Versioning::new(k))
        .filter(|v| version_constraint.matches(v))
        .max()
        .ok_or_else(|| {
            anyhow!(
                "No simics base package number {} matching version {:?} in {}. Got package infos {:?}",
                package_number,
                version_constraint,
                simics_home.as_ref().display(),
                infos
            )
        })?;

    Ok(infos
        .get(&version.to_string())
        .ok_or_else(|| anyhow!("No such version {} in {:?}", version, infos))?
        .clone())
}
