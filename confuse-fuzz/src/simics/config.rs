use std::{collections::HashMap, fs::read_to_string, path::PathBuf};

use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use serde_yaml::from_str;

#[derive(Serialize, Deserialize)]
pub struct SimicsConfigParam {
    #[serde(rename = "type")]
    typ: String,
    default: String,
}
#[derive(Serialize, Deserialize)]
pub struct SimicsConfig {
    description: String,
    params: HashMap<String, SimicsConfigParam>,
    script: String,
}

impl TryFrom<PathBuf> for SimicsConfig {
    type Error = Error;
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let content = read_to_string(&value)?;
        from_str(&content).context("Unable to deserialize YAML data.")
    }
}
