//! Runs the SIMICS tests for the project

use anyhow::Result;
use ispm_wrapper::{
    data::ProjectPackage,
    ispm::{
        self,
        packages::{InstallOptions, UninstallOptions},
        projects::CreateOptions,
        GlobalOptions,
    },
};
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

pub fn test_simics_home<P>(tmpdir: P) -> Result<PathBuf>
where
    P: AsRef<Path>,
{
    let simics_tmpdir = tmpdir.as_ref().join("simics");

    if !simics_tmpdir.exists() {
        create_dir_all(&simics_tmpdir)?;
    }

    ispm::packages::uninstall(
        &UninstallOptions::builder()
            .packages([ProjectPackage::builder()
                .package_number(31337)
                .version("latest")
                .build()])
            .global(GlobalOptions::builder().install_dir(&simics_tmpdir).build())
            .build(),
    )
    .map_err(|e| eprintln!("Not uninstalling package: {}", e))
    .ok();

    ispm::packages::install(
        &InstallOptions::builder()
            .package_paths([PathBuf::from(CARGO_MANIFEST_DIR)
                .join("../../../")
                .join("linux64")
                .join("packages")
                .join("simics-pkg-31337-6.0.0-linux64.ispm")])
            .global(
                GlobalOptions::builder()
                    .install_dir(&simics_tmpdir)
                    .trust_insecure_packages(true)
                    .build(),
            )
            .build(),
    )?;

    Ok(simics_tmpdir)
}

pub fn test_project_x86<S>(tmpdir: S, name: S) -> Result<PathBuf>
where
    S: AsRef<str>,
{
    let test_dir = PathBuf::from(tmpdir.as_ref()).join(name.as_ref());

    let test_project = test_dir.join("project");
    let test_simics_home = test_simics_home(&test_dir)?;

    if test_project.is_dir() {
        remove_dir_all(&test_project)?;
    }

    ispm::projects::create(
        &CreateOptions::builder()
            .packages([
                ProjectPackage::builder()
                    .package_number(1000)
                    .version("latest")
                    .build(),
                ProjectPackage::builder()
                    .package_number(2096)
                    .version("latest")
                    .build(),
                ProjectPackage::builder()
                    .package_number(8112)
                    .version("latest")
                    .build(),
                ProjectPackage::builder()
                    .package_number(31337)
                    .version("latest")
                    .build(),
            ])
            .global(
                GlobalOptions::builder()
                    .install_dir(test_simics_home)
                    .build(),
            )
            .build(),
        &test_project,
    )?;

    Ok(test_project)
}
