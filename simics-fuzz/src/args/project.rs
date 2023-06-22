use anyhow::{anyhow, bail, Error, Result};
use simics::{
    module::Module,
    package::{Package, PackageBuilder, PackageNumber},
    project::SimicsPath,
};
use std::{path::PathBuf, str::FromStr};
use version_tools::VersionConstraint;

#[derive(Debug, Clone)]
pub struct PackageArg {
    pub package: Package,
}

impl FromStr for PackageArg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = s.split(':').collect::<Vec<_>>();

        match (parts.first(), parts.get(1)) {
            (Some(number), Some(version)) => {
                let package_number: PackageNumber = number.parse()?;
                let version: VersionConstraint = version.parse()?;
                Ok(Self {
                    package: PackageBuilder::default()
                        .package_number(package_number)
                        .version(version)
                        .build()?,
                })
            }
            _ => Err(anyhow!(
                "Couldn't parse package number and version from {}",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleArg {
    pub module: Module,
}

impl FromStr for ModuleArg {
    type Err = Error;

    fn from_str(_s: &str) -> Result<Self> {
        todo!("Modules are not implemented, but the CONFUSE module is automatically loaded");
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryArg {
    pub src: PathBuf,
    pub dst: SimicsPath,
}

impl FromStr for DirectoryArg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = s.split(':').collect::<Vec<_>>();
        match (parts.first(), parts.get(1)) {
            (Some(src), Some(dst)) => Ok(Self {
                src: PathBuf::from(src),
                dst: dst.parse()?,
            }),
            _ => bail!("Directory argument {} not of the form 'src:dst'", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileArg {
    pub src: PathBuf,
    pub dst: SimicsPath,
}

impl FromStr for FileArg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = s.split(':').collect::<Vec<_>>();
        match (parts.first(), parts.get(1)) {
            (Some(src), Some(dst)) => Ok(Self {
                src: PathBuf::from(src),
                dst: dst.parse()?,
            }),
            _ => bail!("File argument {} not of the form 'src:dst'", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathSymlinkArg {
    pub src: PathBuf,
    pub dst: SimicsPath,
}

impl FromStr for PathSymlinkArg {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = s.split(':').collect::<Vec<_>>();
        match (parts.first(), parts.get(1)) {
            (Some(src), Some(dst)) => Ok(Self {
                src: PathBuf::from(src),
                dst: dst.parse()?,
            }),
            _ => bail!("Symlink argument {} not of the form 'src:dst'", s),
        }
    }
}
