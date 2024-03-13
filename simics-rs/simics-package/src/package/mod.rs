// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! An ISPM package which can be built from a subcommand invocation and output to a directory
//! on disk.

use crate::{Error, IspmMetadata, PackageArtifacts, PackageInfo, PackageSpec, Result};
use cargo_subcommand::Subcommand;
use flate2::{write::GzEncoder, Compression};
#[cfg(unix)]
use std::time::SystemTime;
use std::{
    fs::write,
    path::{Path, PathBuf},
};
use tar::{Builder, Header};
use typed_builder::TypedBuilder;

#[cfg(unix)]
/// The directory name for the linux host
pub const HOST_DIRNAME: &str = "linux64";

#[cfg(windows)]
/// The directory name for the windows host
pub const HOST_DIRNAME: &str = "win64";

#[derive(TypedBuilder, Debug, Clone)]
/// A package, which is built from a specification written in a cargo manifest and a set of
/// artifacts pulled from the target profile directory
pub struct Package {
    /// The specification, which is written in [package.metadata.simics] in the crate manifest
    /// of the crate to package
    pub spec: PackageSpec,
    /// The target profile directory from which to pull artifacts and output the built package
    pub target_profile_dir: PathBuf,
}

impl Package {
    /// The name of the inner package file which decompresses to the package directory
    pub const INNER_PACKAGE_FILENAME: &'static str = "package.tar.gz";
    /// The name of the file containing metadata for ISPM to use when installing the package
    pub const METADATA_FILENAME: &'static str = "ispm-metadata";
    /// The name of an addon package type
    pub const ADDON_TYPE: &'static str = "addon";
    /// Default level used by simics
    pub const COMPRESSION_LEVEL: u32 = 6;

    /// Instantiate a package from a cargo subcommand input, which is parsed from command line
    /// arguments
    pub fn from_subcommand(subcommand: &Subcommand) -> Result<Self> {
        let target_profile_dir = subcommand.build_dir(subcommand.target());

        let spec = PackageSpec::from_subcommand(subcommand)?
            .with_artifacts(&PackageArtifacts::from_subcommand(subcommand)?);

        Ok(Self {
            spec,
            target_profile_dir,
        })
    }

    /// Construct the directory name of the package after expansion. It is an error to build a
    /// Rust crate package into any type other than an addon package (simics base is not a Rust
    /// package)
    pub fn package_dirname(&self) -> Result<String> {
        if self.spec.typ == Self::ADDON_TYPE {
            Ok(format!(
                "simics-{}-{}",
                self.spec.package_name, self.spec.version
            ))
        } else {
            Err(Error::NonAddonPackage)
        }
    }

    /// Construct the full package name, which includes the host directory name
    pub fn full_package_name(&self) -> String {
        format!("{}-{}", self.spec.package_name, self.spec.host)
    }

    /// Construct the package name, which is the package number and version, without an
    /// extension
    pub fn package_name(&self) -> String {
        format!(
            "simics-pkg-{}-{}",
            self.spec.package_number, self.spec.version
        )
    }

    /// Construct the package name with the host directory name
    pub fn package_name_with_host(&self) -> String {
        format!("{}-{}", self.package_name(), self.spec.host)
    }

    /// Construct the filename for the output of this ISPM package
    pub fn package_filename(&self) -> String {
        format!("{}.ispm", self.package_name_with_host())
    }

    #[cfg(unix)]
    /// Set common options on a tar header. On Unix, the modified time is set to the current
    /// time and the uid/gid are set to the current user.
    pub fn set_header_common(header: &mut Header) -> Result<()> {
        use libc::{getgid, getpwuid, getuid};
        use std::ffi::CStr;

        header.set_mtime(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        );
        header.set_uid(unsafe { getuid() } as u64);
        header.set_gid(unsafe { getgid() } as u64);
        let username = unsafe {
            CStr::from_ptr(
                getpwuid(getuid())
                    .as_ref()
                    .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                        field_name: "username".to_string(),
                    })?
                    .pw_name,
            )
        }
        .to_str()?
        .to_string();
        let groupname = unsafe {
            CStr::from_ptr(
                getpwuid(getuid())
                    .as_ref()
                    .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                        field_name: "groupname".to_string(),
                    })?
                    .pw_name,
            )
        }
        .to_str()?
        .to_string();
        header.set_username(&username)?;
        header.set_groupname(&groupname)?;
        header.set_mode(0o755);

        Ok(())
    }

    #[cfg(windows)]
    /// On windows, no additional options need to be set for headers and this method is a no-op
    pub fn set_header_common(_header: &mut Header) -> Result<()> {
        Ok(())
    }

    /// Create the inner package.tar.gz tarball which expands to the simics package.
    pub fn create_inner_tarball(&self) -> Result<(Vec<u8>, usize)> {
        let tar_gz = Vec::new();
        let encoder = GzEncoder::new(tar_gz, Compression::new(Self::COMPRESSION_LEVEL));
        let mut tar = Builder::new(encoder);
        // The uncompressed size is used by simics, and must be calculated the way simics
        // expects
        let mut uncompressed_size = 0;

        // Add the packageinfo to the inner package tarball
        let package_info = PackageInfo::from(&self.spec);
        let package_info_string = serde_yaml::to_string(&package_info)? + &package_info.files();
        let package_info_data = package_info_string.as_bytes();
        uncompressed_size += package_info_data.len();
        let mut metadata_header = Header::new_gnu();
        metadata_header.set_size(package_info_data.len() as u64);
        Self::set_header_common(&mut metadata_header)?;
        tar.append_data(
            &mut metadata_header,
            PathBuf::from(self.package_dirname()?)
                .join("packageinfo")
                .join(self.full_package_name()),
            package_info_data,
        )?;
        self.spec.files.iter().try_for_each(|(pkg_loc, src_loc)| {
            let src_path = PathBuf::from(src_loc);
            uncompressed_size += src_path.metadata()?.len() as usize;
            tar.append_path_with_name(src_path, pkg_loc)?;
            Ok::<(), Error>(())
        })?;

        tar.finish()?;

        Ok((tar.into_inner()?.finish()?, uncompressed_size))
    }

    /// Create the outer tarball (actually an ISPM package) containing the inner package and a
    /// metadata file used by ISPM
    pub fn create_tarball(&self) -> Result<Vec<u8>> {
        let tar_gz = Vec::new();
        let encoder = GzEncoder::new(tar_gz, Compression::new(Self::COMPRESSION_LEVEL));
        let mut tar = Builder::new(encoder);
        let (inner_tarball, uncompressed_size) = self.create_inner_tarball()?;

        let mut ispm_metadata = IspmMetadata::from(&self.spec);
        // This size should be exactly equal to the total size of the files in the inner tarball
        // (equal to the size given by du -sb <extracted-tarball-dir>) and does not include the
        // size of the ispm-metadata file itself
        ispm_metadata.uncompressed_size = uncompressed_size;

        let ispm_metadata_string = serde_json::to_string(&ispm_metadata)?;
        let ispm_metadata_data = ispm_metadata_string.as_bytes();
        let mut ispm_metadata_header = Header::new_gnu();
        ispm_metadata_header.set_size(ispm_metadata_data.len() as u64);
        Self::set_header_common(&mut ispm_metadata_header)?;
        tar.append_data(
            &mut ispm_metadata_header,
            Self::METADATA_FILENAME,
            ispm_metadata_data,
        )?;

        let mut inner_tarball_header = Header::new_gnu();
        inner_tarball_header.set_size(inner_tarball.len() as u64);
        Self::set_header_common(&mut inner_tarball_header)?;
        tar.append_data(
            &mut inner_tarball_header,
            Self::INNER_PACKAGE_FILENAME,
            inner_tarball.as_slice(),
        )?;

        tar.finish()?;

        Ok(tar.into_inner()?.finish()?)
    }

    /// Build the package, writing it to the directory specified by `output` and returning
    /// the path to the package
    pub fn build<P>(&mut self, output: P) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        let package_dirname = PathBuf::from(self.package_dirname()?);

        // Rewrite the in-package paths of the spec's files so they begin with the package
        // directory name. This must be done *before* creating the inner tarball and before
        // the package info structure is created because it needs these prefix paths to be
        // present
        self.spec.files.iter_mut().try_for_each(|pkg_src_loc| {
            pkg_src_loc.0 = package_dirname
                .join(&pkg_src_loc.0)
                .to_str()
                .ok_or_else(|| Error::PathConversionError {
                    path: package_dirname.join(&pkg_src_loc.0),
                })?
                .to_string();
            Ok::<(), Error>(())
        })?;

        let tarball = self.create_tarball()?;
        let path = output.as_ref().join(self.package_filename());

        write(&path, tarball).map_err(|e| Error::WritePackageError {
            path: path.clone(),
            source: e,
        })?;

        Ok(path)
    }
}
