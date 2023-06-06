use crate::package::{package_infos, PackageInfo};
use anyhow::{Context, Result};
use std::path::Path;
use version_tools::VersionConstraint;
use versions::Versioning;

/// Parse all SIMICS manifest(s) in the installation to determine the latest simics version and
/// return its manifest
pub fn simics_base_latest<P: AsRef<Path>>(simics_home: P) -> Result<PackageInfo> {
    let infos = package_infos(simics_home)?[&1000].clone();

    let max_base = infos
        .into_iter()
        .max_by_key(|k| Versioning::new(&k.0).expect("Invalid version string"))
        .context("No versions for base")?;

    Ok(max_base.1)
}

/// Find the latest version of the Simics Base package with a particular constraint.
pub fn simics_base_version<P: AsRef<Path>, S: AsRef<str>>(
    simics_home: P,
    base_version_constraint: S,
) -> Result<PackageInfo> {
    let constraint: VersionConstraint = base_version_constraint.as_ref().parse()?;
    let infos = package_infos(simics_home)?[&1000].clone();
    let version = infos
        .keys()
        .filter_map(|k| Versioning::new(k))
        .filter(|v| constraint.matches(v))
        .max()
        .context("No matching version")?;

    Ok(infos
        .get(&version.to_string())
        .context(format!("No such version {}", version))?
        .clone())
}
