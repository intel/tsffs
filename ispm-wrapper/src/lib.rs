// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Wrappers for the small subset of ISPM commands the fuzzer and its build processes need to
//!
//! To implement or update this subset using public SIMICS, install ISPM (Intel SIMICS
//! Package Manager) to `~/simics-public/ispm/`, then:
//!
//! ```sh,ignore
//! npx asar -h
//! npx
//! npx asar extract ~/simics-public/ispm/resources/app.asar \
//!     ~/simics-public/ispm/resources/app.asar.extracted
//! npx webcrack ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.js \
//!     > ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.unmin.js
//! npx deobfuscator ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.js
//! ```

#[allow(deprecated)]
// NOTE: Use of deprecated home_dir is ok because the "incorrect" windows behavior is actually
// correct for SIMICS' use case.
use std::env::home_dir;

use std::{fs::read, path::PathBuf, process::Command};

use anyhow::{anyhow, Result};
use command_ext::CommandExtCheck;
use derive_getters::Getters;
use serde::Deserialize;
use serde_json::from_slice;
use version_tools::from_string;
use versions::Versioning;

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path object that is optionally an internet URI or local filesystem path
pub struct IPathObject {
    id: isize,
    priority: isize,
    value: String,
    enabled: bool,
    #[serde(rename = "isWritable")]
    writable: Option<bool>,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RepoPath {
    value: String,
    enabled: bool,
    priority: isize,
    id: isize,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Rectangle {
    x: isize,
    y: isize,
    width: isize,
    height: isize,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxySettingTypes {
    None,
    Env,
    Manual,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum InstallationPreference {
    RepoOrder,
    LegacyStyle,
    NewStyle,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
/// V3 ISPM configuration, all fields are optional so older configs that we support should also work
/// without an issue
pub struct Settings {
    /// Package repositories that ISPM can install from. Managed by the `ispm config
    /// package-repos` command.
    archives: Option<Vec<RepoPath>>,
    #[serde(rename = "cacheTimeout")]
    cache_timeout: Option<isize>,
    #[serde(rename = "installPath")]
    /// Installation path. Managed by the `ispm config install-dir` command.
    install_path: Option<IPathObject>,
    #[serde(rename = "readOnlyInstallationPaths")]
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
    projects_default: Option<String>,
    #[serde(rename = "enableRemoteManifests")]
    enable_remote_manifests: Option<bool>,
    #[serde(rename = "manifestRepos")]
    /// Platform repositories that ISPM can install from. Managed by the `ispm config
    /// platform-repos` command.
    manifest_repos: Option<Vec<IPathObject>>,
    #[serde(rename = "projects")]
    projects: Option<Vec<IPathObject>>,
    #[serde(rename = "manifests")]
    manifests: Option<Vec<IPathObject>>,
    #[serde(rename = "keyStore")]
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
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy
    /// --https` command and flags.
    https_proxy: Option<String>,
    #[serde(rename = "httpProxy")]
    /// Proxy settings that should be used for HTTPS. Managed by the `ispm config proxy`
    /// command.
    http_proxy: Option<String>,
    #[serde(rename = "noProxy")]
    /// URL/IP list (comma-delimited) of resources that should not use a proxy for access.
    no_proxy: Option<String>,
    #[serde(rename = "AuthenticationConfig")]
    /// The path to the SIMICS authentication file. Not used for public release. Managed by the
    /// `ispm config auth-file` command.
    authentication_config: Option<String>,
    #[serde(rename = "logfile")]
    logfile: Option<String>,
    #[serde(rename = "preferPackageType")]
    prefer_package_type: Option<InstallationPreference>,
}

impl Settings {
    pub fn get() -> Result<Self> {
        Ok(from_slice(&read(Internal::cfg_file_path()?)?)?)
    }
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InstalledPackage {
    #[serde(rename = "pkgNumber")]
    package_number: isize,
    #[serde(deserialize_with = "from_string")]
    version: Versioning,
    name: String,
    paths: Vec<PathBuf>,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AvailablePackage {
    #[serde(rename = "pkgNumber")]
    package_number: isize,
    #[serde(deserialize_with = "from_string")]
    version: Versioning,
    name: String,
    installed: bool,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Packages {
    #[serde(rename = "installedPackages")]
    installed_packages: Option<Vec<InstalledPackage>>,
    #[serde(rename = "availablePackages")]
    available_packages: Option<Vec<AvailablePackage>>,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProjectPackage {
    #[serde(rename = "pkgNumber")]
    package_number: isize,
    #[serde(deserialize_with = "from_string")]
    version: Versioning,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Project {
    name: String,
    description: String,
    path: PathBuf,
    packages: Vec<ProjectPackage>,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Projects {
    projects: Vec<Project>,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Platform {
    name: String,
    group: String,
    path: String,
    remote: bool,
}

#[derive(Getters, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Platforms {
    platforms: Vec<Platform>,
}

/// Minimal implementation of internal ISPM functionality to use it externally
pub struct Internal {}

impl Internal {
    // NOTE: Can be found in package.json in extracted ispm application
    const PRODUCT_NAME: &str = "Intel Simics Package Manager";

    // NOTE: Can be found in `AppInfo` class in extracted ispm application
    const CFG_FILENAME: &str = "simics-package-manager.cfg";

    // NOTE: Can be found in `constructAppDataPath` in extracted ispm application
    /// Retrieve the path to the directory containing ISPM's application data, in particular the
    /// configuration file.
    fn app_data_path() -> Result<PathBuf> {
        #[allow(deprecated)]
        // NOTE: Use of deprecated home_dir is ok because the "incorrect" windows behavior is actually
        // correct for SIMICS' use case.
        let home_dir = home_dir().ok_or_else(|| anyhow!("No home directory found"))?;

        #[cfg(unix)]
        return Ok(home_dir.join(".config").join(Self::PRODUCT_NAME));

        #[cfg(windows)]
        return Ok(home_dir
            .join("AppData")
            .join("Local")
            .join(Self::PRODUCT_NAME));
    }

    // NOTE: Can be found in `getCfgFileName` in extracted ispm application
    /// Retrieve the path to the ISPM configuration file
    pub fn cfg_file_path() -> Result<PathBuf> {
        Ok(Self::app_data_path()?.join(Self::CFG_FILENAME))
    }
}

/// Wrappers for ISPM commands
pub struct Ispm {}

impl Ispm {
    /// Get the current ISPM configuration
    pub fn settings() -> Result<Settings> {
        from_slice(
            &Command::new("ispm")
                .arg("settings")
                .arg("--json")
                .check()?
                .stdout,
        )
        .or_else(|_| {
            // Fall back to reading the config from disk
            Settings::get()
        })
    }

    /// Get the currently installed and available packages
    pub fn packages() -> Result<Packages> {
        Ok(from_slice(
            &Command::new("ispm")
                .arg("packages")
                // NOTE: There is a bug happening when running e.g.:
                // `ispm packages --list --json | cat > test.txt; stat -c '%s' test.txt`
                // where the output to the pipe from ISPM stops after the size of the
                // PIPE_BUF. For now, we mitigate this by passing `--list-installed` only.
                .arg("--list-installed")
                .arg("--json")
                .check()?
                .stdout,
        )?)
    }

    /// Get existing projects
    pub fn projects() -> Result<Projects> {
        Ok(from_slice(
            &Command::new("ispm")
                .arg("projects")
                // NOTE: There is a bug happening when running e.g.:
                // `ispm packages --list --json | cat > test.txt; stat -c '%s' test.txt`
                // where the output to the pipe from ISPM stops after the size of the
                // PIPE_BUF. For now, we mitigate this by passing `--list-installed` only.
                .arg("--list")
                .arg("--json")
                .check()?
                .stdout,
        )?)
    }

    /// Get existing platforms
    pub fn platforms() -> Result<Platforms> {
        Ok(from_slice(
            &Command::new("ispm")
                .arg("platforms")
                // NOTE: There is a bug happening when running e.g.:
                // `ispm packages --list --json | cat > test.txt; stat -c '%s' test.txt`
                // where the output to the pipe from ISPM stops after the size of the
                // PIPE_BUF. For now, we mitigate this by passing `--list-installed` only.
                .arg("--list")
                .arg("--json")
                .check()?
                .stdout,
        )?)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use std::path::PathBuf;

    use crate::{IPathObject, Ispm, ProxySettingTypes, RepoPath, Settings};
    use serde_json::from_str;

    #[test]
    fn test_simple_public() {
        let expected: Settings = Settings {
            archives: Some(vec![RepoPath {
                value: "https://artifactory.example.com/artifactory/repos/example/".to_string(),
                enabled: true,
                priority: 0,
                id: 0,
            }]),
            cache_timeout: None,
            install_path: Some(IPathObject {
                id: 1,
                priority: 0,
                value: "/home/user/simics".to_string(),
                enabled: true,
                writable: Some(true),
            }),
            read_only_installation_paths: None,
            cfg_version: Some(2),
            gui_bounds: None,
            gui_maximized: None,
            powershell_path: None,
            temp_directory: Some(PathBuf::from("/home/user/tmp")),
            multi_user: None,
            projects_default: None,
            enable_remote_manifests: None,
            manifest_repos: Some(vec![
                IPathObject {
                    id: 0,
                    priority: 0,
                    value: "https://x.y.example.com".to_string(),
                    enabled: true,
                    writable: Some(false),
                },
                IPathObject {
                    id: 1,
                    priority: 1,
                    value: "https://artifactory.example.com/artifactory/repos/example/".to_string(),
                    enabled: true,
                    writable: None,
                },
            ]),
            projects: Some(vec![IPathObject {
                id: 0,
                priority: 0,
                value: "/home/user/simics-projects/qsp-x86-project".to_string(),
                enabled: true,
                writable: None,
            }]),
            manifests: None,
            key_store: Some(vec![IPathObject {
                id: 0,
                priority: 0,
                value: "/home/user/simics/keys".to_string(),
                enabled: true,
                writable: None,
            }]),
            ignore_legacy_platform_repo_deprecation: None,
            proxy_settings_to_use: Some(ProxySettingTypes::Env),
            https_proxy: None,
            http_proxy: None,
            no_proxy: None,
            authentication_config: None,
            logfile: None,
            prefer_package_type: None,
        };
        const SETTINGS_TEST_SIMPLE_PUBLIC: &str =
            include_str!("../tests/config/simple-public/simics-package-manager.cfg");

        let settings: Settings = from_str(SETTINGS_TEST_SIMPLE_PUBLIC)
            .unwrap_or_else(|e| panic!("Error loading simple configuration: {e}"));

        assert_eq!(settings, expected)
    }

    #[test]
    fn test_current() -> Result<()> {
        Ispm::settings()?;
        Ok(())
    }

    #[test]
    fn test_packages() -> Result<()> {
        Ispm::packages()?;
        Ok(())
    }

    // #[test]
    // fn test_platforms() -> Result<()> {
    //     Ispm::platforms()?;
    //     Ok(())
    // }
}
