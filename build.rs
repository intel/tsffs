#!/usr/bin/env -S cargo +nightly -Z script

//! This script builds and packages the TSFFS package. It simply:
//! - Invokes make to build the module
//! - Invokes make again to ensure the interface is up to date
//! - Generates the packages.list file for the module and interface
//! - Calls the `create-package-specs` script
//! - Calls the `create-modcache` script
//! - Calls the `create-packages` script
//!
//! ```cargo
//! [dependencies]
//! anyhow = "*"
//! command-ext = { path = "modules/tsffs/src/util/command-ext" }
//! typed-builder = "*"
//! walkdir = "*"
//! serde = { version = "*", features = ["derive"] }
//! serde_json = "*"
//! cargo_metadata = "*"
//! ```

use anyhow::{anyhow, bail, Error, Result};
use cargo_metadata::MetadataCommand;
use command_ext::CommandExtCheck;
use serde::{Deserialize, Serialize};
use serde_json::{to_string, value::Value};
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{create_dir_all, write},
    iter::once,
    path::{Path, PathBuf},
    process::Command,
};
use typed_builder::TypedBuilder;
use walkdir::WalkDir;

fn make() -> Result<()> {
    #[cfg(unix)]
    Command::new("make").check()?;
    #[cfg(windows)]
    Command::new("mingw32-make.exe").check()?;
    Ok(())
}

fn version() -> Result<String> {
    let metadata = MetadataCommand::new().exec()?;
    let Value::Object(workspace_metadata) = metadata.workspace_metadata else {
        bail!("No workspace metadata");
    };

    let Some(Value::String(version)) = workspace_metadata.get("version") else {
        bail!("No version in workspace metadata");
    };

    Ok(version.to_string())
}

fn build_id() -> Result<isize> {
    let metadata = MetadataCommand::new().exec()?;
    let Value::Object(workspace_metadata) = metadata.workspace_metadata else {
        bail!("No workspace metadata");
    };

    let Some(Value::Number(num)) = workspace_metadata.get("build-id") else {
        bail!("No build-id in workspace metadata");
    };

    num.as_i64()
        .map(|i| i as isize)
        .ok_or_else(|| anyhow!("build-id is not an integer"))
}

fn current_crate() -> Result<String> {
    let metadata = MetadataCommand::new().exec()?;
    Ok(metadata
        .workspace_root
        .file_name()
        .ok_or_else(|| anyhow!("No file name"))?
        .to_string())
}

fn recursive_directory_listing<P>(directory: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    WalkDir::new(directory.as_ref())
        .into_iter()
        .filter_map(|p| p.ok())
        .map(|p| p.path().to_path_buf())
        .filter(|p| p.is_file())
        .collect::<Vec<_>>()
}

#[derive(TypedBuilder, Debug)]
struct PackagesList {
    #[builder(setter(into))]
    dist: String,
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option, into))]
    package_number: Option<i32>,
    #[builder(default = false)]
    disabled: bool,
    #[builder(setter(into))]
    description: String,
    #[builder(default, setter(strip_option, into))]
    bin_encryption_key: Option<String>,
    #[builder(default, setter(into))]
    owners: Vec<String>,
    #[builder(default, setter(into))]
    access_labels: Vec<String>,
    #[builder(default, setter(into))]
    hosts: Vec<String>,
    #[builder(default, setter(into))]
    make: Vec<String>,
    #[builder(default, setter(strip_option, into))]
    doc_title: Option<String>,
    #[builder(default, setter(into))]
    refman_localfiles: Vec<String>,
    #[builder(default, setter(strip_option, into))]
    comment: Option<String>,
    #[builder(default = false)]
    include_refmanual: bool,
    #[builder(default = false)]
    include_release_notes: bool,
    #[builder(setter(into))]
    confidentiality: String,
    #[builder(default, setter(into))]
    ip_plans: Vec<String>,
    #[builder(default, setter(into))]
    data: Vec<String>,
    #[builder(default, setter(into))]
    groups: Vec<PackagesListGroup>,
}

impl ToString for PackagesList {
    fn to_string(&self) -> String {
        let mut list = format!("Dist: {}\nName: {}\n", self.dist, self.name);
        list += &self
            .package_number
            .map(|n| format!("Package-number: {}\n", n))
            .unwrap_or_default();
        list += self
            .disabled
            .then_some("Disabled: Yes\n")
            .unwrap_or_default();
        list += &format!("Description: {}\n", self.description);
        list += &self
            .bin_encryption_key
            .as_ref()
            .map(|b| format!("Bin-encryption-key: {}", b))
            .unwrap_or_default();
        list += &(!self.owners.is_empty())
            .then_some(format!("Owners: {}\n", self.owners.join(", ")))
            .unwrap_or_default();
        list += &(!self.access_labels.is_empty())
            .then_some(format!(
                "Access-labels: {}\n",
                self.access_labels.join(", ")
            ))
            .unwrap_or_default();
        list += &(!self.hosts.is_empty())
            .then_some(format!("Hosts: {}\n", self.hosts.join(" ")))
            .unwrap_or_default();
        list += &(!self.make.is_empty())
            .then_some(format!("Make: {}\n", self.make.join(", ")))
            .unwrap_or_default();
        list += &self
            .doc_title
            .as_ref()
            .map(|t| format!("Doc-title: {}\n", t))
            .unwrap_or_default();
        list += &(!self.refman_localfiles.is_empty())
            .then_some(format!(
                "Refman-localfiles: {}\n",
                self.refman_localfiles.join(", ")
            ))
            .unwrap_or_default();
        list += &self
            .comment
            .as_ref()
            .map(|c| format!("Comment: {}\n", c))
            .unwrap_or_default();
        list += self
            .include_refmanual
            .then_some("Include-refmanual: Yes\n")
            .unwrap_or_default();
        list += self
            .include_release_notes
            .then_some("Include-release-notes: Yes\n")
            .unwrap_or_default();
        list += &format!("Confidentiality: {}\n", self.confidentiality);
        list += &(!self.ip_plans.is_empty())
            .then_some(format!("IP-plans: {}\n", self.ip_plans.join(", ")))
            .unwrap_or_default();
        list += &self
            .data
            .iter()
            .map(|d| format!("    {}", d))
            .collect::<Vec<_>>()
            .join("\n");
        list += "\n\n";
        list += &self
            .groups
            .iter()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        list
    }
}

#[derive(TypedBuilder, Debug)]
struct PackagesListGroup {
    #[builder(setter(into))]
    group: String,
    #[builder(default, setter(into))]
    hosts: Vec<String>,
    #[builder(default, setter(into))]
    make: Vec<String>,
    #[builder(default, setter(into))]
    doc_make: Vec<String>,
    #[builder(default, setter(into))]
    doc_formats: Vec<String>,
    #[builder(default, setter(into))]
    require_tokens: Vec<String>,
    #[builder(default, setter(strip_option, into))]
    directory: Option<String>,
    #[builder(default, setter(into))]
    data: Vec<String>,
}

impl ToString for PackagesListGroup {
    fn to_string(&self) -> String {
        let mut group = format!("Group: {}\n", self.group);
        group += &(!self.hosts.is_empty())
            .then_some(format!("Hosts: {}\n", self.hosts.join(" ")))
            .unwrap_or_default();
        group += &(!self.make.is_empty())
            .then_some(format!("Make: {}\n", self.make.join(", ")))
            .unwrap_or_default();
        group += &(!self.doc_make.is_empty())
            .then_some(format!("Doc-make: {}\n", self.doc_make.join(", ")))
            .unwrap_or_default();
        group += &(!self.doc_formats.is_empty())
            .then_some(format!("Doc-formats: {}\n", self.doc_formats.join(", ")))
            .unwrap_or_default();
        group += &(!self.require_tokens.is_empty())
            .then_some(format!(
                "Require-tokens: {}\n",
                self.require_tokens.join(", ")
            ))
            .unwrap_or_default();
        group += &self
            .directory
            .as_ref()
            .map(|d| format!("Directory: {}\n", d))
            .unwrap_or_default();
        group += &self
            .data
            .iter()
            .map(|d| format!("    {}", d))
            .collect::<Vec<_>>()
            .join("\n");
        group
    }
}

impl PackagesListGroup {
    pub fn group_ref(&self) -> String {
        format!("@{}", self.group)
    }
}

fn generate_packages_list<P>(directory: P) -> Result<PackagesList>
where
    P: AsRef<Path>,
{
    // Src has no dependencies
    let src_group = PackagesListGroup::builder()
        .group("src")
        .data(
            recursive_directory_listing(
                directory.as_ref().join("modules").join("tsffs").join("src"),
            )
            .iter()
            .filter_map(|p| {
                p.strip_prefix(directory.as_ref())
                    .map(|p| p.to_path_buf())
                    .ok()
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        )
        .build();

    // The module depends on the module src (of course)
    let tsffs_group = PackagesListGroup::builder()
        .group("tsffs")
        .make(["tsffs".to_string()])
        .data([
            "$(HOST)/lib/tsffs$(SO)".to_string(),
            src_group.group_ref(),
            "modules/tsffs/Makefile".to_string(),
        ])
        .build();

    // The interface src is generated by the tsffs build process
    let tsffs_interface_src_group = PackagesListGroup::builder()
        .group("tsffs-interface-src")
        .make(["tsffs".to_string()])
        .data([
            "modules/tsffs-interface/Makefile".to_string(),
            "modules/tsffs-interface/tsffs-interface.dml".to_string(),
            "modules/tsffs-interface/tsffs-interface.h".to_string(),
        ])
        .build();

    // The interface depends on the interface src
    let tsffs_interface_group = PackagesListGroup::builder()
        .group("tsffs-interface")
        .make(["tsffs-interface".to_string()])
        .data([
            "$(HOST)/lib/tsffs-interface$(SO)".to_string(),
            tsffs_interface_src_group.group_ref(),
        ])
        .build();

    let tl_data = vec![tsffs_group.group_ref(), tsffs_interface_group.group_ref()];

    let groups = vec![
        src_group,
        tsffs_group,
        tsffs_interface_src_group,
        tsffs_interface_group,
    ];

    let packages_list = PackagesList::builder()
        .dist("TSFFS")
        .name("TSFFS Fuzzer")
        .package_number(31337)
        .owners(["rhart".to_string()])
        .access_labels(["external-intel".to_string()])
        .hosts(["linux64".to_string(), "win64".to_string()])
        .doc_title("TSFFS Fuzzer")
        .comment("TSFFS: Target Software Fuzzer For SIMICS")
        .description("TSFFS: Target Software Fuzzer For SIMICS")
        .confidentiality("Public")
        .data(tl_data)
        .groups(groups)
        .build();

    Ok(packages_list)
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    #[serde(rename = "package-name")]
    package_name: String,
    #[serde(rename = "package-name-full")]
    package_name_full: String,
    #[serde(rename = "package-number")]
    package_number: Option<isize>,
    name: String,
    description: String,
    host: String,
    version: String,
    #[serde(rename = "build-id")]
    build_id: isize,
    #[serde(rename = "build-id-namespace")]
    build_id_namespace: String,
    confidentiality: String,
    files: HashMap<String, String>,
    #[serde(rename = "type")]
    // NOTE: Either "addon" or "base" -- convert to enum
    typ: String,
    disabled: bool,
    #[serde(rename = "doc-title")]
    doc_title: Option<String>,
    #[serde(rename = "make-targets")]
    make_targets: Vec<String>,
    #[serde(rename = "include-release-notes")]
    include_release_notes: bool,
    #[serde(rename = "ip-plans")]
    ip_plans: Vec<String>,
    #[serde(rename = "legacy-doc-make-targets")]
    legacy_doc_make_targets: Vec<String>,
    #[serde(rename = "release-notes")]
    release_notes: Vec<String>,
    #[serde(rename = "access-labels")]
    access_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpecs(Vec<PackageSpec>);

impl TryFrom<PackagesList> for PackageSpecs {
    type Error = Error;

    fn try_from(value: PackagesList) -> Result<Self> {
        let version = version()?;
        let build_id_namespace = current_crate()?;
        let build_id = build_id()?;
        let files = value
            .groups
            .iter()
            .map(|g| g.data.clone())
            .flatten()
            .collect::<Vec<_>>();
        let make_targets = value
            .groups
            .iter()
            .map(|g| g.make.clone())
            .flatten()
            .collect::<Vec<_>>();
        Ok(Self(
            value
                .hosts
                .iter()
                .map(|h| {
                    let package_name_full = format!("{}-{}", value.dist, h);
                    let files = files
                        .iter()
                        .map(|f| {
                            f.replace("$(HOST)", h)
                                .replace("$(SO)", if h == "linux64" { ".so" } else { ".dll" })
                        })
                        .filter_map(|f| match PathBuf::from(".").join(&f).canonicalize() {
                            Ok(fc) => Some((f.clone(), fc.to_string_lossy().to_string())),
                            Err(_) => None,
                        })
                        .chain(once((
                            format!("{}/lib/{}.modcache", h, package_name_full),
                            PathBuf::from(".")
                                .canonicalize()
                                .unwrap_or_else(|_| panic!("Failed to canonicalize modcache path"))
                                .join(format!("{}/lib/{}.modcache", h, package_name_full))
                                .to_string_lossy()
                                .to_string(),
                        )))
                        .collect::<HashMap<_, _>>();
                    PackageSpec {
                        name: value.name.clone(),
                        package_name: value.dist.clone(),
                        package_name_full: package_name_full,
                        package_number: value.package_number.map(|pn| pn as isize),
                        disabled: value.disabled,
                        description: value.description.clone(),
                        host: h.clone(),
                        version: version.clone(),
                        build_id: build_id.clone(),
                        build_id_namespace: build_id_namespace.clone(),
                        confidentiality: value.confidentiality.clone(),
                        files: files.clone(),
                        typ: "addon".to_string(),
                        doc_title: value.doc_title.clone(),
                        make_targets: make_targets.clone(),
                        include_release_notes: value.include_release_notes,
                        ip_plans: value.ip_plans.clone(),
                        legacy_doc_make_targets: vec![],
                        release_notes: vec![],
                        access_labels: value.access_labels.clone(),
                    }
                })
                .collect(),
        ))
    }
}

fn create_package_specs(packages_list: PackagesList) -> Result<()> {
    // NOTE: On systems with doc-and-packaging available, this will work, but since end users
    // do not have access to this package, we do this ourselves.
    // Command::new(directory.as_ref().join("bin").join("create-package-specs"))
    //     .arg("-o")
    //     .arg("linux64/package-specs.json")
    //     .arg("config/dist")
    //     .check()?;
    let package_spec: PackageSpecs = packages_list.try_into()?;
    #[cfg(unix)]
    {
        create_dir_all("linux64")?;
        write(
            &PathBuf::from("linux64/package-specs.json"),
            to_string(&package_spec)?.as_bytes(),
        )?;
    }
    #[cfg(windows)]
    {
        create_dir_all("win64")?;
        write(
            &PathBuf::from("win64/package-specs.json"),
            to_string(&package_spec)?.as_bytes(),
        )?;
    }
    Ok(())
}

fn create_modcache<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    #[cfg(unix)]
    Command::new(directory.as_ref().join("bin").join("create-modcache"))
        .arg("-p")
        .arg("linux64/package-specs.json")
        .check()?;

    #[cfg(windows)]
    Command::new(directory.as_ref().join("bin").join("create-modcache.bat"))
        .arg("-p")
        .arg("win64/package-specs.json")
        .check()?;
    Ok(())
}

fn create_packages<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    #[cfg(unix)]
    Command::new(directory.as_ref().join("bin").join("create-packages"))
        .arg("--package-specs")
        .arg("linux64/package-specs.json")
        .arg("-o")
        .arg("linux64/packages")
        .arg("31337")
        .check()?;
    #[cfg(windows)]
    Command::new(directory.as_ref().join("bin").join("create-packages.bat"))
        .arg("--package-specs")
        .arg("win64/package-specs.json")
        .arg("-o")
        .arg("win64/packages")
        .arg("31337")
        .check()?;
    Ok(())
}

fn main() -> Result<()> {
    let current_dir = current_dir()?;

    if !current_dir.join("build.rs").is_file() {
        bail!("build.rs must be run from the package root like ./build.rs");
    }

    make()?;
    make()?;
    let packages_list_path = current_dir
        .join("config")
        .join("dist")
        .join("packages.list");
    let packages_list = generate_packages_list(&current_dir)?;
    write(&packages_list_path, packages_list.to_string())?;
    create_package_specs(packages_list)?;
    create_modcache(&current_dir)?;
    create_packages(&current_dir)?;

    Ok(())
}
