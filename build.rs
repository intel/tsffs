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
//! ```

use anyhow::{bail, Result};
use command_ext::CommandExtCheck;
use std::{
    env::current_dir,
    fs::write,
    path::{Path, PathBuf},
    process::Command,
};
use typed_builder::TypedBuilder;
use walkdir::WalkDir;

fn make() -> Result<()> {
    Command::new("make").check()?;
    Ok(())
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
            .then_some(format!("Owners: {}\n", self.owners.join(",")))
            .unwrap_or_default();
        list += &(!self.access_labels.is_empty())
            .then_some(format!("Access-labels: {}\n", self.access_labels.join(",")))
            .unwrap_or_default();
        list += &(!self.hosts.is_empty())
            .then_some(format!("Hosts: {}\n", self.hosts.join(",")))
            .unwrap_or_default();
        list += &(!self.make.is_empty())
            .then_some(format!("Make: {}\n", self.make.join(",")))
            .unwrap_or_default();
        list += &self
            .doc_title
            .as_ref()
            .map(|t| format!("Doc-title: {}\n", t))
            .unwrap_or_default();
        list += &(!self.refman_localfiles.is_empty())
            .then_some(format!(
                "Refman-localfiles: {}\n",
                self.refman_localfiles.join(",")
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
            .then_some(format!("IP-plans: {}\n", self.ip_plans.join(",")))
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
            .then_some(format!("Hosts: {}\n", self.hosts.join(",")))
            .unwrap_or_default();
        group += &(!self.make.is_empty())
            .then_some(format!("Make: {}\n", self.make.join(",")))
            .unwrap_or_default();
        group += &(!self.doc_make.is_empty())
            .then_some(format!("Doc-make: {}\n", self.doc_make.join(",")))
            .unwrap_or_default();
        group += &(!self.doc_formats.is_empty())
            .then_some(format!("Doc-formats: {}\n", self.doc_formats.join(",")))
            .unwrap_or_default();
        group += &(!self.require_tokens.is_empty())
            .then_some(format!(
                "Require-tokens: {}\n",
                self.require_tokens.join(",")
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

fn generate_packages_list<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let packages_list_path = directory
        .as_ref()
        .join("config")
        .join("dist")
        .join("packages.list");

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
        .hosts(["linux64".to_string()])
        .doc_title("TSFFS Fuzzer")
        .comment("TSFFS: Target Software Fuzzer For SIMICS")
        .description("TSFFS: Target Software Fuzzer For SIMICS")
        .confidentiality("Public")
        .data(tl_data)
        .groups(groups)
        .build();

    write(&packages_list_path, packages_list.to_string())?;

    Ok(())
}

fn create_package_specs<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    Command::new(directory.as_ref().join("bin").join("create-package-specs"))
        .arg("-o")
        .arg("linux64/package-specs.json")
        .arg("config/dist")
        .check()?;
    Ok(())
}

fn create_modcache<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    Command::new(directory.as_ref().join("bin").join("create-modcache"))
        .arg("-p")
        .arg("linux64/package-specs.json")
        .check()?;
    Ok(())
}

fn create_packages<P>(directory: P) -> Result<()>
where
    P: AsRef<Path>,
{
    Command::new(directory.as_ref().join("bin").join("create-packages"))
        .arg("--package-specs")
        .arg("linux64/package-specs.json")
        .arg("-o")
        .arg("linux64/packages")
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
    generate_packages_list(&current_dir)?;
    create_package_specs(&current_dir)?;
    create_modcache(&current_dir)?;
    create_packages(&current_dir)?;

    Ok(())
}
