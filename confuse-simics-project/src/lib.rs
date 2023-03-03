use std::{
    collections::{HashMap, HashSet},
    fs::{copy, create_dir_all, remove_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{ensure, Context, Result};
use dotenvy_macro::dotenv;
use log::{error, info};

use confuse_simics_manifest::{package_infos, simics_latest, PackageNumber};
use confuse_simics_module::SimicsModule;
use serde::{Deserialize, Serialize};
use serde_yaml::to_string;
use tempdir::TempDir;
use walkdir::WalkDir;

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
const SIMICS_PROJECT_PREFIX: &str = "simics_project";

pub struct SimicsProject {
    pub base_path: PathBuf,
    pub home: PathBuf,
    packages: HashSet<PackageNumber>,
    modules: HashSet<SimicsModule>,
    tmp: bool,
}

impl SimicsProject {
    /// Try to create a new temporary simics project. If a project is created this way, it is
    /// removed from disk when this object is dropped.
    pub fn try_new() -> Result<Self> {
        let base_path = TempDir::new(SIMICS_PROJECT_PREFIX)?;
        let base_path = base_path.into_path();
        let mut project = SimicsProject::try_new_at(base_path)?;
        project.tmp = true;
        Ok(project)
    }

    /// Try to add a package to this project by number
    pub fn try_with_package(mut self, package: PackageNumber) -> Result<Self> {
        if self.packages.contains(&package) {
            return Ok(self);
        }

        let package_infos = package_infos(&self.home)?;

        let package_info = package_infos.get(&package).with_context(|| {
            error!("Package {:?} not be found in package info", package);
            "Package does not exist"
        })?;

        let simics_package_list_path = self.base_path.join(".package-list");

        let package_path = package_info
            .get_package_path(&self.home)?
            .to_string_lossy()
            .to_string();

        let simics_package_list = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&simics_package_list_path)?;

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

    /// Create a simics project at a specific path. When a project is created this way, it is
    /// not deleted when it is dropped and will instead persist on disk.
    pub fn try_new_at<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let base_path = base_path.canonicalize()?;
        if !base_path.exists() {
            create_dir_all(&base_path)?;
        }
        let simics_home = PathBuf::from(SIMICS_HOME).canonicalize()?;
        let latest_simics_manifest = simics_latest(&simics_home)?;
        let simics_base_dir = simics_home.join(format!(
            "simics-{}",
            latest_simics_manifest.packages[&PackageNumber::Base].version
        ));

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

    pub fn module_load_args(&self) -> Vec<String> {
        vec![]
    }

    /// Get the simics executable for this project as a command ready to run with arguments
    pub fn command(&self) -> Command {
        Command::new(&self.base_path.join("simics"))
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "default")]
pub enum SimicsAppParamType {
    Int(Option<i64>),
    File(Option<String>),
    Bool(Option<bool>),
    Str(Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimicsAppParam {
    // default: Option<T>,
    #[serde(flatten)]
    pub param: SimicsAppParamType,
    pub output: Option<bool>,
}

impl SimicsAppParam {
    pub fn default() -> Self {
        SimicsAppParam::new_int()
    }
    pub fn new_int() -> Self {
        Self {
            param: SimicsAppParamType::Int(None),
            output: None,
        }
    }

    pub fn new_file() -> Self {
        Self {
            param: SimicsAppParamType::File(None),
            output: None,
        }
    }

    pub fn new_bool() -> Self {
        Self {
            param: SimicsAppParamType::Bool(None),
            output: None,
        }
    }

    pub fn new_str() -> Self {
        Self {
            param: SimicsAppParamType::Str(None),
            output: None,
        }
    }

    pub fn int(mut self, value: i64) -> Self {
        self.param = SimicsAppParamType::Int(Some(value));
        self
    }

    pub fn file<S: AsRef<str>>(mut self, value: S) -> Self {
        self.param = SimicsAppParamType::File(Some(value.as_ref().to_string()));
        self
    }

    pub fn bool(mut self, value: bool) -> Self {
        self.param = SimicsAppParamType::Bool(Some(value));
        self
    }

    pub fn str<S: AsRef<str>>(mut self, value: S) -> Self {
        self.param = SimicsAppParamType::Str(Some(value.as_ref().to_string()));
        self
    }

    pub fn output(mut self, value: bool) -> Self {
        self.output = Some(value);
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// YAML Serializable Simics app description
pub struct SimicsApp {
    pub description: String,
    pub params: HashMap<String, SimicsAppParam>,
    pub script: String,
}

impl SimicsApp {
    pub fn new<S: AsRef<str>>(description: S, script: S) -> Self {
        Self {
            description: description.as_ref().to_string(),
            params: HashMap::new(),
            script: script.as_ref().to_string(),
        }
    }

    pub fn param<S: AsRef<str>>(mut self, key: S, param: SimicsAppParam) -> Self {
        self.params.insert(key.as_ref().to_string(), param);
        self
    }

    pub fn try_to_string(&self) -> Result<String> {
        let mut appstr: String = to_string(&self)?;
        // YAML allows no quotes, signle quotes, or double quotes. Simics, on the other hand, will
        // reject anything not double quoted. :))))))))))))))))))))))))))))))))))))))
        appstr = appstr.replace(r#"'"#, r#"""#);
        Ok("%YAML 1.2\n---\n".to_owned() + &appstr)
    }
}
