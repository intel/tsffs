// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Error types

use cargo_metadata::Target;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
/// An error raised during the packaging process
pub enum Error {
    #[error("No package {name} found in metadata")]
    /// No package found in metadata
    PackageNotFound {
        /// Package name
        name: String,
    },
    #[error("Invalid value build ID namespace {value:?} in 'package.metadata.simics.build-id-namespace = \"\"' field. Expected string.")]
    /// Invalid value build ID namespace
    InvalidBuildIdNamespace {
        /// The invalid value
        value: Value,
    },
    #[error("No package-number field found in metadata for {manifest}. Missing 'package.metadata.simics.package-number = 99999' to Cargo.toml?")]
    /// No package number found in metadata
    PackageNumberNotFound {
        /// Path to the manifest
        manifest: PathBuf,
    },
    #[error("Invalid package number {value} in 'package.metadata.simics.package-number = 99999' field. Expected integer.")]
    /// Invalid package number
    InvalidPackageNumber {
        /// The invalid value
        value: Value,
    },
    #[error("Invalid confidentiality {value:?} in 'package.metadata.simics.confidentiality = \"\"' field. Expected string.")]
    /// Invalid confidentiality
    InvalidConfidentiality {
        /// The invalid value
        value: Value,
    },
    #[error("Invalid access label {value:?} in 'package.metadata.simics.access-labels = [\"\", \"\"]' field. Expected string.")]
    /// Invalid access label
    InvalidAccessLabel {
        /// The invalid value
        value: Value,
    },
    #[error("No cdylib target in {targets:?}")]
    /// No cdylib target found
    CdylibTargetNotFound {
        /// The set of targets not containing a cdylib
        targets: Vec<Target>,
    },
    #[error("No parent found for path {path:?}")]
    /// No parent found
    ParentNotFound {
        /// The path with missing parent
        path: PathBuf,
    },
    #[error("No cdylib artifact found for {package:?}. Ensure the build succeeded and there is a [lib] entry in Cargo.toml with 'crate-type = [\"cdylib\"]'.")]
    /// No cdylib artifact found
    CdylibArtifactNotFound {
        /// The package with no cdylib artifact
        package: String,
    },
    #[error("Failed to convert path {path:?} to string")]
    /// Failed to convert path to string
    PathConversionError {
        /// The path that could not be converted
        path: PathBuf,
    },
    #[error("{path:?} is not a directory")]
    /// Not a directory
    NotADirectory {
        /// The path that is not a directory
        path: PathBuf,
    },
    #[error("Filename for {path:?} not found")]
    /// Filename not found
    FilenameNotFound {
        /// The path with no filename
        path: PathBuf,
    },
    #[error("Simics package metadata not found in manifest at {manifest_path:?}. Ensure there is a [package.metadata.simics] entry in Cargo.toml.")]
    /// Simics package metadata not found
    PackageMetadataNotFound {
        /// The path to the manifest with no package metadata
        manifest_path: PathBuf,
    },
    #[error("Package metadata field {field_name} missing")]
    /// Package metadata field not found
    PackageMetadataFieldNotFound {
        /// The missing field name
        field_name: String,
    },
    #[error("Package specifications is empty")]
    /// Package specifications is empty
    PackageSpecNotFound,
    #[error("Error writing package file to {path:?}: {source}")]
    /// Error writing package file
    WritePackageError {
        /// The path to the package file
        path: PathBuf,
        /// The underlying error
        source: std::io::Error,
    },
    #[error("Non-addon type packages are not supported")]
    /// Non-addon type packages are not supported
    NonAddonPackage,
    #[error(transparent)]
    /// Cargo metadata error
    CargoMetadataError(#[from] cargo_metadata::Error),
    #[error(transparent)]
    /// Parse integer error
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    /// IO error
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    /// Strip prefix error
    StripPrefixError(#[from] std::path::StripPrefixError),
    #[error(transparent)]
    /// Serde json error
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    /// Serde yaml error
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error(transparent)]
    /// Utf8 error
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    /// System time error
    SystemTimeError(#[from] std::time::SystemTimeError),
}

/// Simics packaging result type
pub type Result<T> = std::result::Result<T, Error>;
