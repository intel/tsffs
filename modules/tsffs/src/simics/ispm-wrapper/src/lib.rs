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
use std::env::home_dir;
// NOTE: Use of deprecated home_dir is ok because the "incorrect" windows behavior is actually
// correct for SIMICS' use case.
use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub mod data;

pub const ISPM_NAME: &str = "ispm";
pub const NON_INTERACTIVE_FLAG: &str = "--non-interactive";
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

pub trait ToArgs {
    fn to_args(&self) -> Vec<String>;
}

/// Wrappers for ISPM commands
pub mod ispm {
    use std::{iter::repeat, path::PathBuf};

    use derive_getters::Getters;
    use typed_builder::TypedBuilder;

    use crate::{ToArgs, NON_INTERACTIVE_FLAG};

    #[derive(TypedBuilder, Getters, Clone, Debug)]
    pub struct GlobalOptions {
        #[builder(default, setter(into))]
        package_repo: Vec<String>,
        #[builder(default, setter(into, strip_option))]
        install_dir: Option<PathBuf>,
        #[builder(default, setter(into, strip_option))]
        https_proxy: Option<String>,
        #[builder(default, setter(into, strip_option))]
        no_proxy: Option<String>,
        #[builder(default = true)]
        non_interactive: bool,
        #[builder(default = false)]
        trust_insecure_packages: bool,
        #[builder(default, setter(into, strip_option))]
        config_file: Option<PathBuf>,
        #[builder(default = false)]
        no_config_file: bool,
        #[builder(default, setter(into, strip_option))]
        temp_dir: Option<PathBuf>,
        #[builder(default, setter(into, strip_option))]
        auth_file: Option<PathBuf>,
    }

    impl ToArgs for GlobalOptions {
        fn to_args(&self) -> Vec<String> {
            let mut args = Vec::new();

            args.extend(
                repeat("--package-repo".to_string())
                    .zip(self.package_repo.iter())
                    .flat_map(|(flag, arg)| [flag, arg.to_string()]),
            );
            args.extend(self.install_dir.as_ref().iter().flat_map(|id| {
                [
                    "--install-dir".to_string(),
                    id.to_string_lossy().to_string(),
                ]
            }));
            args.extend(
                self.https_proxy
                    .as_ref()
                    .iter()
                    .flat_map(|p| ["--https-proxy".to_string(), p.to_string()]),
            );
            args.extend(
                self.no_proxy
                    .as_ref()
                    .iter()
                    .flat_map(|p| ["--no-proxy".to_string(), p.to_string()]),
            );
            if self.non_interactive {
                args.push(NON_INTERACTIVE_FLAG.to_string())
            }
            if self.trust_insecure_packages {
                args.push("--trust-insecure-packages".to_string())
            }
            args.extend(self.config_file.as_ref().iter().flat_map(|cf| {
                [
                    "--config-file".to_string(),
                    cf.to_string_lossy().to_string(),
                ]
            }));
            if self.no_config_file {
                args.push("--no-config-file".to_string());
            }
            args.extend(
                self.temp_dir
                    .as_ref()
                    .iter()
                    .flat_map(|td| ["--temp-dir".to_string(), td.to_string_lossy().to_string()]),
            );
            args.extend(
                self.auth_file
                    .as_ref()
                    .iter()
                    .flat_map(|af| ["--auth-file".to_string(), af.to_string_lossy().to_string()]),
            );

            args
        }
    }

    impl Default for GlobalOptions {
        fn default() -> Self {
            Self::builder().build()
        }
    }

    pub mod packages {
        use crate::{
            data::{Packages, ProjectPackage},
            ToArgs, ISPM_NAME, NON_INTERACTIVE_FLAG,
        };
        use anyhow::Result;
        use command_ext::CommandExtCheck;
        use derive_getters::Getters;
        use serde_json::from_slice;
        use std::{iter::repeat, path::PathBuf, process::Command};
        use typed_builder::TypedBuilder;

        use super::GlobalOptions;

        const PACKAGES_SUBCOMMAND: &str = "packages";

        /// Get the currently installed and available packages
        pub fn list(options: &GlobalOptions) -> Result<Packages> {
            Ok(from_slice(
                &Command::new(ISPM_NAME)
                    .arg(PACKAGES_SUBCOMMAND)
                    .arg(NON_INTERACTIVE_FLAG)
                    // NOTE: There is a bug happening when running e.g.:
                    // `ispm packages --list --json | cat > test.txt; stat -c '%s' test.txt`
                    // where the output to the pipe from ISPM stops after the size of the
                    // PIPE_BUF. For now, we mitigate this by passing `--list-installed` only.
                    .arg("--list-installed")
                    .arg("--json")
                    .args(options.to_args())
                    .check()?
                    .stdout,
            )?)
        }

        #[derive(TypedBuilder, Getters, Clone, Debug)]
        pub struct InstallOptions {
            #[builder(default, setter(into))]
            /// Packages to install by number/version
            packages: Vec<ProjectPackage>,
            #[builder(default, setter(into))]
            /// Packages to install by local path
            package_paths: Vec<PathBuf>,
            #[builder(default)]
            global: GlobalOptions,
            #[builder(default = false)]
            install_all: bool,
        }

        impl ToArgs for InstallOptions {
            fn to_args(&self) -> Vec<String> {
                repeat("-i".to_string())
                    .zip(
                        self.packages.iter().map(|p| p.to_string()).chain(
                            self.package_paths
                                .iter()
                                .map(|p| p.to_string_lossy().to_string()),
                        ),
                    )
                    .flat_map(|(flag, arg)| [flag, arg])
                    .chain(self.global.to_args().iter().cloned())
                    .chain(self.install_all.then_some("--install-all".to_string()))
                    .collect::<Vec<_>>()
            }
        }

        pub fn install(install_options: &InstallOptions) -> Result<()> {
            Command::new(ISPM_NAME)
                .arg(PACKAGES_SUBCOMMAND)
                .args(install_options.to_args())
                .arg(NON_INTERACTIVE_FLAG)
                .check()?;
            Ok(())
        }

        #[derive(TypedBuilder, Getters, Clone, Debug)]
        pub struct UninstallOptions {
            #[builder(default, setter(into))]
            /// Packages to install by number/version
            packages: Vec<ProjectPackage>,
            #[builder(default)]
            global: GlobalOptions,
        }

        impl ToArgs for UninstallOptions {
            fn to_args(&self) -> Vec<String> {
                repeat("-u".to_string())
                    .zip(self.packages.iter().map(|p| p.to_string()))
                    .flat_map(|(flag, arg)| [flag, arg])
                    .chain(self.global.to_args().iter().cloned())
                    .collect::<Vec<_>>()
            }
        }

        pub fn uninstall(uninstall_options: &UninstallOptions) -> Result<()> {
            Command::new(ISPM_NAME)
                .arg(PACKAGES_SUBCOMMAND)
                .args(uninstall_options.to_args())
                .arg(NON_INTERACTIVE_FLAG)
                .check()?;
            Ok(())
        }
    }

    pub mod projects {
        use crate::{
            data::{ProjectPackage, Projects},
            ToArgs, ISPM_NAME, NON_INTERACTIVE_FLAG,
        };
        use anyhow::{anyhow, Result};
        use command_ext::CommandExtCheck;
        use derive_getters::Getters;
        use serde_json::from_slice;
        use std::{collections::HashSet, iter::once, path::Path, process::Command};
        use typed_builder::TypedBuilder;

        use super::GlobalOptions;

        const IGNORE_EXISTING_FILES_FLAG: &str = "--ignore-existing-files";
        const CREATE_PROJECT_FLAG: &str = "--create";
        const PROJECTS_SUBCOMMAND: &str = "projects";

        #[derive(TypedBuilder, Getters, Clone, Debug)]
        pub struct CreateOptions {
            #[builder(default, setter(into))]
            packages: HashSet<ProjectPackage>,
            #[builder(default = false)]
            ignore_existing_files: bool,
            #[builder(default)]
            global: GlobalOptions,
        }

        impl ToArgs for CreateOptions {
            fn to_args(&self) -> Vec<String> {
                self.packages()
                    .iter()
                    .map(|p| Some(p.to_string()))
                    .chain(once(
                        self.ignore_existing_files
                            .then_some(IGNORE_EXISTING_FILES_FLAG.to_string()),
                    ))
                    .flatten()
                    .chain(self.global.to_args().iter().cloned())
                    .collect::<Vec<_>>()
            }
        }

        /// Create a project
        pub fn create<P>(create_options: &CreateOptions, project_path: P) -> Result<()>
        where
            P: AsRef<Path>,
        {
            let mut args = vec![
                PROJECTS_SUBCOMMAND.to_string(),
                project_path
                    .as_ref()
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert to string"))?
                    .to_string(),
                CREATE_PROJECT_FLAG.to_string(),
            ];
            args.extend(create_options.to_args());
            println!("{:?}", args);
            Command::new(ISPM_NAME).args(args).check()?;

            Ok(())
        }

        /// Get existing projects
        pub fn list(options: &GlobalOptions) -> Result<Projects> {
            Ok(from_slice(
                &Command::new(ISPM_NAME)
                    .arg(PROJECTS_SUBCOMMAND)
                    .arg(NON_INTERACTIVE_FLAG)
                    // NOTE: There is a bug happening when running e.g.:
                    // `ispm packages --list --json | cat > test.txt; stat -c '%s' test.txt`
                    // where the output to the pipe from ISPM stops after the size of the
                    // PIPE_BUF. For now, we mitigate this by passing `--list-installed` only.
                    .arg("--list")
                    .arg("--json")
                    .args(options.to_args())
                    .check()?
                    .stdout,
            )?)
        }
    }

    pub mod platforms {
        use crate::{data::Platforms, ISPM_NAME, NON_INTERACTIVE_FLAG};
        use anyhow::Result;
        use command_ext::CommandExtCheck;
        use serde_json::from_slice;
        use std::process::Command;

        const PLATFORMS_SUBCOMMAND: &str = "platforms";

        /// Get existing platforms
        pub fn list() -> Result<Platforms> {
            Ok(from_slice(
                &Command::new(ISPM_NAME)
                    .arg(PLATFORMS_SUBCOMMAND)
                    .arg(NON_INTERACTIVE_FLAG)
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

    pub mod settings {
        use crate::{data::Settings, ISPM_NAME, NON_INTERACTIVE_FLAG};
        use anyhow::Result;
        use command_ext::CommandExtCheck;
        use serde_json::from_slice;
        use std::process::Command;

        const SETTINGS_SUBCOMMAND: &str = "settings";

        /// Get the current ISPM configuration
        pub fn list() -> Result<Settings> {
            from_slice(
                &Command::new(ISPM_NAME)
                    .arg(SETTINGS_SUBCOMMAND)
                    .arg(NON_INTERACTIVE_FLAG)
                    .arg("--json")
                    .check()?
                    .stdout,
            )
            .or_else(|_| {
                // Fall back to reading the config from disk
                Settings::get()
            })
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use std::path::PathBuf;

    use crate::{
        data::{IPathObject, ProxySettingTypes, RepoPath, Settings},
        ispm::{self, GlobalOptions},
    };
    use serde_json::from_str;

    #[test]
    fn test_simple_public() {
        let expected = Settings::builder()
            .archives([RepoPath::builder()
                .value("https://artifactory.example.com/artifactory/repos/example/")
                .enabled(true)
                .priority(0)
                .id(0)
                .build()])
            .install_path(
                IPathObject::builder()
                    .id(1)
                    .priority(0)
                    .value("/home/user/simics")
                    .enabled(true)
                    .writable(true)
                    .build(),
            )
            .cfg_version(2)
            .temp_directory(PathBuf::from("/home/user/tmp"))
            .manifest_repos([
                IPathObject::builder()
                    .id(0)
                    .priority(0)
                    .value("https://x.y.example.com")
                    .enabled(true)
                    .writable(false)
                    .build(),
                IPathObject::builder()
                    .id(1)
                    .priority(1)
                    .value("https://artifactory.example.com/artifactory/repos/example/")
                    .enabled(true)
                    .build(),
            ])
            .projects([IPathObject::builder()
                .id(0)
                .priority(0)
                .value("/home/user/simics-projects/qsp-x86-project")
                .enabled(true)
                .build()])
            .key_store([IPathObject::builder()
                .id(0)
                .priority(0)
                .value("/home/user/simics/keys")
                .enabled(true)
                .build()])
            .proxy_settings_to_use(ProxySettingTypes::Env)
            .build();
        const SETTINGS_TEST_SIMPLE_PUBLIC: &str =
            include_str!("../tests/config/simple-public/simics-package-manager.cfg");

        let settings: Settings = from_str(SETTINGS_TEST_SIMPLE_PUBLIC)
            .unwrap_or_else(|e| panic!("Error loading simple configuration: {e}"));

        assert_eq!(settings, expected)
    }

    #[test]
    fn test_current() -> Result<()> {
        ispm::settings::list()?;
        Ok(())
    }

    #[test]
    fn test_packages() -> Result<()> {
        ispm::packages::list(&GlobalOptions::default())?;
        Ok(())
    }
}
