use crate::{
    faults::Fault,
    traits::{ConfuseInterface, ConfuseState},
};
use anyhow::Result;
use simics_api::ConfClass;
use std::collections::HashSet;

pub struct Detector {
    pub faults: HashSet<Fault>,
    pub timeout_seconds: Option<f64>,
    pub timeout_event: ConfClass,
}

impl Default for Detector {
    fn default() -> Self {
        let timeout_event = ConfClass::default();

        Self {
            faults: HashSet::new(),
            timeout_seconds: None,
            timeout_event,
        }
    }
}

impl Detector {
    pub fn try_new() -> Result<Self> {
        Ok(Detector::default())
    }
}

impl ConfuseState for Detector {}

impl ConfuseInterface for Detector {}
