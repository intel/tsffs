// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS test utilities for test environment setup and configuration

use anyhow::{anyhow, bail, ensure, Result};
use getters::Getters;
use ispm_wrapper::{
    data::ProjectPackage,
    ispm::{
        self,
        packages::{InstallOptions, UninstallOptions},
        projects::CreateOptions,
        GlobalOptions,
    },
    Internal,
};
use std::{
    collections::HashSet,
    env::var,
    fs::{copy, create_dir_all, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
};
use typed_builder::TypedBuilder;
use walkdir::WalkDir;

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
pub fn copy_dir_contents<P>(src_dir: P, dst_dir: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let src_dir = src_dir.as_ref().to_path_buf();
    ensure!(src_dir.is_dir(), "Source must be a directory");
    let dst_dir = dst_dir.as_ref().to_path_buf();
    if !dst_dir.is_dir() {
        create_dir_all(&dst_dir)?;
    }

    for (src, dst) in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let src = p.path().to_path_buf();
            match src.strip_prefix(&src_dir) {
                Ok(suffix) => Some((src.clone(), dst_dir.join(suffix))),
                Err(_) => None,
            }
        })
    {
        if src.is_dir() {
            create_dir_all(&dst)?;
        } else if src.is_file() {
            if let Err(e) = copy(&src, &dst) {
                eprintln!(
                    "Warning: failed to copy file from {} to {}: {}",
                    src.display(),
                    dst.display(),
                    e
                );
            }
        }
    }
    Ok(())
}

/// Abstract install procedure for public and internal ISPM
pub fn local_or_remote_pkg_install(mut options: InstallOptions) -> Result<()> {
    if Internal::is_internal()? {
        ispm::packages::install(&options)?;
    } else {
        let installed = ispm::packages::list(&GlobalOptions::default())?;

        for package in options.packages() {
            let Some(installed) = installed.installed_packages() else {
                bail!("Did not get any installed packages");
            };
            let Some(available) = installed.iter().find(|p| {
                p.package_number() == package.package_number() && p.version() == package.version()
            }) else {
                bail!("Did not find package {package:?} in {installed:?}");
            };
            let Some(path) = available.paths().first() else {
                bail!("No paths for available package {available:?}");
            };
            let Some(install_dir) = options.global().install_dir() else {
                bail!("No install dir for global options {options:?}");
            };

            let package_install_dir = install_dir
                .components()
                .last()
                .ok_or_else(|| {
                    anyhow!(
                        "No final component in install dir {}",
                        install_dir.display()
                    )
                })?
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert component to string"))?
                .to_string();

            create_dir_all(&install_dir.join(&package_install_dir))?;
            copy_dir_contents(path, &install_dir.join(&package_install_dir))?;
        }

        // Clear the remote packages to install, we can install local paths no problem
        options.packages_mut().clear();

        if !options.package_paths().is_empty() {
            ispm::packages::install(&options)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum Architecture {
    X86,
    Riscv,
}

impl Architecture {
    fn packages(&self) -> Vec<ProjectPackage> {
        match self {
            Architecture::X86 => vec![
                ProjectPackage::builder()
                    .package_number(1000)
                    .version("6.0.169")
                    .build(),
                // QSP-x86
                ProjectPackage::builder()
                    .package_number(2096)
                    .version("6.0.70")
                    .build(),
                // QSP-CPU
                ProjectPackage::builder()
                    .package_number(8112)
                    .version("6.0.17")
                    .build(),
            ],
            Architecture::Riscv => vec![
                ProjectPackage::builder()
                    .package_number(1000)
                    .version("6.0.169")
                    .build(),
                // RISC-V-CPU
                ProjectPackage::builder()
                    .package_number(2050)
                    .version("6.0.57")
                    .build(),
                // RISC-V-Simple
                ProjectPackage::builder()
                    .package_number(2053)
                    .version("6.0.4")
                    .build(),
            ],
        }
    }
}

#[derive(TypedBuilder, Debug)]
pub struct TestEnvSpec {
    #[builder(setter(into))]
    cargo_manifest_dir: String,
    #[builder(setter(into))]
    cargo_target_tmpdir: String,
    #[builder(setter(into))]
    name: String,

    #[builder(default, setter(strip_option, into))]
    arch: Option<Architecture>,
    #[builder(default, setter(into))]
    extra_packages: Vec<ProjectPackage>,
    #[builder(default, setter(into))]
    extra_nonrepo_packages: Vec<ProjectPackage>,
    #[builder(default = true)]
    tsffs: bool,
    #[builder(default, setter(into))]
    files: Vec<(String, Vec<u8>)>,
    #[builder(default, setter(into))]
    directories: Vec<PathBuf>,
    #[builder(default, setter(into, strip_option))]
    simics_home: Option<PathBuf>,
    #[builder(default, setter(into, strip_option))]
    package_repo: Option<String>,
    #[builder(default = false)]
    install_all: bool,
}

impl TestEnvSpec {
    pub fn to_env(&self) -> Result<TestEnv> {
        TestEnv::build(self)
    }
}

#[derive(Getters)]
pub struct TestEnv {
    /// The base of the test environment, e.g. the `CARGO_TARGET_TMPDIR` directory
    test_base: PathBuf,
    /// The subdirectory in the test environment for this test
    test_dir: PathBuf,
    /// The project subdirectory in the test environment for this test
    project_dir: PathBuf,
    /// The simics home subdirectory in the test environment for this test
    simics_home_dir: PathBuf,
}

impl TestEnv {
    pub fn simics_base_dir(&self) -> Result<PathBuf> {
        read_dir(self.simics_home_dir())?
            .filter_map(|d| d.ok())
            .filter(|d| d.path().is_dir())
            .map(|d| d.path())
            .find(|d| {
                d.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("simics-6."))
            })
            .ok_or_else(|| anyhow!("No simics base"))
    }
}

impl TestEnv {
    fn install_tsffs<P, S>(simics_home_dir: P, cargo_manifest_dir: S) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        // NOTE: Uninstall and reinstall the tsffs module (installs the latest build)
        ispm::packages::uninstall(
            &UninstallOptions::builder()
                .packages([ProjectPackage::builder()
                    .package_number(31337)
                    .version("latest")
                    .build()])
                .global(
                    GlobalOptions::builder()
                        .install_dir(simics_home_dir.as_ref())
                        .build(),
                )
                .build(),
        )
        .map_err(|e| eprintln!("Not uninstalling package: {}", e))
        .ok();

        local_or_remote_pkg_install(
            InstallOptions::builder()
                .package_paths([PathBuf::from(cargo_manifest_dir.as_ref())
                    .join("../../../")
                    .join("linux64")
                    .join("packages")
                    .join("simics-pkg-31337-6.0.1-linux64.ispm")])
                .global(
                    GlobalOptions::builder()
                        .install_dir(simics_home_dir.as_ref())
                        .trust_insecure_packages(true)
                        .build(),
                )
                .build(),
        )?;

        Ok(())
    }

    pub fn install_files<P>(project_dir: P, files: &Vec<(String, Vec<u8>)>) -> Result<()>
    where
        P: AsRef<Path>,
    {
        for (name, content) in files {
            let target = project_dir.as_ref().join(name);

            if let Some(target_parent) = target.parent() {
                if target_parent != project_dir.as_ref() {
                    create_dir_all(target_parent)?;
                }
            }
            write(target, content)?;
        }

        Ok(())
    }

    pub fn install_directories<P>(project_dir: P, directories: &Vec<PathBuf>) -> Result<()>
    where
        P: AsRef<Path>,
    {
        for directory in directories {
            copy_dir_contents(directory, &project_dir.as_ref().to_path_buf())?;
        }

        Ok(())
    }

    fn build(spec: &TestEnvSpec) -> Result<Self> {
        let test_base = PathBuf::from(&spec.cargo_target_tmpdir);
        let test_dir = test_base.join(&spec.name);

        let project_dir = test_dir.join("project");

        let simics_home_dir = if let Some(simics_home) = spec.simics_home.as_ref() {
            simics_home.clone()
        } else {
            create_dir_all(test_dir.join("simics"))?;

            test_dir.join("simics")
        };

        // Install nonrepo packages which do not use a possibly-provided package repo
        if !spec.extra_nonrepo_packages.is_empty() {
            println!("installing extra nonrepo packages");
            local_or_remote_pkg_install(
                InstallOptions::builder()
                    .global(
                        GlobalOptions::builder()
                            .install_dir(&simics_home_dir)
                            .trust_insecure_packages(true)
                            .build(),
                    )
                    .packages(spec.extra_nonrepo_packages.clone())
                    .build(),
            )?;
        }

        let mut installed_packages = spec
            .extra_nonrepo_packages
            .iter()
            .cloned()
            .collect::<HashSet<_>>();

        let mut packages = spec.extra_packages.clone();

        if let Some(arch) = spec.arch.as_ref() {
            packages.extend(arch.packages().clone());
        }

        if let Some(package_repo) = &spec.package_repo {
            if !packages.is_empty() {
                println!("Installing extra and arch packages with package repo");
                local_or_remote_pkg_install(
                    InstallOptions::builder()
                        .packages(packages.clone())
                        .global(
                            GlobalOptions::builder()
                                .install_dir(&simics_home_dir)
                                .trust_insecure_packages(true)
                                .package_repo([package_repo.to_string()])
                                .build(),
                        )
                        .build(),
                )?;
            }
        } else if !packages.is_empty() {
            println!("Installing extra and arch packages without package repo");
            local_or_remote_pkg_install(
                InstallOptions::builder()
                    .packages(packages.clone())
                    .global(
                        GlobalOptions::builder()
                            .install_dir(&simics_home_dir)
                            .trust_insecure_packages(true)
                            .build(),
                    )
                    .build(),
            )?;
        }

        installed_packages.extend(packages);

        if spec.install_all {
            if let Some(package_repo) = &spec.package_repo {
                println!("Installing all packages without package repo");
                local_or_remote_pkg_install(
                    InstallOptions::builder()
                        .install_all(spec.install_all)
                        .global(
                            GlobalOptions::builder()
                                .install_dir(&simics_home_dir)
                                .trust_insecure_packages(true)
                                .package_repo([package_repo.to_string()])
                                .build(),
                        )
                        .build(),
                )?;

                let installed = ispm::packages::list(
                    &GlobalOptions::builder()
                        .install_dir(&simics_home_dir)
                        .build(),
                )?;

                if let Some(installed) = installed.installed_packages() {
                    installed_packages.extend(installed.iter().map(|ip| {
                        ProjectPackage::builder()
                            .package_number(*ip.package_number())
                            .version(ip.version().clone())
                            .build()
                    }));
                }
            }
        }

        // Install TSFFS separately from local package
        if spec.tsffs {
            Self::install_tsffs(&simics_home_dir, &spec.cargo_manifest_dir)?;

            installed_packages.insert(
                ProjectPackage::builder()
                    .package_number(31337)
                    .version("latest")
                    .build(),
            );
        }

        // Create the project using the installed packages
        ispm::projects::create(
            &CreateOptions::builder()
                .packages(installed_packages)
                .global(
                    GlobalOptions::builder()
                        .install_dir(&simics_home_dir)
                        .trust_insecure_packages(true)
                        .build(),
                )
                .ignore_existing_files(true)
                .build(),
            &project_dir,
        )
        .ok();

        Self::install_files(&project_dir, &spec.files)?;
        Self::install_directories(&project_dir, &spec.directories)?;

        Ok(Self {
            test_base,
            test_dir,
            project_dir,
            simics_home_dir,
        })
    }

    pub fn cleanup(&mut self) -> Result<()> {
        remove_dir_all(self.test_dir()).map_err(|e| anyhow!("Error cleaning up: {e}"))
    }

    pub fn cleanup_if_env(&mut self) -> Result<()> {
        if let Ok(_cleanup) = var("TSFFS_TEST_CLEANUP_EACH") {
            self.cleanup()?;
        }

        Ok(())
    }
}
