// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

// #![deny(missing_docs)]

use artifact_dependency::ARTIFACT_NAMEPARTS;
use cargo_subcommand::{Args, Subcommand};
use clap::Parser;
use command_ext::CommandExtCheck;
use ispm_wrapper::ispm::{self, GlobalOptions};
use itertools::Itertools;
use simics_package::Package;
use simics_sign::Sign;
use std::{
    env::var,
    fs::{copy, read_dir},
    io::BufRead,
    path::PathBuf,
    process::Command,
    time::SystemTime,
};

#[derive(Debug, thiserror::Error)]
/// An error raised during build
pub enum Error {
    #[error("env.SIMICS_BASE variable set in config.toml but could not be parsed from {output:?}")]
    /// Raised when the SIMICS_BASE environment variable is set but could not be parsed
    SimicsBaseParseError { output: Option<String> },
    #[error("The SIMICS_BASE environment variable was not set or present in config.toml, and no packages were installed.")]
    /// Raised when the SIMICS_BASE environment variable was not set or present in config.toml, and no base package was installed
    NoInstalledPackages,
    #[error("The SIMICS_BASE environment variable was not set or present in config.toml, and no base package was installed.")]
    /// Raised when the SIMICS_BASE environment variable was not set or present in config.toml, and no base package was installed
    NoBasePackage,
    #[error("Base package found, but no paths registered for package number 1000")]
    /// Raised when a base package is found, but no paths are registered for package number 1000
    NoPathsForBasePackage,
    #[error("Base package directory {path:?} does not exist")]
    /// Raised when the base package directory does not exist
    BasePackageDirectoryDoesNotExist { path: PathBuf },
    #[error("No cdylib crate artifact found for package {package}. Ensure the build succeeded and there is a [lib] entry in Cargo.toml with crate-type 'cdylib'.")]
    /// Raised when no cdylib crate artifact is found for a package
    NoCdylibArtifact { package: String },
    #[error("No parent directory found for {path:?}")]
    /// Raised when no parent directory is found for a path
    NoParentDirectory { path: PathBuf },
    #[error("No filename found for {path:?}")]
    /// Raised when no filename is found for a path
    NoFilename { path: PathBuf },
    #[error("Failed to copy library from {from:?} to {to:?}: {source:?}")]
    /// An error occurred while copying a library
    CopyLibrary {
        /// The source path
        from: PathBuf,
        /// The destination path
        to: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Failed to read directory {path:?}: {source}")]
    /// An error occurred while reading a directory
    ReadDirectory {
        /// The path to the directory that could not be read
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error(transparent)]
    /// A wrapped std::io::Error
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    /// Any wrapped other error
    Other(#[from] anyhow::Error),
    #[error(transparent)]
    /// A wrapped std::env::VarError
    VarError(#[from] std::env::VarError),
    #[error(transparent)]
    /// A wrapped subcommand error
    SubcommandError(#[from] cargo_subcommand::Error),
    #[error(transparent)]
    /// A wrapped CommandExt error
    CommandExtError(#[from] command_ext::CommandExtError),
    #[error(transparent)]
    /// A wrapped signature error
    SignatureError(#[from] simics_sign::Error),
    #[error(transparent)]
    /// A wrapped package error
    PackageError(#[from] simics_package::Error),
    #[error(transparent)]
    /// A wrapped from utf8 error
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    #[clap(subcommand)]
    pub simics_build: SimicsBuildCmd,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SimicsBuildCmd {
    /// Helps cargo build apks for Android
    SimicsBuild {
        #[clap(flatten)]
        args: Args,
        #[clap(long)]
        simics_base: Option<PathBuf>,
    },
}

pub struct App;

type Result<T> = std::result::Result<T, Error>;

impl App {
    pub fn run(cmd: Cmd) -> Result<PathBuf> {
        let SimicsBuildCmd::SimicsBuild { args, simics_base } = cmd.simics_build;

        let subcommand = Subcommand::new(args)?;
        let cargo = var("CARGO")?;

        // First, check if `env.SIMICS_BASE` is set:
        let simics_base = if let Some(simics_base) = simics_base {
            simics_base.clone()
        } else if let Ok(output) = Command::new(&cargo)
            .arg("-Zunstable-options")
            .arg("config")
            .arg("get")
            .arg("env.SIMICS_BASE")
            .check()
        {
            let line = output.stdout.lines().next().transpose()?;
            line.clone()
                .and_then(|l| l.split('=').last().map(|s| s.trim().replace('"', "")))
                .map(PathBuf::from)
                .ok_or_else(|| Error::SimicsBaseParseError { output: line })?
        } else if let Ok(simics_base) = var("SIMICS_BASE") {
            PathBuf::from(simics_base)
        } else {
            if !subcommand.quiet() {
                println!("No SIMICS_BASE variable set, using the latest installed package with package number 1000")
            }

            let mut packages = ispm::packages::list(&GlobalOptions::default())?;
            packages.sort();
            let Some(installed) = packages.installed_packages.as_ref() else {
                return Err(Error::NoInstalledPackages);
            };
            let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
                return Err(Error::NoBasePackage);
            };
            base.paths
                .first()
                .ok_or_else(|| Error::NoPathsForBasePackage)?
                .clone()
        };

        if !simics_base.exists() {
            return Err(Error::BasePackageDirectoryDoesNotExist { path: simics_base });
        }

        // Clean the package's release profile
        if !subcommand.quiet() {
            println!("Building package {}", subcommand.package());
        }

        // Build the package
        let mut build_cmd = Command::new(&cargo);
        build_cmd.arg("rustc");
        build_cmd.env("SIMICS_BASE", simics_base);
        subcommand.args().apply(&mut build_cmd);
        #[cfg(unix)]
        build_cmd.args(["--", "-C", "link-args=-Wl,--gc-sections"]);
        build_cmd.check()?;

        // Get the module cdylib
        let module_cdylib = subcommand
            .artifacts()
            .map(|a| {
                subcommand.build_dir(subcommand.target()).join(format!(
                    "{}{}{}",
                    ARTIFACT_NAMEPARTS.0,
                    a.name.replace('-', "_"),
                    ARTIFACT_NAMEPARTS.1
                ))
            })
            .find(|p| p.exists())
            .ok_or_else(|| Error::NoCdylibArtifact {
                package: subcommand.package().to_string(),
            })?;

        // Sign the module cdylib
        if !subcommand.quiet() {
            println!("Signing module {module_cdylib:?}");
        }

        let mut signed = Sign::new(&module_cdylib)?;

        let signed_module_cdylib = module_cdylib
            .parent()
            .ok_or_else(|| Error::NoParentDirectory {
                path: module_cdylib.to_path_buf(),
            })?
            .join({
                let file_name = module_cdylib
                    .file_name()
                    .ok_or_else(|| Error::NoFilename {
                        path: module_cdylib.to_path_buf(),
                    })?
                    .to_str()
                    .ok_or_else(|| Error::NoFilename {
                        path: module_cdylib.to_path_buf(),
                    })?;
                let module_cdylib_dir =
                    module_cdylib
                        .parent()
                        .ok_or_else(|| Error::NoParentDirectory {
                            path: module_cdylib.to_path_buf(),
                        })?;

                module_cdylib_dir
                    .join(file_name.replace('_', "-"))
                    .to_str()
                    .ok_or_else(|| Error::NoFilename {
                        path: module_cdylib.to_path_buf(),
                    })?
            });

        signed.write(&signed_module_cdylib)?;

        let target_profile_build_dir = subcommand.build_dir(subcommand.target()).join("build");

        // Find interfaces
        let target_profile_build_subdirs = read_dir(&target_profile_build_dir)
            .map_err(|e| Error::ReadDirectory {
                path: target_profile_build_dir,
                source: e,
            })?
            .filter_map(|rd| rd.ok())
            .map(|de| de.path())
            .filter(|p| {
                p.is_dir()
                    && p.file_name().is_some_and(|n| {
                        n.to_str()
                            .is_some_and(|ns| ns.starts_with(subcommand.package()))
                    })
                    && !p
                        .join(format!("build-script-build{}", ARTIFACT_NAMEPARTS.4))
                        .exists()
                    && p.join("out").is_dir()
            })
            .collect::<Vec<_>>();

        // Source, Destination of interface libraries
        let cdylib_out_artifacts = target_profile_build_subdirs
            .iter()
            .map(|bd| bd.join("out"))
            .map(|od| {
                read_dir(&od)
                    .map_err(|e| Error::ReadDirectory {
                        path: od.clone(),
                        source: e,
                    })
                    .map(|rd| {
                        Ok(rd
                            .filter_map(|rd| rd.ok())
                            .map(|de| de.path())
                            .filter(|p| {
                                p.file_name().is_some_and(|n| {
                                    n.to_str().is_some_and(|ns| {
                                        ns.starts_with(ARTIFACT_NAMEPARTS.0)
                                            && ns.ends_with(ARTIFACT_NAMEPARTS.1)
                                    })
                                })
                            })
                            .collect::<Vec<_>>())
                    })?
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .filter_map(|p| {
                p.clone()
                    .file_name()
                    .and_then(|n| n.to_str().map(|n| (p, n.to_string())))
            })
            .sorted_by(|(_, a), (_, b)| a.cmp(b))
            .chunk_by(|(_, n)| n.clone())
            // Get the newest one
            .into_iter()
            .filter_map(|(_, g)| {
                g.max_by_key(|(p, _)| {
                    p.metadata()
                        .map(|m| m.modified().unwrap_or(SystemTime::UNIX_EPOCH))
                        .unwrap_or_else(|_| SystemTime::UNIX_EPOCH)
                })
            })
            .map(|(a, _)| {
                a.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| Error::NoFilename { path: a.clone() })
                    .map(|t| (a.clone(), subcommand.build_dir(subcommand.target()).join(t)))
            })
            .collect::<Result<Vec<_>>>()?;

        // Sign all the interfaces, re-signing if needed, and copy them to the build directory
        cdylib_out_artifacts.iter().try_for_each(|(a, t)| {
            if !subcommand.quiet() {
                println!("Copying interface library {a:?}");
            }

            copy(a, t).map_err(|e| Error::CopyLibrary {
                from: a.clone(),
                to: t.clone(),
                source: e,
            })?;

            Ok::<(), anyhow::Error>(())
        })?;

        let package = Package::from_subcommand(&subcommand)?
            .build(subcommand.build_dir(subcommand.target()))?;

        if !subcommand.quiet() {
            println!("Built ISPM package {:?}", package);
        }

        Ok(package)
    }
}
