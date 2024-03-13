// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Artifacts for a package. Typically, these artifacts are the main library for the package and
//! any interfaces which are built as separate libraries by the interface declaration.

use crate::{Error, Result, HOST_DIRNAME};
use artifact_dependency::ARTIFACT_NAMEPARTS;
use cargo_subcommand::Subcommand;
use std::{fs::read_dir, path::PathBuf};
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Debug, Clone, Default)]
/// A set of artifacts that will be added into a Simics package
pub struct PackageArtifacts {
    /// Source paths of signed libraries in the build directory. These will be copied into
    /// $(HOST)/lib/
    pub libs: Vec<PathBuf>,
    /// Files mapping of in-package to on-disk files which will be used to generate the
    /// package spec
    pub files: Vec<(String, String)>,
}

impl PackageArtifacts {
    /// Create a new `PackageArtifacts` from a `Subcommand` by reading the crate state and
    /// obtaining build results including macro-built interfaces.
    pub fn from_subcommand(subcommand: &Subcommand) -> Result<Self> {
        let module_cdylib = subcommand
            .artifacts()
            .map(|a| {
                subcommand.build_dir(subcommand.target()).join(format!(
                    "{}{}{}",
                    ARTIFACT_NAMEPARTS.0,
                    a.name.replace('_', "-"),
                    ARTIFACT_NAMEPARTS.1
                ))
            })
            .find(|p| p.exists())
            .ok_or_else(|| Error::CdylibArtifactNotFound {
                package: subcommand.package().to_string(),
            })?;

        let module_artifact = PathBuf::from({
            let file_name = module_cdylib
                .file_name()
                .ok_or_else(|| Error::FilenameNotFound {
                    path: module_cdylib.to_path_buf(),
                })?
                .to_str()
                .ok_or_else(|| Error::FilenameNotFound {
                    path: module_cdylib.to_path_buf(),
                })?;
            let module_cdylib_dir =
                module_cdylib
                    .parent()
                    .ok_or_else(|| Error::ParentNotFound {
                        path: module_cdylib.to_path_buf(),
                    })?;

            module_cdylib_dir
                .join(file_name.replace('_', "-"))
                .to_str()
                .ok_or_else(|| Error::FilenameNotFound {
                    path: module_cdylib.to_path_buf(),
                })?
        });

        let target_profile_build_dir = subcommand.build_dir(subcommand.target()).join("build");

        // Find interfaces
        let target_profile_build_subdirs = read_dir(target_profile_build_dir)?
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
                read_dir(od).map(|rd| {
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
            .map(|a| {
                a.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| Error::FilenameNotFound { path: a.clone() })
                    .map(|t| (a.clone(), subcommand.build_dir(subcommand.target()).join(t)))
            })
            .collect::<Result<Vec<_>>>()?;

        let cdylib_out_artifacts = cdylib_out_artifacts
            .iter()
            .map(|a| a.1.clone())
            .collect::<Vec<_>>();

        let libs = vec![module_artifact]
            .into_iter()
            .chain(cdylib_out_artifacts)
            .collect::<Vec<_>>();

        let host_dir = PathBuf::from(HOST_DIRNAME);
        let lib_dir = host_dir.join("lib");

        // Build the mapping from in-package relative path (which will be prefixed with the
        // package directory name) to the on-disk path the file currently resides at. This is
        // used later to generate the package tarball by appending all of these files at their
        // correct locations
        let files = libs
            .iter()
            .map(|file_path| {
                file_path
                    .canonicalize()
                    .map_err(Error::from)
                    .and_then(|file_path| {
                        file_path
                            .file_name()
                            .ok_or_else(|| Error::FilenameNotFound {
                                path: file_path.clone(),
                            })
                            .and_then(|file_name| {
                                lib_dir
                                    .join(file_name)
                                    .to_str()
                                    .ok_or_else(|| Error::PathConversionError {
                                        path: lib_dir.join(file_name),
                                    })
                                    .and_then(|packaged_file_path| {
                                        Ok((
                                            packaged_file_path.to_string(),
                                            file_path
                                                .to_str()
                                                .ok_or_else(|| Error::PathConversionError {
                                                    path: file_path.clone(),
                                                })?
                                                .to_string(),
                                        ))
                                    })
                            })
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { libs, files })
    }
}
