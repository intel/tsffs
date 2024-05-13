// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Specifications for internal file formats used in the Simics packaging process

use std::{env::var, iter::once, path::PathBuf};

use crate::{Error, PackageArtifacts, Result, HOST_DIRNAME};
use cargo_metadata::{MetadataCommand, Package};
use cargo_subcommand::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::from_value;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Implements the Schema for package-specs.json
/// {
///     "$schema": "https://json-schema.org/draft/2020-12/schema",
///     "type": "array",
///     "title": "Simics Package Specification file",
///     "items": {
///         "type": "object",
///         "required": [
///             "package-name", "package-number", "name", "description",
///             "host", "version", "build-id", "build-id-namespace",
///             "confidentiality", "files"
///         ],
///         "properties": {
///             "package-name": {
///                 "type": "string"
///             },
///             "package-number": {
///                 "anyOf": [{"type": "integer"}, {"type": "null"}]
///             },
///             "name": {
///                 "type": "string"
///             },
///             "description": {
///                 "type": "string"
///             },
///             "host": {
///                 "type": "string"
///             },
///             "version": {
///                 "type": "string"
///             },
///             "build-id": {
///                 "type": "integer"
///             },
///             "build-id-namespace": {
///                 "type": "string"
///             },
///             "confidentiality": {
///                 "type": "string"
///             },
///             "files": {
///                 "type": "object",
///                 "patternProperties": {
///                     "^[^\\:]*/$": {
///                         "type": "object",
///                         "properties": {
///                             "source-directory": {
///                                 "type": "string"
///                             },
///                             "file-list": {
///                                 "type": "string"
///                             },
///                             "suffixes": {
///                                 "type": "array",
///                                 "items": {
///                                     "type": "string"
///                                 }
///                             }
///                         }
///                     },
///                     "^[^\\:]*[^/]$": {
///                         "type": "string"
///                     }
///                 }
///             },
///             "type": {
///                 "enum": ["addon", "base"]
///             },
///             "disabled": {
///                 "type": "boolean"
///             },
///             "doc-title": {
///                 "anyOf": [{"type": "string"}, {"type": "null"}]
///             },
///             "make-targets": {
///                 "type": "array",
///                 "items": {
///                     "type": "string"
///                 }
///             }
///         }
///     }
/// }
pub struct PackageSpec {
    #[serde(rename = "package-name")]
    /// The one-word alphanumeric package name, e.g. 'TSFFS-Fuzzer' in Camel-Kebab-Case
    pub package_name: String,
    #[serde(rename = "package-number")]
    /// The package number. This is the only field that must be included in the
    /// crate metadata. It must be *globally* unique.
    pub package_number: isize,
    /// The human-readable name of the package e.g. 'TSFFS Fuzzer', the package name with
    /// dashes replaced with spaces.
    pub name: String,
    /// A description of the package, e.g. 'TSFFS: The Target Software Fuzzer for SIMICS'
    pub description: String,
    /// The host this package is built for, either 'win64' or 'linux64'
    pub host: String,
    /// The version number for this package, e.g. '6.0.2' or '6.0.pre6'
    pub version: String,
    #[serde(rename = "build-id")]
    /// The build ID for this package, later versions should have later IDs. This number should
    /// monotonically increase and only has meaning between two packages with the same
    /// `build_id_namespace`
    pub build_id: isize,
    #[serde(rename = "build-id-namespace")]
    /// An identifier for the build ID, e.g. 'tsffs'
    pub build_id_namespace: String,
    /// The confidentiality of the package, e.g. 'Public', but can be any string value based on
    /// the authors confidentiality requirements.
    pub confidentiality: String,
    #[serde(default)]
    /// A mapping from the path in the package to the full path on disk of the file.
    pub files: Vec<(String, String)>,
    #[serde(rename = "type")]
    /// Either "addon" or "base", all packages should be 'addon'
    pub typ: String,
    /// Whether the package is disabled, default is not disabled
    pub disabled: bool,
    #[serde(rename = "doc-title")]
    /// The title used in documentation for the package
    pub doc_title: String,
    #[serde(rename = "make-targets")]
    /// The list of targets to build for this package
    pub make_targets: Vec<String>,
    #[serde(rename = "include-release-notes")]
    /// Whether release notes should be included in the package, not included by default
    pub include_release_notes: bool,
    #[serde(rename = "ip-plans")]
    /// Plans for the IP of this package. Typically empty.
    pub ip_plans: Vec<String>,
    #[serde(rename = "legacy-doc-make-targets")]
    /// Legacy support for doc make targets. Typically empty.
    pub legacy_doc_make_targets: Vec<String>,
    #[serde(rename = "release-notes")]
    /// Release notes. Typically empty.
    pub release_notes: Vec<String>,
    #[serde(rename = "access-labels")]
    /// Labels for managing package access, e.g. 'external-intel'
    pub access_labels: Vec<String>,
}

impl PackageSpec {
    /// Create a package spec by reading the manifest specified by a subcommand
    pub fn from_subcommand(subcommand: &Subcommand) -> Result<Self> {
        let manifest_spec = ManifestPackageSpec::from_subcommand(subcommand)?;
        Ok(Self {
            package_name: manifest_spec.package_name.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "package_name".to_string(),
                }
            })?,
            package_number: manifest_spec.package_number.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "package_number".to_string(),
                }
            })?,
            name: manifest_spec
                .name
                .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                    field_name: "name".to_string(),
                })?,
            description: manifest_spec.description.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "description".to_string(),
                }
            })?,
            host: manifest_spec
                .host
                .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                    field_name: "host".to_string(),
                })?,
            version: manifest_spec
                .version
                .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                    field_name: "version".to_string(),
                })?,
            build_id: manifest_spec.build_id.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "build_id".to_string(),
                }
            })?,
            build_id_namespace: manifest_spec.build_id_namespace.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "build_id_namespace".to_string(),
                }
            })?,
            confidentiality: manifest_spec.confidentiality.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "confidentiality".to_string(),
                }
            })?,
            files: manifest_spec.files.clone(),
            typ: manifest_spec
                .typ
                .ok_or_else(|| Error::PackageMetadataFieldNotFound {
                    field_name: "type".to_string(),
                })?,
            disabled: manifest_spec.disabled,
            doc_title: manifest_spec.doc_title.ok_or_else(|| {
                Error::PackageMetadataFieldNotFound {
                    field_name: "doc_title".to_string(),
                }
            })?,
            make_targets: manifest_spec.make_targets.clone(),
            include_release_notes: manifest_spec.include_release_notes,
            ip_plans: manifest_spec.ip_plans.clone(),
            legacy_doc_make_targets: manifest_spec.legacy_doc_make_targets.clone(),
            release_notes: manifest_spec.release_notes.clone(),
            access_labels: manifest_spec.access_labels.clone(),
        })
    }

    /// Add a set of artifacts (not specified in the manifest) to the specification
    pub fn with_artifacts(mut self, artifacts: &PackageArtifacts) -> Self {
        self.files = artifacts.files.clone();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// A package specification deserialized from the
///
/// [package.metadata.simics]
///
/// field in Cargo.toml. This specification is used to generate the real specification, and many
/// options left optional in the manifest are not optional to Simics. Sane defaults are provided
/// for all options.
pub struct ManifestPackageSpec {
    #[serde(rename = "package-name", default)]
    /// The one-word alphanumeric package name, e.g. 'TSFFS-Fuzzer' in Camel-Kebab-Case
    package_name: Option<String>,
    #[serde(rename = "package-number", default)]
    /// The package number. This is the only field that must be included in the
    /// crate metadata. It must be *globally* unique.
    package_number: Option<isize>,
    #[serde(default)]
    /// The human-readable name of the package e.g. 'TSFFS Fuzzer', the package name with
    /// dashes replaced with spaces.
    name: Option<String>,
    #[serde(default)]
    /// A description of the package, e.g. 'TSFFS: The Target Software Fuzzer for SIMICS'
    description: Option<String>,
    #[serde(default)]
    /// The host this package is built for, either 'win64' or 'linux64'
    host: Option<String>,
    #[serde(default)]
    /// The version number for this package, e.g. '6.0.2' or '6.0.pre6'
    version: Option<String>,
    #[serde(rename = "build-id", default)]
    /// The build ID for this package, later versions should have later IDs. This number should
    /// monotonically increase and only has meaning between two packages with the same
    /// `build_id_namespace`
    build_id: Option<isize>,
    #[serde(rename = "build-id-namespace", default)]
    /// An identifier for the build ID, e.g. 'tsffs'
    build_id_namespace: Option<String>,
    #[serde(default)]
    /// The confidentiality of the package, e.g. 'Public', but can be any string value based on
    /// the authors confidentiality requirements.
    confidentiality: Option<String>,
    #[serde(default)]
    /// A mapping from the path in the package to the full path on disk of the file.
    files: Vec<(String, String)>,
    #[serde(rename = "type", default)]
    // Either "addon" or "base", all packages should be 'addon'
    typ: Option<String>,
    #[serde(default)]
    /// Whether the package is disabled, default is not disabled
    disabled: bool,
    #[serde(rename = "doc-title", default)]
    /// The title used in documentation for the package
    doc_title: Option<String>,
    #[serde(rename = "make-targets", default)]
    /// The list of targets to build for this package
    make_targets: Vec<String>,
    #[serde(rename = "include-release-notes", default)]
    /// Whether release notes should be included in the package, not included by default
    include_release_notes: bool,
    #[serde(rename = "ip-plans", default)]
    ip_plans: Vec<String>,
    #[serde(rename = "legacy-doc-make-targets", default)]
    legacy_doc_make_targets: Vec<String>,
    #[serde(rename = "release-notes", default)]
    release_notes: Vec<String>,
    #[serde(rename = "access-labels", default)]
    /// Labels for managing package access, e.g. 'external-intel'
    access_labels: Vec<String>,
}

impl ManifestPackageSpec {
    /// Return the default type when deserializing
    pub fn default_type() -> String {
        "addon".to_string()
    }
}

impl ManifestPackageSpec {
    /// Create a specification from the package metadata returned from a cargo metadata
    /// invocation
    pub fn from_package(package: &Package) -> Result<Self> {
        let mut spec: ManifestPackageSpec = if let Some(spec) = package.metadata.get("simics") {
            from_value(spec.clone()).map_err(Error::from)?
        } else {
            ManifestPackageSpec::default()
        };

        if spec.package_number.is_none() {
            // Zero is a safe default for package number, but it is not a valid package number
            // so a real package must obtain a package number when it is published.
            spec.package_number = Some(0);
        }

        if spec.package_name.is_none() {
            spec.package_name = Some(package.name.clone());
        }

        if spec.name.is_none() {
            spec.name = Some(package.name.clone());
        }

        if spec.description.is_none() {
            spec.description = package.description.clone();
        }

        if spec.host.is_none() {
            spec.host = Some(HOST_DIRNAME.to_string());
        }

        if spec.version.is_none() {
            spec.version = Some(package.version.to_string());
        }

        if spec.build_id.is_none() {
            spec.build_id = Some(
                package
                    .version
                    .to_string()
                    .chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .map_err(Error::from)?,
            )
        }

        if spec.build_id_namespace.is_none() {
            spec.build_id_namespace = Some(package.name.clone());
        }

        if spec.confidentiality.is_none() {
            spec.confidentiality = Some("Public".to_string());
        }

        if spec.typ.is_none() {
            spec.typ = Some("addon".to_string());
        }

        if spec.doc_title.is_none() {
            spec.doc_title = Some(package.name.clone());
        }

        if let Ok(package_name) = var("SIMICS_PACKAGE_PACKAGE_NAME") {
            spec.package_name = Some(package_name);
        }

        if let Ok(package_number) = var("SIMICS_PACKAGE_PACKAGE_NUMBER") {
            spec.package_number = Some(package_number.parse().map_err(Error::from)?);
        }

        if let Ok(package_name) = var("SIMICS_PACKAGE_NAME") {
            spec.name = Some(package_name);
        }

        if let Ok(description) = var("SIMICS_PACKAGE_DESCRIPTION") {
            spec.description = Some(description);
        }

        if let Ok(host) = var("SIMICS_PACKAGE_HOST") {
            spec.host = Some(host);
        }

        if let Ok(version) = var("SIMICS_PACKAGE_VERSION") {
            spec.version = Some(version);
        }

        if let Ok(build_id) = var("SIMICS_PACKAGE_BUILD_ID") {
            spec.build_id = Some(build_id.parse().map_err(Error::from)?);
        }

        if let Ok(build_id_namespace) = var("SIMICS_PACKAGE_BUILD_ID_NAMESPACE") {
            spec.build_id_namespace = Some(build_id_namespace);
        }

        if let Ok(confidentiality) = var("SIMICS_PACKAGE_CONFIDENTIALITY") {
            spec.confidentiality = Some(confidentiality);
        }

        if let Ok(typ) = var("SIMICS_PACKAGE_TYPE") {
            spec.typ = Some(typ);
        }

        if let Ok(doc_title) = var("SIMICS_PACKAGE_DOC_TITLE") {
            spec.doc_title = Some(doc_title);
        }

        Ok(spec)
    }

    /// Read the manifest specified by the subcommand and parse it into a package specification.
    pub fn from_subcommand(subcommand: &Subcommand) -> Result<Self> {
        Self::from_package(
            MetadataCommand::new()
                .manifest_path(subcommand.manifest())
                .no_deps()
                .exec()?
                .packages
                .iter()
                .find(|p| p.name == subcommand.package())
                .ok_or_else(|| Error::PackageNotFound {
                    name: subcommand.package().to_string(),
                })?,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A list of package specifications. This data structure can be written to a package-specs.json
/// file and consumed by Simics packaging utilities.
pub struct PackageSpecs(pub Vec<PackageSpec>);

impl PackageSpecs {
    /// Generate the list of specifications from a subcommand input
    pub fn from_subcommand(subcommand: &Subcommand) -> Result<Self> {
        Ok(Self(vec![PackageSpec::from_subcommand(subcommand)?]))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Output format for the ispm-metadata file at the top-level of the package.
/// It contains a subset of the package spec information
pub struct IspmMetadata {
    /// The human-readable name of the package
    pub name: String,
    #[serde(rename = "packageNumber")]
    /// The package number
    pub package_number: isize,
    /// The package version
    pub version: String,
    #[serde(rename = "packageName")]
    /// The package name, which should be Camel-Kebab-Cased.
    pub package_name: String,
    /// The package kind, typically "addon"
    pub kind: String,
    /// The host supporting this package, either linux64 or win64
    pub host: String,
    /// The confidentiality setting of this package
    pub confidentiality: String,
    #[serde(rename = "buildId")]
    /// The build ID of this package
    pub build_id: String,
    #[serde(rename = "buildIdNamespace")]
    /// The namespace for which the build ID of this package is valid
    pub build_id_namespace: String,
    /// The description of this package
    pub description: String,
    #[serde(rename = "uncompressedSize")]
    /// The size of the inner package.tar.gz file as given by du -sb <dir>
    pub uncompressed_size: usize,
}

impl From<&PackageSpec> for IspmMetadata {
    fn from(value: &PackageSpec) -> Self {
        let value = value.clone();
        Self {
            name: value.name,
            package_number: value.package_number,
            version: value.version,
            package_name: value.package_name,
            kind: value.typ,
            host: value.host,
            confidentiality: value.confidentiality,
            build_id: value.build_id.to_string(),
            build_id_namespace: value.build_id_namespace,
            description: value.description,
            uncompressed_size: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// The package info file, which is a subset of the package spec and is added into the
/// inner tarball at /package-dir-name/packageinfo/full_package_name
pub struct PackageInfo {
    /// The human-readable name of the package
    pub name: String,
    /// The description of the package
    pub description: String,
    /// The version of the package
    pub version: String,
    /// The host supporting this package, either linux64 or win64
    pub host: String,
    #[serde(rename = "package-name")]
    /// The package name, which should be Camel-Kebab-Cased.
    pub package_name: String,
    #[serde(rename = "package-number")]
    /// The package number
    pub package_number: isize,
    #[serde(rename = "build-id")]
    /// The build ID of this package
    pub build_id: isize,
    #[serde(rename = "build-id-namespace")]
    /// The namespace for which the build ID of this package is valid
    pub build_id_namespace: String,
    #[serde(rename = "type")]
    /// The package kind, typically "addon"
    pub typ: String,
    #[serde(rename = "extra-version", default)]
    /// An extra version string, usually empty
    pub extra_version: String,
    /// The confidentiality setting of this package
    pub confidentiality: String,
    #[serde(skip)]
    // Files are skipped when serializing and must be serialized separately because the output
    // format is not exactly YAML: it needs to output like:
    // files:
    //     top-level/file1
    //     top-level/file2
    //     top-level/dir1/file3
    /// A list of files present in the package
    pub files: Vec<String>,
}

impl From<&PackageSpec> for PackageInfo {
    fn from(value: &PackageSpec) -> Self {
        let dirname = format!("simics-{}-{}", value.package_name, value.version);
        let self_file = PathBuf::from(dirname)
            .join("packageinfo")
            .join(format!("{}-{}", value.package_name, value.host));
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            version: value.version.clone(),
            host: value.host.clone(),
            package_name: value.package_name.clone(),
            package_number: value.package_number,
            build_id: value.build_id,
            build_id_namespace: value.build_id_namespace.clone(),
            typ: value.typ.clone(),
            confidentiality: value.confidentiality.clone(),
            files: value
                .files
                .iter()
                .map(|f| f.0.clone())
                .chain(once(self_file.to_str().unwrap_or_default().to_string()))
                .collect(),
            ..Default::default()
        }
    }
}

impl PackageInfo {
    /// Get the list of files for this package info file. Because the file is not exactly YAML,
    /// deserializing the `files` list returns a list like:
    /// files:
    /// - file1
    /// - dir1/file2
    ///
    /// But it must actually be formatted like:
    // files:
    //     top-level/file1
    //     top-level/file2
    //     top-level/dir1/file3
    ///
    /// This method returns in the second format.
    pub fn files(&self) -> String {
        "files:\n".to_string()
            + &self
                .files
                .iter()
                .map(|f| format!("    {}", f))
                .collect::<Vec<String>>()
                .join("\n")
            + "\n"
    }
}
