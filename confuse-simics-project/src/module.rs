//! Utilities for managing simics modules, specifically adding them to a project

use anyhow::{ensure, Result};
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use crate::util::{copy_dir_contents, find_crate_dir, find_static_library};

#[derive(Eq, Hash, PartialEq)]
pub struct SimicsModule {
    crate_dir: PathBuf,
    static_library: PathBuf,
    project_base_path: PathBuf,
}

impl SimicsModule {
    /// Container for adding a module crate to a simics project.
    ///
    /// The crate for the module should be set up like so:
    /// - CRATE_NAME
    ///     - modules
    ///         - CRATE_NAME
    ///         - CRATE_NAME-interface (optional)
    ///     - src
    ///     - Cargo.toml
    ///
    /// The module C sources should have Makefiles that will link to the static library
    /// produced by building the crate, which will be named libCRATE_NAME.a
    pub fn try_new<S: AsRef<str>, P: AsRef<Path>>(
        crate_name: S,
        simics_project_base: P,
    ) -> Result<Self> {
        let crate_dir = find_crate_dir(crate_name.as_ref())?;
        ensure!(
            crate_dir.is_dir(),
            "No such directory: {}",
            crate_dir.display()
        );

        let static_library = find_static_library(crate_name.as_ref())?;
        ensure!(
            static_library.is_file(),
            "No such file: {}",
            static_library.display()
        );

        let modules_dir = crate_dir.join("modules");
        ensure!(
            modules_dir.is_dir(),
            "No modules in crate {}",
            crate_dir.display()
        );

        let project_base_path = simics_project_base.as_ref().to_path_buf();
        ensure!(
            project_base_path.is_dir(),
            "SIMICS project does not exist at {}",
            project_base_path.display()
        );

        let project_modules_path = project_base_path.join("modules");

        if !project_modules_path.is_dir() {
            create_dir_all(&project_modules_path)?;
        }

        copy_dir_contents(modules_dir, project_modules_path)?;

        Ok(Self {
            crate_dir,
            static_library,
            project_base_path,
        })
    }
}
