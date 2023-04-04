//! Confuse Simics Project
//!
//! This crate provides tools for managing simics projects, including linking to simics, loading
//! modules, and creating and destroying temporary project directories

pub mod link;
pub mod module;
mod util;
pub mod yml;

use anyhow::{bail, ensure, Context, Result};
use confuse_simics_manifest::{package_infos, simics_base_version, PackageNumber};
use dotenvy_macro::dotenv;
use log::{error, info};
use module::SimicsModule;
use std::{
    collections::HashSet,
    fs::{copy, create_dir_all, remove_dir_all, OpenOptions},
    io::Write,
    os::unix::fs::symlink,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};
use tempdir::TempDir;
use util::copy_dir_contents;
use version_tools::VersionConstraint;
use versions::Versioning;
/// The SIMICS home installation directory. A `.env` file containing a line like:
/// SIMICS_HOME=/home/username/simics/ must be present in the workspace tree
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
/// Prefix for naming temporary directories
const SIMICS_PROJECT_PREFIX: &str = "simics_project";

/// Return the SIMICS_HOME directory as a PathBuf. This depends on the SIMICS_HOME environment
/// variable being defined at compile time, and runtime changes to this variable will have no
/// effect.
pub fn simics_home() -> Result<PathBuf> {
    let simics_home = PathBuf::from(SIMICS_HOME);
    match simics_home.exists() {
        true => Ok(simics_home),
        false => {
            bail!(
                "SIMICS_HOME is defined, but {} does not exist.",
                SIMICS_HOME
            )
        }
    }
}

/// Structure for managing simics projects on disk, including the packages added to the project
/// and the modules loaded in it.
pub struct SimicsProject {
    pub base_path: PathBuf,
    pub home: PathBuf,
    packages: HashSet<PackageNumber>,
    modules: HashSet<SimicsModule>,
    tmp: bool,
}

impl SimicsProject {
    /// Try to create a new temporary simics project. If a project is created this way, it is
    /// removed from disk when this object is dropped. Creates the project using the newest
    /// Simics-Base package it finds in SIMICS_HOME
    pub fn try_new_latest() -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path, "*")?;
        project.tmp = true;
        Ok(project)
    }

    /// Try to create a new temporary simics project, with a particular simics base version.
    pub fn try_new<S: AsRef<str>>(base_version_constraint: S) -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path, base_version_constraint)?;
        project.tmp = true;
        Ok(project)
    }

    /// Create a simics project at a specific path with a specific base version. When a project is
    /// created this way, it is not deleted when it is dropped and will instead persist on disk.
    pub fn try_new_at<P: AsRef<Path>, S: AsRef<str>>(
        base_path: P,
        base_version_constraint: S,
    ) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let base_path = base_path.canonicalize()?;
        if !base_path.exists() {
            create_dir_all(&base_path)?;
        }
        let simics_home = PathBuf::from(SIMICS_HOME).canonicalize()?;
        let simics_manifest = simics_base_version(&simics_home, &base_version_constraint)?;
        let simics_base_dir = simics_home.join(format!("simics-{}", simics_manifest.version));

        let simics_base_project_setup = simics_base_dir.join("bin").join("project-setup");

        Command::new(simics_base_project_setup)
            .arg("--ignore-existing-files")
            .arg(&base_path)
            .current_dir(&base_path)
            .output()?;

        Ok(Self {
            base_path,
            home: simics_home,
            packages: HashSet::new(),
            modules: HashSet::new(),
            tmp: false,
        })
    }

    /// Retrieve the arguments for loading all the modules that are added to the project. The order
    /// is arbitrary, so if there is an ordering dependency you should specify these arguments
    /// manually
    pub fn module_load_args(&self) -> Vec<String> {
        // self.modules
        //     .iter()
        //     .flat_map(|sm| ["-e".to_string(), format!("load-module {}", sm.name)])
        //     .collect()
        // TODO
        vec![]
    }

    /// Build this project, including any modules, and return the simics executable for this project
    /// as a command ready to run with arguments
    pub fn build(&self) -> Result<Command> {
        for module in &self.modules {
            module.install(&self.base_path)?;
        }

        let res = Command::new("make")
            .current_dir(&self.base_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        ensure!(
            res.status.success(),
            "Failed to build project!\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&res.stdout),
            String::from_utf8_lossy(&res.stderr)
        );

        Ok(Command::new(self.base_path.join("simics")))
    }

    /// Make this project persistent (ie it will not be deleted when dropped)
    pub fn persist(&mut self) {
        self.tmp = false;
    }
}

/// Builder functions for SimicsProject
impl SimicsProject {
    /// Try to add a shared object module to the simics project. This module may or may not already
    /// be signed using `sign_simics_module` but will be re-signed in all cases. This will fail if
    /// the module does not correctly include the symbols needed for simics to load it.
    pub fn try_with_module<S: AsRef<str>>(mut self, module_crate_name: S) -> Result<Self> {
        let module = SimicsModule::try_new(module_crate_name)?;
        self.modules.insert(module);
        Ok(self)
    }

    /// Try to add a package to this project by number. If multiple versions are available, pick
    /// the latest version
    pub fn try_with_package_latest<P: Into<PackageNumber>>(self, package: P) -> Result<Self> {
        self.try_with_package_version(package, "*")
    }

    /// Try to add a package to this project by number and optional version constraint
    pub fn try_with_package_version<S: AsRef<str>, P: Into<PackageNumber>>(
        mut self,
        package: P,
        version_constraint: S,
    ) -> Result<Self> {
        let package = package.into();
        if self.packages.contains(&package) {
            return Ok(self);
        }

        // let constraint = VersionReq::parse(version_constraint.as_ref())?;
        let constraint: VersionConstraint = version_constraint.as_ref().parse()?;

        let package_infos = package_infos(&self.home)?;

        let package_infos = package_infos.get(&package).with_context(|| {
            error!("Package {:?} not be found in package info", package);
            "Package does not exist"
        })?;

        let version = package_infos
            .keys()
            .filter_map(|k| Versioning::new(k))
            .filter(|v| constraint.matches(v))
            .max()
            .context("No matching version")?;

        let simics_package_list_path = self.base_path.join(".package-list");

        let package_info = package_infos
            .get(&version.to_string())
            .context("No such version")?;

        let package_path = package_info
            .get_package_path(&self.home)?
            .to_string_lossy()
            .to_string();

        let simics_package_list = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(simics_package_list_path)?;

        writeln!(&simics_package_list, "{}", package_path)?;

        let simics_project_project_setup = self.base_path.join("bin").join("project-setup");

        info!("Running {:?}", simics_project_project_setup);

        Command::new(&simics_project_project_setup)
            .current_dir(&self.base_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        self.packages.insert(package);

        Ok(self)
    }

    /// Copy the contents from the base directory `src_dir` into the simics project directory,
    /// overwriting any files that already exist but not replacing any directories. For example,
    /// if `src_dir` looked like:
    ///
    /// ```text
    /// resource$ ls -hR
    /// .:
    /// simics-scripts  targets
    ///
    /// ./simics-scripts:
    /// blueprint  qsp-x86-uefi-app.py  qsp-x86-uefi-app.yml
    ///
    /// ./simics-scripts/blueprint:
    /// testme.yml  uefi-app-blueprint.include
    ///
    /// ./targets:
    /// images  qsp-x86-uefi-app.yml  run-uefi-app.simics
    ///
    /// ./targets/images:
    /// minimal_boot_disk.craff  run_uefi_app.nsh
    /// ```
    ///
    /// Then this function would copy the simics-scripts directory and the targets directory
    /// into the simics project root, recursively
    pub fn try_with_contents<P: AsRef<Path>>(self, src_dir: P) -> Result<Self> {
        let src_dir = src_dir.as_ref().to_path_buf();
        copy_dir_contents(&src_dir, &self.base_path)?;
        Ok(self)
    }

    /// Add a file into the simics project at a path relative to the project directory. For example
    ///
    /// ```text
    /// project.try_with_file(PathBuf::from("/tmp/some_file", "modules/mod.so"))
    /// ```
    ///
    /// This would copy /tmp/some_file into the simics project in the modules directory as mod.so
    pub fn try_with_file<P: AsRef<Path>, S: AsRef<str>>(
        self,
        src_path: P,
        dst_relative_path: S,
    ) -> Result<Self> {
        // It's not 100% coverage but sanity check against dumb path traversals here
        ensure!(
            !PathBuf::from_str(dst_relative_path.as_ref())?
                .components()
                .any(|c| c == Component::ParentDir),
            "Path must be relative to the project directory and contain no parent directories!"
        );
        let dst_path = self.base_path.join(dst_relative_path.as_ref());
        let dst_path_dir = dst_path
            .parent()
            .context("Destination path has no parent.")?;

        create_dir_all(dst_path_dir)?;

        copy(src_path, &dst_path)?;

        Ok(self)
    }

    /// Add a file into the simics project at a path relative to the project directory.
    pub fn try_with_file_contents<S: AsRef<str>>(
        self,
        contents: &[u8],
        dst_relative_path: S,
    ) -> Result<Self> {
        // It's not 100% coverage but sanity check against dumb path traversals here
        ensure!(
            !PathBuf::from_str(dst_relative_path.as_ref())?
                .components()
                .any(|c| c == Component::ParentDir),
            "Path must be relative to the project directory and contain no parent directories!"
        );
        let dst_path = self.base_path.join(dst_relative_path.as_ref());
        let dst_path_dir = dst_path
            .parent()
            .context("Destination path has no parent.")?;

        create_dir_all(dst_path_dir)?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&dst_path)?;

        file.write_all(contents)?;

        Ok(self)
    }

    /// Symlink a file into the simics project at a path relative to the project directory.
    ///
    /// This is useful when a very large file needs to be available in the project but you
    /// don't necessarily want to copy or move it.
    pub fn try_with_file_symlink<P: AsRef<Path>, S: AsRef<str>>(
        self,
        src_path: P,
        dst_relative_path: S,
    ) -> Result<Self> {
        ensure!(
            src_path.as_ref().is_file(),
            "Path {} does not exist or is not a file",
            src_path.as_ref().display()
        );
        let dst_path = self.base_path.join(dst_relative_path.as_ref());
        symlink(src_path, dst_path)?;
        Ok(self)
    }
}

impl Drop for SimicsProject {
    /// Remove the simics project from disk if it was created with an automatic project directory,
    /// does nothing otherwise.
    fn drop(&mut self) {
        if self.tmp {
            remove_dir_all(&self.base_path).ok();
        }
    }
}
