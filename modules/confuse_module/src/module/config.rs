//! Configuration data for the module, passed to it when it starts up

use anyhow::{bail, Context, Result};
use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{components::detector::fault::Fault, map_type::MapType};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Contains parameters for the module to configure things like timeout duration, which faults
/// indicate a crash, etc. This is sent by the client in `ClientMessage::Initialize`
pub struct InitializeConfig {
    pub faults: HashSet<Fault>,
    pub timeout: f64,
}

impl Default for InitializeConfig {
    fn default() -> Self {
        Self {
            faults: HashSet::new(),
            timeout: f64::MAX,
        }
    }
}

impl InitializeConfig {
    /// Add a fault to the set of faults considered crashes for a given fuzzing campaign
    pub fn with_fault(mut self, fault: Fault) -> Self {
        self.faults.insert(fault);
        self
    }

    /// Add one or more faults to the set of faults considered crashes for a given fuzzing
    /// campaign
    pub fn with_faults<I: IntoIterator<Item = Fault>>(mut self, faults: I) -> Self {
        faults.into_iter().for_each(|i| {
            self.faults.insert(i);
        });
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout_seconds(mut self, seconds: f64) -> Self {
        self.timeout = seconds;
        self
    }

    pub fn with_timeout_milliseconds(mut self, milliseconds: f64) -> Self {
        self.timeout = milliseconds / 1000.0;
        self
    }

    pub fn with_timeout_microseconds(mut self, microseconds: f64) -> Self {
        self.timeout = microseconds / 1_000_000.0;
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
/// Contains the resulting configuration of the module after initialization with the provided
/// `InitializeConfig`. This is used to pass memory maps back to the client for things like
/// coverage and cmplog data, but can be extended.
pub struct InitializedConfig {
    maps: Vec<MapType>,
}

impl InitializedConfig {
    pub fn with_map(mut self, map: MapType) -> Self {
        self.maps.push(map);
        self
    }

    pub fn with_maps<I: IntoIterator<Item = MapType>>(mut self, maps: I) -> Self {
        maps.into_iter().for_each(|m| {
            self.maps.push(m);
        });
        self
    }

    pub fn coverage(&mut self) -> Result<IpcShm> {
        match self.maps.remove(
            self.maps
                .iter()
                .position(|m| matches!(m, MapType::Coverage(_)))
                .context("No coverage map found")?,
        ) {
            MapType::Coverage(coverage) => Ok(coverage),
            _ => bail!("Failed to get coverage map"),
        }
    }
}
