// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Data deserializable from ISPM commands and configurations

use anyhow::Result;
use getters2::Getters;
use serde::Deserialize;
use serde_json::from_slice;
use std::{fmt::Display, fs::read, path::PathBuf};
use typed_builder::TypedBuilder;
use version_tools::{version_constraint_from_string, VersionConstraint};

use crate::Internal;

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path object that is optionally an internet URI or local filesystem path
pub struct IPathObject {
    id: isize,
    priority: isize,
    #[builder(setter(into))]
    value: String,
    enabled: bool,
    #[serde(rename = "isWritable")]
    #[builder(default, setter(strip_option))]
    writable: Option<bool>,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path to a SIMICS repo. This is an artifactory repository.
pub struct RepoPath {
    #[builder(setter(into))]
    value: String,
    enabled: bool,
    priority: isize,
    id: isize,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(setter(into)))]
/// An electron rectangle definition
pub struct Rectangle {
    x: isize,
    y: isize,
    width: isize,
    height: isize,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Proxy settings
pub enum ProxySettingTypes {
    None,
    Env,
    Manual,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
/// Preference for which method should be used to install packages
pub enum InstallationPreference {
    RepoOrder,
    LegacyStyle,
    NewStyle,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(strip_option)))]
/// V3 ISPM configuration, all fields are optional so older configs that we support should also work
/// without an issue
pub struct Settings {
    #[builder(setter(into))]
    /// Package repositories that ISPM can install from. Managed by the `ispm config
    /// package-repos` command.
    archives: Option<Vec<RepoPath>>,
    #[serde(rename = "cacheTimeout")]
    cache_timeout: Option<isize>,
    #[serde(rename = "installPath")]
    /// Installation path. Managed by the `ispm config install-dir` command.
    install_path: Option<IPathObject>,
    #[serde(rename = "readOnlyInstallationPaths")]
    #[builder(setter(into))]
    /// Installation paths that are set as read-only. Managed by the `ispm config
    /// ro-install-paths` command.
    read_only_installation_paths: Option<Vec<IPathObject>>,
    #[serde(rename = "cfgVersion")]
    cfg_version: Option<isize>,
    #[serde(rename = "guiBounds")]
    gui_bounds: Option<Rectangle>,
    #[serde(rename = "guiMaximized")]
    gui_maximized: Option<bool>,
    #[serde(rename = "powershellPath")]
    powershell_path: Option<PathBuf>,
    #[serde(rename = "tempDirectory")]
    /// The temporary directory used by ISPM. Managed by the `ispm config temp-dir` command.
    temp_directory: Option<PathBuf>,
    #[serde(rename = "multiUser")]
    multi_user: Option<bool>,
    #[serde(rename = "projectsDefault")]
    #[builder(setter(into))]
    projects_default: Option<String>,
    #[serde(rename = "enableRemoteManifests")]
    enable_remote_manifests: Option<bool>,
    #[serde(rename = "manifestRepos")]
    #[builder(setter(into))]
    /// Platform repositories that ISPM can install from. Managed by the `ispm config
    /// platform-repos` command.
    manifest_repos: Option<Vec<IPathObject>>,
    #[serde(rename = "projects")]
    #[builder(setter(into))]
    projects: Option<Vec<IPathObject>>,
    #[serde(rename = "manifests")]
    #[builder(setter(into))]
    manifests: Option<Vec<IPathObject>>,
    #[serde(rename = "keyStore")]
    #[builder(setter(into))]
    /// Files that store decryption keys for legacy package installation. Managed by the
    /// `ispm config decryption-key-files` command.
    key_store: Option<Vec<IPathObject>>,
    #[serde(rename = "ignoreLegacyPlatformRepoDeprecation")]
    ignore_legacy_platform_repo_deprecation: Option<bool>,
    #[serde(rename = "proxySettingsToUse")]
    /// Proxy settings that should be used. Managed by the `ispm config proxy
    /// (--dont-use|--use-env)` command and flags.
    proxy_settings_to_use: Option<ProxySettingTypes>,
    #[serde(rename = "httpsProxy")]
    #[builder(setter(into))]
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy
    /// --https` command and flags.
    https_proxy: Option<String>,
    #[serde(rename = "httpProxy")]
    #[builder(setter(into))]
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy`
    /// command.
    http_proxy: Option<String>,
    #[serde(rename = "noProxy")]
    #[builder(setter(into))]
    /// URL/IP list (comma-delimited) of resources that should not use a proxy for access.
    no_proxy: Option<String>,
    #[serde(rename = "AuthenticationConfig")]
    #[builder(setter(into))]
    /// The path to the SIMICS authentication file. Not used for public release. Managed by the
    /// `ispm config auth-file` command.
    authentication_config: Option<String>,
    #[serde(rename = "logfile")]
    #[builder(setter(into))]
    logfile: Option<String>,
    #[serde(rename = "preferPackageType")]
    prefer_package_type: Option<InstallationPreference>,
}

impl Settings {
    /// Get the current settings from the currently set configuration file
    pub fn get() -> Result<Self> {
        Ok(from_slice(&read(Internal::cfg_file_path()?)?)?)
    }
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A package that is already installed
pub struct InstalledPackage {
    #[serde(rename = "pkgNumber")]
    #[getters(deref)]
    /// The package number
    package_number: isize,
    #[serde(deserialize_with = "version_constraint_from_string")]
    #[getters(clone)]
    /// The package version
    version: VersionConstraint,
    #[builder(setter(into))]
    #[getters(clone)]
    /// The package name
    name: String,
    #[builder(default, setter(into))]
    #[getters(clone)]
    /// Paths to this installed package
    paths: Vec<PathBuf>,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A package that can be installed
pub struct AvailablePackage {
    #[serde(rename = "pkgNumber")]
    /// The package number
    package_number: isize,
    #[serde(deserialize_with = "version_constraint_from_string")]
    /// The package version
    version: VersionConstraint,
    #[builder(setter(into))]
    /// The package name
    name: String,
    /// Whether this package is installed
    installed: bool,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(strip_option, into)))]
/// Set of installed and available packages
pub struct Packages {
    #[serde(rename = "installedPackages")]
    /// The list of packages which are installed
    installed_packages: Option<Vec<InstalledPackage>>,
    #[serde(rename = "availablePackages")]
    /// The list of packages which are available to install
    available_packages: Option<Vec<AvailablePackage>>,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
/// A package which is added to a project
pub struct ProjectPackage {
    #[serde(rename = "pkgNumber")]
    #[getters(deref)]
    /// The package number
    package_number: isize,
    #[serde(deserialize_with = "version_constraint_from_string")]
    #[builder(setter(into))]
    /// The package version
    version: VersionConstraint,
}

impl Display for ProjectPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}",
            self.package_number,
            self.version
                .version()
                .map(|v| v.to_string())
                .unwrap_or_default()
        )
    }
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A SIMICS project
pub struct Project {
    #[builder(setter(into))]
    /// The project name
    name: String,
    #[builder(setter(into))]
    /// The project description
    description: String,
    /// The path to the project
    path: PathBuf,
    #[builder(default, setter(into))]
    /// The set of packages this project was configured with
    packages: Vec<ProjectPackage>,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(into)))]
/// List of known projects associated with this ISPM installation
pub struct Projects {
    projects: Vec<Project>,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A platform, which is a collection of packages
pub struct Platform {
    #[builder(setter(into))]
    /// The name of the platform
    name: String,
    #[builder(setter(into))]
    /// The group of the platform
    group: String,
    #[builder(setter(into))]
    /// The path to the platform
    path: String,
    /// Whether this platform is remote
    remote: bool,
}

#[derive(TypedBuilder, Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(into)))]
/// A list of platforms
pub struct Platforms {
    /// The list of platforms
    platforms: Vec<Platform>,
}
