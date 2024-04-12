// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]

//! SIMICS test utilities for test environment setup and configuration

use anyhow::{anyhow, bail, ensure, Error, Result};
use cargo_simics_build::{App, Cmd, SimicsBuildCmd};
use cargo_subcommand::Args;
use command_ext::{CommandExtCheck, CommandExtError};
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
    env::{current_dir, set_current_dir, var},
    fs::{copy, create_dir_all, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use typed_builder::TypedBuilder;
use versions::{Requirement, Versioning};
use walkdir::WalkDir;

/// An environment variable which, if set, causes the entire test workspace to be cleaned up
/// after the test
pub const SIMICS_TEST_CLEANUP_EACH_ENV: &str = "SIMICS_TEST_CLEANUP_EACH";
/// An environment variable which, if set, causes package installation to default to local installation
/// only
pub const SIMICS_TEST_LOCAL_PACKAGES_ONLY_ENV: &str = "SIMICS_TEST_LOCAL_PACKAGES_ONLY";

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
        create_dir_all(&dst_dir).map_err(|e| {
            anyhow!(
                "Failed to create destination directory for directory copy {:?}: {}",
                dst_dir,
                e
            )
        })?;
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
            create_dir_all(&dst).map_err(|e| {
                anyhow!(
                    "Failed to create nested destination directory for copy {:?}: {}",
                    dst,
                    e
                )
            })?;
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
    if Internal::is_internal()? && var(SIMICS_TEST_LOCAL_PACKAGES_ONLY_ENV).is_err() {
        ispm::packages::install(&options)?;
    } else {
        let installed = ispm::packages::list(&GlobalOptions::default())?;

        for package in options.packages.iter() {
            let Some(installed) = installed.installed_packages.as_ref() else {
                bail!("Did not get any installed packages");
            };

            let Some(available) = installed.iter().find(|p| {
                p.package_number == package.package_number
                    && (Requirement::new(&format!("={}", package.version))
                        .or_else(|| {
                            eprintln!("Failed to parse requirement {}", package.version);
                            None
                        })
                        .is_some_and(|r| {
                            Versioning::new(&p.version)
                                .or_else(|| {
                                    eprintln!("Failed to parse version{}", p.version);
                                    None
                                })
                                .is_some_and(|pv| r.matches(&pv))
                        })
                        || package.version == "latest")
            }) else {
                bail!("Did not find package {package:?} in {installed:?}");
            };

            let Some(path) = available.paths.first() else {
                bail!("No paths for available package {available:?}");
            };

            let Some(install_dir) = options.global.install_dir.as_ref() else {
                bail!("No install dir for global options {options:?}");
            };

            let package_install_dir = path
                .components()
                .last()
                .ok_or_else(|| anyhow!("No final component in install dir {}", path.display()))?
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert component to string"))?
                .to_string();

            create_dir_all(&install_dir.join(&package_install_dir)).map_err(|e| {
                anyhow!(
                    "Could not create install dir {:?}: {}",
                    install_dir.join(&package_install_dir),
                    e
                )
            })?;

            copy_dir_contents(&path, &&install_dir.join(&package_install_dir)).map_err(|e| {
                anyhow!(
                    "Error copying installed directory from {:?} to {:?}: {}",
                    path,
                    install_dir.join(&package_install_dir),
                    e
                )
            })?;
        }

        // Clear the remote packages to install, we can install local paths no problem
        options.packages.clear();

        if !options.package_paths.is_empty() {
            ispm::packages::install(&options)?;
        }
    }

    Ok(())
}

#[derive(TypedBuilder, Debug)]
/// A specification for a test environment
pub struct TestEnvSpec {
    #[builder(setter(into))]
    cargo_target_tmpdir: String,
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(into))]
    packages: HashSet<ProjectPackage>,
    #[builder(default, setter(into))]
    nonrepo_packages: HashSet<ProjectPackage>,
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
    #[builder(default, setter(into))]
    package_crates: Vec<PathBuf>,
    #[builder(default, setter(into, strip_option))]
    build_simics_version: Option<String>,
    #[builder(default, setter(into))]
    run_simics_version: Option<String>,
}

impl TestEnvSpec {
    /// Convert the specification for a test environment to a built test environment
    pub fn to_env(&self) -> Result<TestEnv> {
        TestEnv::build(self)
    }
}

/// A test environment, which is a directory that consists of a simics directory with a set of
/// installed packages and a project directory, where test scripts and resources can be placed.
pub struct TestEnv {
    #[allow(unused)]
    /// The base of the test environment, e.g. the `CARGO_TARGET_TMPDIR` directory
    test_base: PathBuf,
    /// The subdirectory in the test environment for this test
    test_dir: PathBuf,
    /// The project subdirectory in the test environment for this test
    project_dir: PathBuf,
    #[allow(unused)]
    /// The simics home subdirectory in the test environment for this test
    simics_home_dir: PathBuf,
}

impl TestEnv {
    /// Return a reference to the test base directory
    pub fn default_simics_base_dir<P>(simics_home_dir: P) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        read_dir(simics_home_dir.as_ref())?
            .filter_map(|d| d.ok())
            .filter(|d| d.path().is_dir())
            .map(|d| d.path())
            .find(|d| {
                d.file_name().is_some_and(|n| {
                    n.to_string_lossy().starts_with("simics-6.")
                        || n.to_string_lossy().starts_with("simics-7.")
                })
            })
            .ok_or_else(|| {
                anyhow!(
                    "No simics base in home directory {:?}",
                    simics_home_dir.as_ref()
                )
            })
    }

    /// Return a reference to the base directory specified by a version
    pub fn simics_base_dir<S, P>(version: S, simics_home_dir: P) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        read_dir(simics_home_dir.as_ref())?
            .filter_map(|d| d.ok())
            .filter(|d| d.path().is_dir())
            .map(|d| d.path())
            .find(|d| {
                d.file_name()
                    .is_some_and(|n| n.to_string_lossy() == format!("simics-{}", version.as_ref()))
            })
            .ok_or_else(|| {
                anyhow!(
                    "No simics base in home directory {:?}",
                    simics_home_dir.as_ref()
                )
            })
    }
}

impl TestEnv {
    /// Install a set of files into a project directory, with the files specified as relative
    /// paths inside the project directory and their raw contents
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

    /// Install a set of existing directories into a project, where each directory will be
    /// copied recursively into the project
    pub fn install_directories<P>(project_dir: P, directories: &Vec<PathBuf>) -> Result<()>
    where
        P: AsRef<Path>,
    {
        for directory in directories {
            copy_dir_contents(directory, &project_dir.as_ref().to_path_buf()).map_err(|e| {
                anyhow!(
                    "Failed to copy directory contents from {:?} to {:?}: {}",
                    directory,
                    project_dir.as_ref(),
                    e
                )
            })?;
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
            create_dir_all(test_dir.join("simics")).map_err(|e| {
                anyhow!(
                    "Could not create simics home directory: {:?}: {}",
                    test_dir.join("simics"),
                    e
                )
            })?;

            test_dir.join("simics")
        };

        // Install nonrepo packages which do not use a possibly-provided package repo
        if !spec.nonrepo_packages.is_empty() {
            local_or_remote_pkg_install(
                InstallOptions::builder()
                    .global(
                        GlobalOptions::builder()
                            .install_dir(&simics_home_dir)
                            .trust_insecure_packages(true)
                            .build(),
                    )
                    .packages(spec.nonrepo_packages.clone())
                    .build(),
            )?;
        }

        let mut installed_packages = spec
            .nonrepo_packages
            .iter()
            .cloned()
            .collect::<HashSet<_>>();

        let packages = spec.packages.clone();

        if let Some(package_repo) = &spec.package_repo {
            if !packages.is_empty() {
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

                if let Some(installed) = installed.installed_packages {
                    installed_packages.extend(
                        installed
                            .iter()
                            .filter(|ip| {
                                if ip.package_number == 1000 {
                                    if let Some(run_version) = spec.run_simics_version.as_ref() {
                                        *run_version == ip.version
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                }
                            })
                            .map(|ip| {
                                ProjectPackage::builder()
                                    .package_number(ip.package_number)
                                    .version(ip.version.clone())
                                    .build()
                            }),
                    );
                }
            }
        }

        let initial_dir = current_dir()?;

        spec.package_crates.iter().try_for_each(|c| {
            // change directory to c
            set_current_dir(c)
                .map_err(|e| anyhow!("Failed to set current directory to {c:?}: {e}"))?;

            #[cfg(debug_assertions)]
            let release = true;
            #[cfg(not(debug_assertions))]
            let release = false;

            let install_args = Args {
                quiet: false,
                manifest_path: Some(c.join("Cargo.toml")),
                package: vec![],
                workspace: false,
                exclude: vec![],
                lib: false,
                bin: vec![],
                bins: false,
                example: vec![],
                examples: false,
                release,
                profile: None,
                features: vec![],
                all_features: false,
                no_default_features: false,
                target: None,
                target_dir: None,
            };

            let cmd = Cmd {
                simics_build: SimicsBuildCmd::SimicsBuild {
                    args: install_args,
                    simics_base: Some(
                        spec.build_simics_version
                            .as_ref()
                            .map(|v| Self::simics_base_dir(&v, &simics_home_dir))
                            .unwrap_or_else(|| Self::default_simics_base_dir(&simics_home_dir))?,
                    ),
                },
            };

            let package = App::run(cmd).map_err(|e| anyhow!("Error running app: {e}"))?;

            let project_package = ProjectPackage::builder()
                .package_number(
                    package
                        .file_name()
                        .ok_or_else(|| anyhow!("No file name"))?
                        .to_str()
                        .ok_or_else(|| anyhow!("Could not convert filename to string"))?
                        .split('-')
                        .nth(2)
                        .ok_or_else(|| anyhow!("No package number"))?
                        .parse::<isize>()?,
                )
                .version(
                    package
                        .file_name()
                        .ok_or_else(|| anyhow!("No file name"))?
                        .to_str()
                        .ok_or_else(|| anyhow!("Could not convert filename to string"))?
                        .split('-')
                        .nth(3)
                        .ok_or_else(|| anyhow!("No version"))?
                        .to_string(),
                )
                .build();

            // Uninstall first, then install. Uninstall is allowed to fail if the output
            // contains 'could not be found to uninstall'
            ispm::packages::uninstall(
                &UninstallOptions::builder()
                    .packages([
                        // Package file names are always 'simics-pkg-<package_number>-<version>-<host>.ispm'
                        project_package.clone(),
                    ])
                    .global(
                        GlobalOptions::builder()
                            .install_dir(&simics_home_dir)
                            .build(),
                    )
                    .build(),
            )
            .or_else(|e| {
                if e.to_string().contains("could not be found to uninstall") {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;

            ispm::packages::install(
                &InstallOptions::builder()
                    .package_paths([package])
                    .global(
                        GlobalOptions::builder()
                            .install_dir(&simics_home_dir)
                            .trust_insecure_packages(true)
                            .build(),
                    )
                    .build(),
            )?;

            installed_packages.insert(project_package);

            Ok::<(), Error>(())
        })?;

        set_current_dir(&initial_dir)
            .map_err(|e| anyhow!("Failed to set current directory to {initial_dir:?}: {e}"))?;

        remove_dir_all(&project_dir).or_else(|e| {
            if e.to_string().contains("No such file or directory") {
                Ok(())
            } else {
                Err(e)
            }
        })?;

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
        )?;

        Self::install_files(&project_dir, &spec.files)?;
        Self::install_directories(&project_dir, &spec.directories)?;

        Ok(Self {
            test_base,
            test_dir,
            project_dir,
            simics_home_dir,
        })
    }

    /// Clean up the test environment
    pub fn cleanup(&mut self) -> Result<(), CommandExtError> {
        remove_dir_all(&self.test_dir).map_err(CommandExtError::from)
    }

    /// Clean up the test environment if the SIMICS_TEST_CLEANUP_EACH environment variable is set
    pub fn cleanup_if_env(&mut self) -> Result<(), CommandExtError> {
        if let Ok(_cleanup) = var(SIMICS_TEST_CLEANUP_EACH_ENV) {
            self.cleanup()?;
        }

        Ok(())
    }

    /// Run a test in the environment in the form of a Simics script. To fail the test, either
    /// exit Simics with an error or check the output result.
    pub fn test<S>(&mut self, script: S) -> Result<Output, CommandExtError>
    where
        S: AsRef<str>,
    {
        let test_script_path = self.project_dir.join("test.simics");
        write(test_script_path, script.as_ref())?;
        let output = Command::new("./simics")
            .current_dir(&self.project_dir)
            .arg("--batch-mode")
            .arg("--no-win")
            .arg("./test.simics")
            .check()?;
        self.cleanup_if_env()?;
        Ok(output)
    }

    /// Run a test in the environment in the form of a Simics script. To fail the test, either
    /// exit Simics with an error or check the output result.
    pub fn test_python<S>(&mut self, script: S) -> Result<Output, CommandExtError>
    where
        S: AsRef<str>,
    {
        let test_script_path = self.project_dir.join("test.py");
        write(test_script_path, script.as_ref())?;
        let output = Command::new("./simics")
            .current_dir(&self.project_dir)
            .arg("--batch-mode")
            .arg("--no-win")
            .arg("./test.py")
            .check()?;
        self.cleanup_if_env()?;
        Ok(output)
    }
}
