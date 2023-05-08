//! Confuse Simics Project
//!
//! This crate provides tools for managing simics projects, including linking to simics, loading
//! modules, and creating and destroying temporary project directories, and actually running
//! the SIMICS process after configuration

pub mod link;
pub mod module;
pub mod util;

use anyhow::{bail, ensure, Context, Result};
use confuse_simics_manifest::{package_infos, simics_base_version, PackageNumber};
use dotenvy_macro::dotenv;
use log::{debug, error, info, Level};
use module::SimicsModule;
use rand::{distributions::Alphanumeric, Rng};
use std::{
    collections::{HashMap, HashSet},
    fs::{copy, create_dir_all, remove_dir_all, OpenOptions},
    io::Write,
    os::unix::fs::symlink,
    path::{Component, Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    str::FromStr,
    sync::Arc,
    thread::{spawn, JoinHandle},
};
use tempdir::TempDir;
use util::{abs_or_rel_base_relpath, copy_dir_contents};
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

/// A SIMICS command, this struct holds the arguments to a SIMICS command as configured with the
/// project builder API as well as its running state.
struct SimicsCommand {
    /// Whether SIMICS runs in batch mode. Defaults to `true`.
    pub batch_mode: bool,
    /// Configuration files. Defaults to no configuration files. Relative paths from project base
    pub configurations: Vec<String>,
    /// CLI Commands that will be executed in order they were added. Defaults to no commands.
    pub commands: Vec<String>,
    /// Whether to enable the GUI. Defaults to `false`.
    pub gui: bool,
    /// An optional license file path.
    pub license: Option<PathBuf>,
    /// Whether to open any windows. Defaults to `false`.
    pub win: bool,
    /// Whether to run in quiet mode. Defaults to `false`. You can set this to `true` if you
    /// know you're running bug-free and want a slight cleanup of initial logs.
    pub quiet: bool,
    /// Files to run Python code from.
    pub python_files: Vec<String>,
    /// Files to run additional scripts or configs from, for example `.yml` configurations
    pub files: Vec<String>,
    /// Directories to search for SIMICS modules in
    pub library_paths: Vec<String>,
    /// Whether the STC (Simulator Translation Cache) is enabled. Defaults to `true`.
    pub stc: bool,
    // Below here are non-simics settings that we use internally
    /// The path to the SIMICS executable (which is probably actually a symlink to the executable
    /// in SIMICS_HOME, but we don't need to account for that)
    pub simics: Option<PathBuf>,
    /// Environment variables to set for the SIMICS command
    pub env: HashMap<String, String>,
    /// The running simics process, if it has been started
    pub simics_process: Option<Child>,
    /// The closure or function used as a callback after the simics process starts to send
    /// input to its stdin. This should be rarely used for extenuating use cases where a
    /// simics script or python script is insufficient
    pub stdin_function: Option<Arc<dyn Fn(ChildStdin) + Send + Sync + 'static>>,
    /// The closure or function used as a callback after the simics process starts to
    /// receive output from the SIMICS stdout. If the output needs analysis or (more likely)
    /// should be directed somewhere for logging, this function should be used to do it.
    pub stdout_function: Option<Arc<dyn Fn(ChildStdout) + Send + Sync + 'static>>,
    /// The closure or function used as a callback after the simics process starts to
    /// receive output from the SIMICS stderr. If the output needs analysis or (more likely)
    /// should be directed somewhere for logging, this function should be used to do it.
    pub stderr_function: Option<Arc<dyn Fn(ChildStderr) + Send + Sync + 'static>>,
    /// The thread the `stdin_function` will run on
    pub stdin_thread: Option<JoinHandle<()>>,
    /// The thread the `stdout_function` will run on
    pub stdout_thread: Option<JoinHandle<()>>,
    /// The thread the `stderr_function` will run on
    pub stderr_thread: Option<JoinHandle<()>>,
}

impl SimicsCommand {
    /// Run the simics command from a simics project base path. The base path will be
    /// searched for the `simics` executable as well as be used for the relative directory
    /// containing various files, configurations, and scripts which can't be run with
    /// absolute paths.
    pub fn run<P: AsRef<Path>>(&mut self, base_path: P) -> Result<()> {
        let base_path = base_path.as_ref().to_path_buf();

        self.simics = Some(base_path.join("simics"));

        ensure!(
            self.simics.clone().context("No simics path")?.is_file(),
            "Simics executable does not exist at {}",
            self.simics.clone().context("No simics path")?.display()
        );

        let mut args = Vec::new();
        if self.batch_mode {
            args.push("-batch-mode".to_string());
        }

        for configuration in &self.configurations {
            args.push("-c".to_string());
            args.push(
                abs_or_rel_base_relpath(&base_path, configuration)?
                    .to_string_lossy()
                    .to_string(),
            );
        }

        if self.gui {
            args.push("-gui".to_string());
        } else {
            args.push("-no-gui".to_string());
        }

        if let Some(license) = &self.license {
            args.push("-l".to_string());
            args.push(license.to_string_lossy().to_string());
        }

        if !self.win {
            args.push("-no-win".to_string());
        }

        if self.quiet {
            args.push("-q".to_string());
        }

        for python_file in &self.python_files {
            args.push("-p".to_string());
            args.push(
                abs_or_rel_base_relpath(&base_path, python_file)?
                    .to_string_lossy()
                    .to_string(),
            );
        }

        for library_path in &self.library_paths {
            args.push("-L".to_string());
            args.push(
                abs_or_rel_base_relpath(&base_path, library_path)?
                    .to_string_lossy()
                    .to_string(),
            );
        }

        if self.stc {
            // These are defaults, so we do not set them
            // args.push("-istc".to_string());
            // args.push("-dstc".to_string());
        } else {
            args.push("-no-istc".to_string());
            args.push("-no-dstc".to_string());
        }

        for file in &self.files {
            args.push(
                abs_or_rel_base_relpath(&base_path, file)?
                    .to_string_lossy()
                    .to_string(),
            );
        }

        for command in &self.commands {
            args.push("-e".to_string());
            args.push(command.to_string());
        }

        let mut command = Command::new(self.simics.clone().context("No simics path")?);

        info!("Running SIMICS with args '{}'", args.join(" "));

        let mut simics_command = command
            .args(args)
            .envs(self.env.clone())
            .current_dir(&base_path);

        if self.stdout_function.is_some() {
            simics_command = simics_command.stdout(Stdio::piped());
        }

        if self.stderr_function.is_some() {
            simics_command = simics_command.stderr(Stdio::piped());
        }

        if self.stdin_function.is_some() {
            simics_command = simics_command.stdin(Stdio::piped());
        }

        let mut simics_process = simics_command.spawn()?;

        if let Some(stdout_function) = &self.stdout_function {
            let simics_stdout = simics_process.stdout.take().context("No child stdout")?;
            let function = stdout_function.clone();
            self.stdout_thread = Some(spawn(move || function(simics_stdout)));
        }

        if let Some(stdin_function) = &self.stdin_function {
            let simics_stdin = simics_process.stdin.take().context("No child stdin")?;
            let function = stdin_function.clone();
            self.stdin_thread = Some(spawn(move || function(simics_stdin)));
        }

        if let Some(stderr_function) = &self.stderr_function {
            let simics_stderr = simics_process.stderr.take().context("No child stdin")?;
            let function = stderr_function.clone();
            self.stderr_thread = Some(spawn(move || function(simics_stderr)));
        }

        self.simics_process = Some(simics_process);

        Ok(())
    }

    /// Forcibly kill the running SIMICS process and join out/input threads
    pub fn kill(&mut self) -> Result<()> {
        info!("Killing simics process");

        if let Some(ref mut simics_process) = self.simics_process {
            simics_process.kill()?;
            self.simics_process = None;
        }

        if let Some(r) = self.stdout_thread.take().map(JoinHandle::join) {
            r.map_err(|e| {
                error!("Error joining stdout thread: {:?}", e);
            })
            .ok();
        }

        if let Some(r) = self.stdin_thread.take().map(JoinHandle::join) {
            r.map_err(|e| {
                error!("Error joining stdin thread: {:?}", e);
            })
            .ok();
        }

        if let Some(r) = self.stderr_thread.take().map(JoinHandle::join) {
            r.map_err(|e| {
                error!("Error joining stderr thread: {:?}", e);
            })
            .ok();
        }

        Ok(())
    }

    pub fn try_clone(&self) -> Result<Self> {
        ensure!(
            self.simics_process.is_none()
                && self.stdin_thread.is_none()
                && self.stdout_thread.is_none()
                && self.stderr_thread.is_none(),
            "Cannot clone simics command after it has been run."
        );
        Ok(Self {
            batch_mode: self.batch_mode,
            configurations: self.configurations.clone(),
            commands: self.commands.clone(),
            gui: self.gui,
            license: self.license.clone(),
            win: self.win,
            quiet: self.quiet,
            python_files: self.python_files.clone(),
            files: self.files.clone(),
            library_paths: self.library_paths.clone(),
            stc: self.stc,
            simics: self.simics.clone(),
            env: self.env.clone(),
            simics_process: None,
            stdin_function: self.stdin_function.clone(),
            stdout_function: self.stdout_function.clone(),
            stderr_function: self.stderr_function.clone(),
            stdin_thread: None,
            stdout_thread: None,
            stderr_thread: None,
        })
    }
}

impl Default for SimicsCommand {
    /// Instantiate a default (empty) Simics Command configuration
    fn default() -> Self {
        Self {
            simics: None,
            batch_mode: true,
            configurations: vec![],
            commands: vec![],
            gui: false,
            license: None,
            win: false,
            quiet: false,
            python_files: vec![],
            files: vec![],
            library_paths: vec![],
            stc: true,
            env: HashMap::new(),
            stdin_function: None,
            stdin_thread: None,
            stdout_function: None,
            stdout_thread: None,
            stderr_function: None,
            stderr_thread: None,
            simics_process: None,
        }
    }
}

#[derive(Clone)]
enum Content {
    /// A directory whose contents will be copied wholesale into the project
    DirContents(PathBuf),
    /// A file pair (src, dst) that will be copied into a relative path in the project
    File((PathBuf, String)),
    /// A file contents that will be copied into a relative path in the project
    FileContents((Vec<u8>, String)),
    /// A path that will be symlinked into a relative path in the project
    PathSymlink((PathBuf, String)),
}

/// Structure for managing simics projects on disk, including the packages added to the project
/// and the modules loaded in it.
pub struct SimicsProject {
    pub base_path: PathBuf,
    base_version_constraint: String,
    pub home: PathBuf,
    // Mapping of package number to its package path on disk (in SIMICS_HOME)
    packages: HashMap<PackageNumber, String>,
    modules: HashSet<SimicsModule>,
    tmp: bool,
    command: SimicsCommand,
    pub loglevel: Level,
    built: bool,
    contents: Vec<Content>,
}

impl SimicsProject {
    /// Try to create a new simics project. If a project is created this way, it is
    /// removed from disk when this object is dropped. Creates the project using the newest
    /// Simics-Base package it finds in SIMICS_HOME
    pub fn try_new_latest() -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path, "*")?;
        project.tmp = false;
        Ok(project)
    }

    /// Try to create a new simics project, with a particular simics base version.
    /// A version constraint is any version in
    /// [versions](https://docs.rs/versions/latest/versions/) format for example
    /// `^6.0.150`.
    pub fn try_new<S: AsRef<str>>(base_version_constraint: S) -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path, base_version_constraint)?;
        project.tmp = false;
        Ok(project)
    }

    /// Create a simics project at a specific path with a specific base version. When a
    /// project is created this way, it is not deleted when it is dropped and will
    /// instead persist on disk.  A version constraint is any version in
    /// [versions](https://docs.rs/versions/latest/versions/) format for example
    /// `^6.0.150`.
    pub fn try_new_at<P: AsRef<Path>, S: AsRef<str>>(
        base_path: P,
        base_version_constraint: S,
    ) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let base_path = base_path.canonicalize()?;
        if !base_path.exists() {
            create_dir_all(&base_path)?;
        }

        info!("Created new simics project at {}", base_path.display());

        let home = PathBuf::from(SIMICS_HOME).canonicalize()?;

        Ok(Self {
            base_path,
            base_version_constraint: base_version_constraint.as_ref().to_string(),
            home,
            packages: HashMap::new(),
            modules: HashSet::new(),
            tmp: false,
            command: SimicsCommand::default(),
            loglevel: Level::Error,
            built: false,
            contents: Vec::new(),
        })
    }

    fn build_setup(&mut self) -> Result<()> {
        let simics_manifest = simics_base_version(&self.home, &self.base_version_constraint)?;
        let simics_base_dir = self
            .home
            .join(format!("simics-{}", simics_manifest.version));
        let simics_base_project_setup = simics_base_dir.join("bin").join("project-setup");

        info!("Installing simics packages to project");

        Command::new(simics_base_project_setup)
            .arg("--ignore-existing-files")
            .arg(&self.base_path)
            .current_dir(&self.base_path)
            .output()?;

        info!("Project setup complete");

        Ok(())
    }

    fn build_install_packages(&mut self) -> Result<()> {
        let simics_package_list_path = self.base_path.join(".package-list");
        let mut simics_package_list = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(simics_package_list_path)?;

        for package_path in self.packages.values() {
            writeln!(&simics_package_list, "{}", package_path)?;
        }

        simics_package_list.flush()?;

        let simics_project_project_setup = self.base_path.join("bin").join("project-setup");

        info!(
            "Running project setup command {:?}",
            simics_project_project_setup
        );

        Command::new(&simics_project_project_setup)
            .current_dir(&self.base_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        Ok(())
    }

    fn build_install_modules(&mut self) -> Result<()> {
        for module in &self.modules {
            info!(
                "Installing module {} to {}",
                module.crate_name,
                self.base_path.display()
            );
            module.install(&self.base_path)?;
        }

        info!("Building simics project at {}", self.base_path.display());

        let res = Command::new("make")
            .current_dir(&self.base_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        info!("Finished building simics project");

        ensure!(
            res.status.success(),
            "Failed to build project!\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&res.stdout),
            String::from_utf8_lossy(&res.stderr)
        );

        Ok(())
    }

    fn build_add_contents(&mut self) -> Result<()> {
        for content in &self.contents {
            match content {
                Content::DirContents(src_dir) => copy_dir_contents(&src_dir, &&self.base_path)?,
                Content::File((src_path, dst_relative_path)) => {
                    let dst_path = self.base_path.join(dst_relative_path);
                    let dst_path_dir = dst_path
                        .parent()
                        .context("Destination path has no parent.")?;

                    create_dir_all(dst_path_dir)?;

                    copy(src_path, &dst_path)?;
                }
                Content::FileContents((contents, dst_relative_path)) => {
                    let dst_path = self.base_path.join(dst_relative_path);
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

                    info!("Added contents to file {}", dst_relative_path);
                }
                Content::PathSymlink((src_path, dst_relative_path)) => {
                    let dst_path = self.base_path.join(dst_relative_path);
                    symlink(src_path, dst_path)?;
                }
            }
        }

        Ok(())
    }

    /// Build this project, including any modules.
    pub fn build(mut self) -> Result<Self> {
        self.build_setup()?;
        self.build_add_contents()?;
        self.build_install_packages()?;
        self.build_install_modules()?;

        self.built = true;

        Ok(self)
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Running simics");
        self.command.run(self.base_path.clone())?;
        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        info!("Killing simics");
        self.command.kill()
    }

    /// Check if a particular module is present
    pub fn has_module<S: AsRef<str>>(&self, crate_name: S) -> bool {
        self.modules
            .iter()
            .any(|m| m.crate_name == crate_name.as_ref())
    }

    /// Make this project persistent (ie it will not be deleted when dropped)
    pub fn persist(&mut self) {
        info!(
            "Persisting simics project at '{}'",
            self.base_path.display()
        );

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

    /// Try to add a package to this project by number and optional version constraint. A version
    /// constraint is any version in [versions](https://docs.rs/versions/latest/versions/) format
    /// for example `^6.0.150`.
    pub fn try_with_package_version<S: AsRef<str>, P: Into<PackageNumber>>(
        mut self,
        package: P,
        version_constraint: S,
    ) -> Result<Self> {
        let package = package.into();

        info!("Adding package {}", package);

        if self.packages.contains_key(&package) {
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

        let package_info = package_infos
            .get(&version.to_string())
            .context("No such version")?;

        let package_path = package_info
            .get_package_path(&self.home)?
            .to_string_lossy()
            .to_string();

        self.packages.insert(package, package_path);

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
    pub fn try_with_contents<P: AsRef<Path>>(mut self, src_dir: P) -> Result<Self> {
        let src_dir = src_dir.as_ref().to_path_buf();
        ensure!(src_dir.is_dir(), "Source directory is not a directory");
        self.contents.push(Content::DirContents(src_dir));
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
        mut self,
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
        self.contents.push(Content::File((
            src_path.as_ref().to_path_buf(),
            dst_relative_path.as_ref().to_string(),
        )));

        Ok(self)
    }

    /// Add a file into the simics project at a path relative to the project directory.
    pub fn try_with_file_contents<S: AsRef<str>>(
        mut self,
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

        self.contents.push(Content::FileContents((
            contents.to_vec(),
            dst_relative_path.as_ref().to_string(),
        )));

        Ok(self)
    }

    /// Symlink a file into the simics project at a path relative to the project directory.
    ///
    /// This is useful when a very large file needs to be available in the project but you
    /// don't necessarily want to copy or move it.
    pub fn try_with_file_symlink<P: AsRef<Path>, S: AsRef<str>>(
        mut self,
        src_path: P,
        dst_relative_path: S,
    ) -> Result<Self> {
        ensure!(
            src_path.as_ref().is_file(),
            "Path {} does not exist or is not a file",
            src_path.as_ref().display()
        );
        self.contents.push(Content::PathSymlink((
            src_path.as_ref().to_path_buf(),
            dst_relative_path.as_ref().to_string(),
        )));
        Ok(self)
    }

    /// Set the command to run in batch mode once it is invoked
    pub fn with_batch_mode(mut self, mode: bool) -> Self {
        self.command.batch_mode = mode;
        self
    }

    /// Add a simics configuration file to pass to the simics command. This file path can
    /// either be absolute (begin with a `/`) or relative (begin with `./` or any other character).
    /// This is equivalent to the `-c` flag.
    ///
    /// The file must exist. If you expect this file to be created by a `try_with` method on this
    /// project, be sure to call that method *before* this one.
    pub fn try_with_configuration<S: AsRef<str>>(mut self, configuration: S) -> Result<Self> {
        self.command
            .configurations
            .push(configuration.as_ref().to_string());
        Ok(self)
    }

    /// Add a command to execute by passing it to the simics command. This is equivalent to
    /// the `-e` flag.
    pub fn with_command<S: AsRef<str>>(mut self, command: S) -> Self {
        self.command.commands.push(command.as_ref().to_string());
        self
    }

    /// Set whether to show the GUI when SIMICS runs
    pub fn with_gui(mut self, gui: bool) -> Self {
        self.command.gui = gui;
        self
    }

    /// Set a different license file than the default (no license). This is probably not necessary.
    pub fn with_license(mut self, license: PathBuf) -> Result<Self> {
        ensure!(
            license.is_file(),
            "License at {} does not exist",
            license.display()
        );
        self.command.license = Some(license);
        Ok(self)
    }

    /// Set whether to open any windows. Defaults to false, this is probably not necessary.
    pub fn with_win(mut self, win: bool) -> Self {
        self.command.win = win;
        self
    }

    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.command.quiet = quiet;
        self
    }

    /// Add a python file to pass to the simics command to execute. This file path can
    /// either be absolute (begin with a `/`) or relative (begin with `./` or any other character).
    /// This is equivalent to the `-p` flag.
    ///
    /// The file must exist. If you expect this file to be created by a `try_with` method on this
    /// project, be sure to call that method *before* this one.
    pub fn try_with_python_file<S: AsRef<str>>(mut self, python_file: S) -> Result<Self> {
        self.command
            .python_files
            .push(python_file.as_ref().to_string());
        Ok(self)
    }

    /// Add a file path to pass to the simics command to execute. This file path can
    /// either be absolute (begin with a `/`) or relative (begin with `./` or any other character).
    /// This is equivalent to passing this path as an additional positional argument
    ///
    /// The file must exist. If you expect this file to be created by a `try_with` method on this
    /// project, be sure to call that method *before* this one.
    pub fn try_with_file_argument<S: AsRef<str>>(mut self, file: S) -> Result<Self> {
        self.command.files.push(file.as_ref().to_string());
        Ok(self)
    }

    /// Add a library path to pass to the simics command to execute to search for
    /// modules. This path can either be absolute or relative. This is equivalent to the
    /// -L flag.
    ///
    ///
    /// The directory must exist. If you expect this file to be created by a `try_with` method on this
    /// project, be sure to call that method *before* this one.
    pub fn try_with_library_path<S: AsRef<str>>(mut self, library_path: S) -> Result<Self> {
        self.command
            .library_paths
            .push(library_path.as_ref().to_string());
        Ok(self)
    }

    /// Set whether the STC (Simulator Translation Cache) is enabled. This is equivalent to the
    /// `-istc` and `-dstc` flags.
    pub fn with_stc(mut self, stc: bool) -> Self {
        self.command.stc = stc;
        self
    }

    /// Add an environment variable to the simics project command.
    pub fn with_env<S: AsRef<str>>(mut self, name: S, value: S) -> Self {
        self.command
            .env
            .insert(name.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    /// Supply a function that will run in a separate thread with the ChildStdout from the simics
    /// process passed to it when it starts. For example, this is useful for directing the SIMICS
    /// output to a log.
    pub fn with_stdout_function<F>(mut self, function: F) -> Self
    where
        F: Fn(ChildStdout) + Send + Sync + 'static,
    {
        self.command.stdout_function = Some(Arc::new(function));
        self
    }

    /// Supply a function that will run in a separate thread with the ChildStderr from the simics
    /// process passed to it when it starts. For example, this is useful for directing the SIMICS
    /// output to a log.
    pub fn with_stderr_function<F>(mut self, function: F) -> Self
    where
        F: Fn(ChildStderr) + Send + Sync + 'static,
    {
        self.command.stderr_function = Some(Arc::new(function));
        self
    }

    /// Supply a function that will run in a separate thread with the ChildStdin from the simics
    /// process passed to it when it starts. For example, this is useful for sending commands to
    /// a simics process from a channel
    pub fn with_stdin_function<F>(mut self, function: F) -> Self
    where
        F: Fn(ChildStdin) + Send + Sync + 'static,
    {
        self.command.stdin_function = Some(Arc::new(function));
        self
    }

    /// Set the log level for the simics project. This won't be used for anything by default, but
    /// it can be accessed to set readers
    pub fn with_loglevel(mut self, level: Level) -> Self {
        self.loglevel = level;
        self
    }
}

impl Drop for SimicsProject {
    /// Remove the simics project from disk if it was created with an automatic project directory,
    /// does nothing otherwise.
    fn drop(&mut self) {
        if self.tmp {
            info!("Removing SIMICS project from disk");
            remove_dir_all(&self.base_path).ok();
        }
    }
}

impl SimicsProject {
    /// Try to clone the project. This is possible as long as the project command is not yet
    /// running. Running multiple copies of SIMICS from the same project is not supported, so this
    /// clone will copy the project to a new directory at a given location
    pub fn try_clone_at<P: AsRef<Path>>(&self, location: P) -> Result<Self> {
        let location = location.as_ref().to_path_buf();

        if !location.is_dir() {
            create_dir_all(&location)?;
            copy_dir_contents(&self.base_path, &location)?;
        } else {
            bail!("Target location {} already exists", location.display());
        }

        Ok(Self {
            base_path: location,
            base_version_constraint: self.base_version_constraint.clone(),
            home: self.home.clone(),
            packages: self.packages.clone(),
            modules: self.modules.clone(),
            tmp: self.tmp,
            command: self.command.try_clone()?,
            loglevel: self.loglevel,
            built: self.built,
            contents: self.contents.clone(),
        })
    }

    /// Try to clone the project. This is possible as long as the project command is not yet
    /// running. Running multiple copies of SIMICS from the same project is not supported, so this
    /// clone will copy the project to a new directory with a random suffix name next to the
    /// original one
    pub fn try_clone(&self) -> Result<Self> {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(4)
            .map(char::from)
            .collect();
        let location = self
            .base_path
            .parent()
            .context("No base path parent")?
            .join(
                self.base_path
                    .components()
                    .last()
                    .context("No final path component")?
                    .as_os_str()
                    .to_string_lossy()
                    .to_string()
                    + &suffix,
            );

        debug!(
            "Cloning simics project to new location {}",
            location.display()
        );

        self.try_clone_at(location)
    }
}
