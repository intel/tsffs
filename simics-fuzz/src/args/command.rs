use super::path::ArgPath;
use anyhow::{anyhow, Error};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub enum Command {
    Command { command: String },
    Python { file: ArgPath },
    Config { config: ArgPath },
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts = s.split(':').collect::<Vec<_>>();
        match (parts.first(), parts.get(1)) {
            (Some(&"PYTHON"), Some(value)) => Ok(Command::Python {
                file: value.parse()?,
            }),
            (Some(&"COMMAND"), Some(value)) => Ok(Command::Command {
                command: value.to_string(),
            }),
            (Some(&"CONFIG"), Some(value)) => Ok(Command::Config {
                config: value.parse()?,
            }),
            _ => Err(anyhow!("Invalid command {}", s)),
        }
    }
}
