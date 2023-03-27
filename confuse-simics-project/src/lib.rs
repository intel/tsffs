//! Confuse Simics Project
//!
//! This crate provides tools for managing simics projects, including linking to simics, loading
//! modules, and creating and destroying temporary project directories

use std::{
    collections::HashSet,
    fs::{copy, create_dir_all, remove_dir_all, OpenOptions},
    io::Write,
    os::unix::fs::symlink,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use anyhow::{bail, ensure, Context, Result};
use dotenvy_macro::dotenv;
use indoc::formatdoc;
use log::{error, info};

use confuse_simics_manifest::{package_infos, simics_base_version, PackageNumber};
use confuse_simics_module::SimicsModule;
use regex::Regex;
use tempdir::TempDir;
use version_tools::VersionConstraint;
use versions::Versioning;
use walkdir::WalkDir;

/// The SIMICS home installation directory. A `.env` file containing a line like:
/// SIMICS_HOME=/home/username/simics/ must be present in the workspace tree
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
/// Prefix for naming temporary directories
const SIMICS_PROJECT_PREFIX: &str = "simics_project";

/// Return the SIMICS_HOME directory as a PathBuf
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

fn find_file_in_simics_base<P: AsRef<Path>, S: AsRef<str>>(
    simics_base_dir: P,
    file_name_pattern: S,
) -> Result<PathBuf> {
    let file_name_regex = Regex::new(file_name_pattern.as_ref())?;
    let found_file = WalkDir::new(&simics_base_dir)
        .into_iter()
        .filter_map(|de| de.ok())
        // is_ok_and is unstable ;_;
        .filter(|de| {
            if let Ok(m) = de.metadata() {
                m.is_file()
            } else {
                false
            }
        })
        .find(|de| {
            if let Some(name) = de.path().file_name() {
                file_name_regex.is_match(&name.to_string_lossy())
            } else {
                false
            }
        })
        .context(format!(
            "Could not find libsimics-common.so in {}",
            simics_base_dir.as_ref().display()
        ))?
        .path()
        .to_path_buf();

    ensure!(
        found_file.is_file(),
        "No file {} found in {}",
        file_name_pattern.as_ref(),
        simics_base_dir.as_ref().display()
    );

    Ok(found_file)
}

/// Link against simics. This is required for any SIMICS module (as well as anything that uses
/// the simics module, for example to access constants from it -- an unfortunate side effect but
/// not a big deal, we'll be linking it in to almost every process regardless.
pub fn link_simics<S: AsRef<str>>(version_constraint: S) -> Result<()> {
    let simics_home_dir = simics_home()?;

    let simics_base_info = simics_base_version(&simics_home_dir, &version_constraint)?;
    let simics_base_dir = simics_base_info.get_package_path(&simics_home_dir)?;

    let simics_common_lib = find_file_in_simics_base(&simics_base_dir, "libsimics-common.so")?;
    let simics_bin_dir = simics_home_dir
        .join(format!(
            "simics-{}",
            simics_base_version(simics_home()?, version_constraint)?.version
        ))
        .join("bin");

    ensure!(
        simics_bin_dir.is_dir(),
        "No bin directory found in {}",
        simics_home_dir.display()
    );

    let output = Command::new("ld.so")
        .arg(simics_common_lib)
        .stdout(Stdio::piped())
        .output()?;

    let ld_line_pattern = Regex::new(r#"\s*([^\s]+)\s*=>\s*not\sfound"#)?;
    let notfound_libs: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| {
            if let Some(captures) = ld_line_pattern.captures(l) {
                captures.get(1)
            } else {
                None
            }
        })
        .map(|m| m.as_str().to_string())
        .collect();

    info!("Locating {}", notfound_libs.join(", "));

    let mut lib_search_dirs = HashSet::new();

    for lib_name in notfound_libs {
        println!("cargo:rustc-link-lib=dylib:+verbatim={}", lib_name);
        let found_lib = find_file_in_simics_base(&simics_base_dir, lib_name)?;
        let found_lib_parent = found_lib.parent().context("No parent path found")?;
        lib_search_dirs.insert(found_lib_parent.to_path_buf());
    }

    for lib_search_dir in &lib_search_dirs {
        println!(
            "cargo:rustc-link-search=native={}",
            lib_search_dir.display()
        );
    }

    // NOTE: This only works for `cargo run` and `cargo test` and won't work for just running
    // the output binary
    let search_dir_strings = lib_search_dirs
        .iter()
        .map(|pb| pb.to_string_lossy())
        .collect::<Vec<_>>();
    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={}",
        search_dir_strings.join(";")
    );
    Ok(())
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

    pub fn try_new<S: AsRef<str>>(base_version_constraint: S) -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path, base_version_constraint)?;
        project.tmp = true;
        Ok(project)
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

    /// Create a simics project at a specific path. When a project is created this way, it is
    /// not deleted when it is dropped and will instead persist on disk.
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

    /// Try to add a shared object module to the simics project. This module may or may not already
    /// be signed using `sign_simics_module` but will be re-signed in all cases. This will fail if
    /// the module does not correctly include the symbols needed for simics to load it.
    pub fn try_with_module<S: AsRef<str>, P: AsRef<Path>>(
        mut self,
        module_crate_name: S,
        module: P,
    ) -> Result<Self> {
        let module_path = module.as_ref().to_path_buf();
        let module = SimicsModule::try_new(module_crate_name, &self.base_path, &module_path)?;
        self.modules.insert(module);
        Ok(self)
    }

    /// Retrieve the arguments for loading all the modules that are added to the project. The order
    /// is arbitrary, so if there is an ordering dependency you should specify these arguments
    /// manually
    pub fn module_load_args(&self) -> Vec<String> {
        self.modules
            .iter()
            .flat_map(|sm| ["-e".to_string(), format!("load-module {}", sm.name)])
            .collect()
    }

    /// Get the simics executable for this project as a command ready to run with arguments
    pub fn command(&self) -> Command {
        Command::new(self.base_path.join("simics"))
    }

    /// Make this project persistent (ie it will not be deleted when dropped)
    pub fn persist(&mut self) {
        self.tmp = false;
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

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
fn copy_dir_contents<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<()> {
    let src_dir = src_dir.as_ref().to_path_buf();
    ensure!(src_dir.is_dir(), "Source must be a directory");
    let dst_dir = dst_dir.as_ref().to_path_buf();

    for (src, dst) in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let src = p.path().to_path_buf();
            match src.strip_prefix(&src_dir) {
                Ok(suffix) => Some((src.clone(), dst_dir.join(suffix))),
                Err(_) => None,
            }
        })
    {
        if src.is_dir() {
            create_dir_all(&dst)?;
        } else if src.is_file() {
            copy(&src, &dst)?;
        }
    }
    Ok(())
}

const YAML_INDENT: &str = "  ";

/// Params to a simics app can be one of four types: int, file, bool, or str. They don't
/// necessarily have a "default" which is the value stored in this enum
pub enum SimicsAppParamType {
    Int(Option<i64>),
    File(Option<String>),
    Bool(Option<bool>),
    Str(Option<String>),
}

/// Parameter to a simics app, these always have a type, may have a default (if the default is
/// not provided, it must be set by the app's script), and they may set the boolean `output`.
pub struct SimicsAppParam {
    pub default: SimicsAppParamType,
    pub output: Option<bool>,
}

impl ToString for SimicsAppParam {
    fn to_string(&self) -> String {
        let mut pstr = vec![format!(
            "type: {}",
            match &self.default {
                SimicsAppParamType::Int(_) => "int",
                SimicsAppParamType::File(_) => "file",
                SimicsAppParamType::Str(_) => "str",
                SimicsAppParamType::Bool(_) => "bool",
            }
        )];

        match &self.default {
            SimicsAppParamType::Int(Some(v)) => {
                pstr.push(format!("default: {}", v));
            }
            SimicsAppParamType::File(Some(v)) => pstr.push(format!(r#"default: "{}""#, v)),
            SimicsAppParamType::Str(Some(v)) => pstr.push(format!(r#"default: "{}""#, v)),
            // Yet more inconsistency with YAML spec
            SimicsAppParamType::Bool(Some(v)) => pstr.push(format!(
                "default: {}",
                match v {
                    true => "TRUE",
                    false => "FALSE",
                }
            )),
            _ => {}
        };

        if let Some(output) = self.output {
            pstr.push(format!("output: {}", output));
        }

        pstr.iter()
            .map(|e| YAML_INDENT.to_string() + e)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl SimicsAppParam {
    pub fn new(typ: SimicsAppParamType) -> Self {
        Self {
            default: typ,
            output: None,
        }
    }

    pub fn set_output(&mut self, value: bool) {
        self.output = Some(value);
    }

    pub fn set_default(&mut self, value: SimicsAppParamType) {
        self.default = value;
    }
}

pub struct SimicsApp {
    pub description: String,
    pub params: Vec<(String, SimicsAppParam)>,
    pub script: String,
}

impl SimicsApp {
    pub fn new<S: AsRef<str>>(description: S, script: S) -> Self {
        Self {
            description: description.as_ref().to_string(),
            params: Vec::new(),
            script: script.as_ref().to_string(),
        }
    }

    pub fn param<S: AsRef<str>>(&mut self, key: S, param: SimicsAppParam) -> &mut Self {
        self.params.push((key.as_ref().to_string(), param));
        self
    }

    pub fn params_string(&self) -> String {
        self.params
            .iter()
            .map(|(k, p)| {
                format!("{}:\n{}", k, p.to_string())
                    .lines()
                    .map(|l| YAML_INDENT.to_string() + l)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn add_param<S: AsRef<str>>(&mut self, key: S, param: SimicsAppParam) {
        self.params.push((key.as_ref().to_string(), param));
    }
}

impl ToString for SimicsApp {
    fn to_string(&self) -> String {
        formatdoc! {r#"
            %YAML 1.2
            ---
            description: {}
            params:
            {}
            script: "{}"
            ...
            "#, 
            self.description,
            self.params_string(),
            self.script
        }
    }
}

#[macro_export]
macro_rules! int_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_default(SimicsAppParamType::Int(Some($dval)));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_default(SimicsAppParamType::Int(Some($dval)));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Int(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! str_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_default(SimicsAppParamType::Str(Some($dval.into())));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_default(SimicsAppParamType::Str(Some($dval.into())));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Str(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! file_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_default(SimicsAppParamType::File(Some($dval.into())));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_default(SimicsAppParamType::File(Some($dval.into())));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::File(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! bool_param {
    ($name:ident : { default: $dval:expr , output: $oval:expr $(,)? }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_default(SimicsAppParamType::Bool(Some($dval)));
        param.set_output($oval);

        (stringify!($name), param)
    }};
    ($name:ident : { default: $dval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_default(SimicsAppParamType::Bool(Some($dval)));

        (stringify!($name), param)
    }};
    ($name:ident : { output: $oval:expr }) => {{
        let mut param = SimicsAppParam::new(SimicsAppParamType::Bool(None));
        param.set_output($oval);

        (stringify!($name), param)
    }};
}

#[macro_export]
macro_rules! simics_app {
    ($description:expr, $script:expr, $($param:expr),* $(,)?) => {
        {
            let mut app = SimicsApp::new($description, $script);
            $(
                app.add_param($param.0, $param.1);
            )*
            app
        }
    }
}

#[macro_export]
/// Create a path relative to the simics project directory.
///
/// # Examples
///
/// ```
/// const SCRIPT_PATH: &str = "scripts/app.py";
/// let app = SimicsApp::new("An app", &simics_path!(SCRIPT_PATH));
/// assert_eq!(app.script, "%simics%/scripts/app.py");
/// ```
macro_rules! simics_path {
    ($path:expr) => {
        format!("%simics%/{}", $path)
    };
}
