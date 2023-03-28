//! Configuration data for the module, passed to it when it starts up

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{components::detector::fault::Fault, map_type::MapType};

#[derive(Debug, Serialize, Deserialize)]
/// Contains parameters for the module to configure things like timeout duration, which faults
/// indicate a crash, etc. This is sent by the client in `ClientMessage::Initialize`
pub struct InitializeConfig {
    pub faults: HashSet<Fault>,
    pub timeout: f64,
}

impl InitializeConfig {
    /// Add a fault to the set of faults considered crashes for a given fuzzing campaign
    pub fn with_fault(&mut self, fault: Fault) -> &mut Self {
        self.faults.insert(fault);
        self
    }

    /// Add one or more faults to the set of faults considered crashes for a given fuzzing
    /// campaign
    pub fn with_faults<I: IntoIterator<Item = Fault>>(&mut self, faults: I) -> &mut Self {
        faults.into_iter().for_each(|i| {
            self.faults.insert(i);
        });
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout_seconds(&mut self, seconds: f64) -> &mut Self {
        self.timeout = seconds;
        self
    }

    pub fn with_timeout_milliseconds(&mut self, milliseconds: f64) -> &mut Self {
        self.timeout = milliseconds / 1000.0;
        self
    }

    pub fn with_timeout_microseconds(&mut self, microseconds: f64) -> &mut Self {
        self.timeout = microseconds / 1_000_000.0;
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Contains the resulting configuration of the module after initialization with the provided
/// `InitializeConfig`. This is used to pass memory maps back to the client for things like
/// coverage and cmplog data, but can be extended.
pub struct InitializedConfig {
    maps: Vec<MapType>,
}

impl InitializedConfig {
    pub fn with_map(&mut self, map: MapType) -> &mut Self {
        self.maps.push(map);
        self
    }

    pub fn with_maps<I: IntoIterator<Item = MapType>>(&mut self, maps: I) -> &mut Self {
        maps.into_iter().for_each(|m| {
            self.maps.push(m);
        });
        self
    }
}
