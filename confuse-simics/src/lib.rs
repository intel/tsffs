pub mod api;

use std::{
    collections::HashMap,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use dotenvy_macro::dotenv;
use log::{error, info, warn};

use confuse_simics_manifest::{package_infos, simics_latest, PackageNumber};
use serde::{Deserialize, Serialize};
use serde_yaml::to_string;

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

/// Set up a SIMICs project with a specified set of packages
pub fn setup_simics_project<P: AsRef<Path>>(
    base_path: P,
    packages: Vec<PackageNumber>,
) -> Result<()> {
    let base_path: PathBuf = base_path.as_ref().to_path_buf();
    info!(
        "Initializing simics project with base path {:?}",
        &base_path
    );

    if !base_path.exists() {
        create_dir_all(&base_path)?;
        info!("Base path {:?} does not exist. Creating it.", &base_path);
    }

    let simics_home = PathBuf::from(SIMICS_HOME);
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

    let package_infos = package_infos(&simics_home)?;

    let package_paths: Vec<String> = packages
        .iter()
        .filter_map(|pn| match package_infos.get(pn) {
            Some(pn) => Some(pn),
            None => {
                warn!("No package info for package number {:?}", pn);
                None
            }
        })
        .filter_map(|pi| match pi.get_package_path(&simics_home) {
            Ok(p) => Some(p.to_string_lossy().to_string()),
            Err(e) => {
                error!("Could not get package path for {:?}: {}", pi, e);
                None
            }
        })
        .collect();

    let simics_package_list_path = base_path.join(".package-list");

    info!(
        "Writing package list ({} packages) to {:?}",
        package_paths.len(),
        simics_package_list_path
    );

    let mut simics_package_list = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&simics_package_list_path)?;

    simics_package_list.write_all((package_paths.join("\n") + "\n").as_bytes())?;

    let simics_project_project_setup = base_path.join("bin").join("project-setup");

    info!("Running {:?}", simics_project_project_setup);

    Command::new(&simics_project_project_setup)
        .current_dir(&base_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

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
