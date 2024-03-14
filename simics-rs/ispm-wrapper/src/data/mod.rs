// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Data deserializable from ISPM commands and configurations

use anyhow::Result;
use serde::Deserialize;
use serde_json::from_slice;
use std::{fmt::Display, fs::read, path::PathBuf};
use typed_builder::TypedBuilder;
use versions::Versioning;

use crate::Internal;

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path object that is optionally an internet URI or local filesystem path
pub struct IPathObject {
    /// The unique id of the path
    pub id: isize,
    /// The priority of the path
    pub priority: isize,
    #[builder(setter(into))]
    /// The path
    pub value: String,
    /// whether this path is enabled
    pub enabled: bool,
    #[serde(rename = "isWritable")]
    #[builder(default, setter(strip_option))]
    /// Whether this path is writable
    pub writable: Option<bool>,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path to a SIMICS repo. This is an artifactory repository.
pub struct RepoPath {
    #[builder(setter(into))]
    /// The path
    pub value: String,
    /// Whether this path is enabled
    pub enabled: bool,
    /// The priority of the path
    pub priority: isize,
    /// The unique id of the path
    pub id: isize,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(setter(into)))]
/// An electron rectangle definition
pub struct Rectangle {
    /// The x value of the rectangle's coordinate
    pub x: isize,
    /// The y value of the rectangle's coordinate
    pub y: isize,
    /// The width
    pub width: isize,
    /// The height
    pub height: isize,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Proxy settings
pub enum ProxySettingTypes {
    /// No proxy should be used
    None,
    /// Use the proxy settings from environment variables
    Env,
    /// Use the proxy settings from the manual configuration
    Manual,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
/// Preference for which method should be used to install packages
pub enum InstallationPreference {
    /// Install in the order in the repository
    RepoOrder,
    /// Install in legacy style
    LegacyStyle,
    /// Install in the new style
    NewStyle,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(strip_option)))]
/// V3 ISPM configuration, all fields are optional so older configs that we support should also work
/// without an issue
pub struct Settings {
    #[builder(setter(into))]
    /// Package repositories that ISPM can install from. Managed by the `ispm config
    /// package-repos` command.
    pub archives: Option<Vec<RepoPath>>,
    #[serde(rename = "cacheTimeout")]
    /// The timeout of the cache
    pub cache_timeout: Option<isize>,
    #[serde(rename = "installPath")]
    /// Installation path. Managed by the `ispm config install-dir` command.
    pub install_path: Option<IPathObject>,
    #[serde(rename = "readOnlyInstallationPaths")]
    #[builder(setter(into))]
    /// Installation paths that are set as read-only. Managed by the `ispm config
    /// ro-install-paths` command.
    pub read_only_installation_paths: Option<Vec<IPathObject>>,
    #[serde(rename = "cfgVersion")]
    /// The configuration version number
    pub cfg_version: Option<isize>,
    #[serde(rename = "guiBounds")]
    /// Last saved bounds of the ISPM GUI
    pub gui_bounds: Option<Rectangle>,
    #[serde(rename = "guiMaximized")]
    /// Whether the GUI was maximized
    pub gui_maximized: Option<bool>,
    #[serde(rename = "powershellPath")]
    /// The path to the powershell executable
    pub powershell_path: Option<PathBuf>,
    #[serde(rename = "tempDirectory")]
    /// The temporary directory used by ISPM. Managed by the `ispm config temp-dir` command.
    pub temp_directory: Option<PathBuf>,
    #[serde(rename = "multiUser")]
    /// Whether this is a multi-user installation
    pub multi_user: Option<bool>,
    #[serde(rename = "projectsDefault")]
    #[builder(setter(into))]
    /// The default projects
    pub projects_default: Option<String>,
    #[serde(rename = "enableRemoteManifests")]
    /// Whether remtoe manifests are enabled
    pub enable_remote_manifests: Option<bool>,
    #[serde(rename = "manifestRepos")]
    #[builder(setter(into))]
    /// Platform repositories that ISPM can install from. Managed by the `ispm config
    /// platform-repos` command.
    pub manifest_repos: Option<Vec<IPathObject>>,
    #[serde(rename = "projects")]
    #[builder(setter(into))]
    /// A list of registered projects
    pub projects: Option<Vec<IPathObject>>,
    #[serde(rename = "manifests")]
    #[builder(setter(into))]
    /// A list of manifests
    pub manifests: Option<Vec<IPathObject>>,
    #[serde(rename = "keyStore")]
    #[builder(setter(into))]
    /// Files that store decryption keys for legacy package installation. Managed by the
    /// `ispm config decryption-key-files` command.
    pub key_store: Option<Vec<IPathObject>>,
    #[serde(rename = "ignoreLegacyPlatformRepoDeprecation")]
    /// Whether to ignore deprecation warnings for legacy platforms
    pub ignore_legacy_platform_repo_deprecation: Option<bool>,
    #[serde(rename = "proxySettingsToUse")]
    /// Proxy settings that should be used. Managed by the `ispm config proxy
    /// (--dont-use|--use-env)` command and flags.
    pub proxy_settings_to_use: Option<ProxySettingTypes>,
    #[serde(rename = "httpsProxy")]
    #[builder(setter(into))]
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy
    /// --https` command and flags.
    pub https_proxy: Option<String>,
    #[serde(rename = "httpProxy")]
    #[builder(setter(into))]
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy`
    /// command.
    pub http_proxy: Option<String>,
    #[serde(rename = "noProxy")]
    #[builder(setter(into))]
    /// URL/IP list (comma-delimited) of resources that should not use a proxy for access.
    pub no_proxy: Option<String>,
    #[serde(rename = "AuthenticationConfig")]
    #[builder(setter(into))]
    /// The path to the SIMICS authentication file. Not used for public release. Managed by the
    /// `ispm config auth-file` command.
    pub authentication_config: Option<String>,
    #[serde(rename = "logfile")]
    #[builder(setter(into))]
    /// The current logfile
    pub logfile: Option<String>,
    #[serde(rename = "preferPackageType")]
    /// The package type that is favored
    pub prefer_package_type: Option<InstallationPreference>,
}

impl Settings {
    /// Get the current settings from the currently set configuration file
    pub fn get() -> Result<Self> {
        Ok(from_slice(&read(Internal::cfg_file_path()?)?)?)
    }
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A package that is already installed
pub struct InstalledPackage {
    #[serde(rename = "pkgNumber")]
    /// The package number
    pub package_number: isize,
    /// The package version
    pub version: String,
    #[builder(setter(into))]
    /// The package name
    pub name: String,
    #[builder(default, setter(into))]
    /// Paths to this installed package
    pub paths: Vec<PathBuf>,
}

impl InstalledPackage {
    /// Get this package's version as a comparable version object
    pub fn version(&self) -> Versioning {
        Versioning::new(&self.version).expect("Failed to parse installed package version")
    }
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A package that can be installed
pub struct AvailablePackage {
    #[serde(rename = "pkgNumber")]
    /// The package number
    pub package_number: isize,
    /// The package version
    pub version: String,
    #[builder(setter(into))]
    /// The package name
    pub name: String,
    /// Whether this package is installed
    pub installed: bool,
}

impl AvailablePackage {
    /// Get this package's version as a comparable version object
    pub fn version(&self) -> Versioning {
        Versioning::new(&self.version).expect("Failed to parse available package version")
    }
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(strip_option, into)))]
/// Set of installed and available packages
pub struct Packages {
    #[serde(rename = "installedPackages")]
    /// The list of packages which are installed
    pub installed_packages: Option<Vec<InstalledPackage>>,
    #[serde(rename = "availablePackages")]
    /// The list of packages which are available to install
    pub available_packages: Option<Vec<AvailablePackage>>,
}

impl Packages {
    /// Sort the installed and available packages by their version number (highest first)
    pub fn sort(&mut self) {
        if let Some(installed_packages) = self.installed_packages.as_mut() {
            installed_packages.sort_by_key(|b| std::cmp::Reverse(b.version()))
        }
    }
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
/// A package which is added to a project
pub struct ProjectPackage {
    #[serde(rename = "pkgNumber")]
    /// The package number
    pub package_number: isize,
    #[builder(setter(into))]
    /// The package version
    pub version: String,
}

impl Display for ProjectPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.package_number, self.version)
    }
}

impl ProjectPackage {
    /// Get this package's version as a comparable version object
    pub fn version(&self) -> Versioning {
        Versioning::new(&self.version).expect("Failed to parse project package version")
    }
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A SIMICS project
pub struct Project {
    #[builder(setter(into))]
    /// The project name
    pub name: String,
    #[builder(setter(into))]
    /// The project description
    pub description: String,
    /// The path to the project
    pub path: PathBuf,
    #[builder(default, setter(into))]
    /// The set of packages this project was configured with
    pub packages: Vec<ProjectPackage>,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(into)))]
/// List of known projects associated with this ISPM installation
pub struct Projects {
    /// A list of known projects
    pub projects: Vec<Project>,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A platform, which is a collection of packages
pub struct Platform {
    #[builder(setter(into))]
    /// The name of the platform
    pub name: String,
    #[builder(setter(into))]
    /// The group of the platform
    pub group: String,
    #[builder(setter(into))]
    /// The path to the platform
    pub path: String,
    /// Whether this platform is remote
    pub remote: bool,
}

#[derive(TypedBuilder, Deserialize, Clone, Debug, PartialEq, Eq)]
#[builder(field_defaults(default, setter(into)))]
/// A list of platforms
pub struct Platforms {
    /// The list of platforms
    pub platforms: Vec<Platform>,
}
