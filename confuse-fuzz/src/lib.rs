pub mod message;

extern crate num_traits;
#[macro_use]
extern crate num_derive;

use anyhow::{Context, Error, Result};
use num;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, FromPrimitive, Hash, PartialEq, Eq)]
#[repr(i64)]
pub enum Fault {
    Triple = -1,
    Division = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    X86FpeFault = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFpeFault = 19,
    VirtualizationException = 20,
    ControlprotectionException = 21,
}

impl TryFrom<i64> for Fault {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        num::FromPrimitive::from_i64(value).context("Could not convert to Fault")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitInfo {
    pub faults: HashSet<Fault>,
}

impl InitInfo {
    pub fn add_fault(&mut self, fault: Fault) {
        self.faults.insert(fault);
    }
}

impl Default for InitInfo {
    fn default() -> Self {
        Self {
            faults: HashSet::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StopType {
    Normal,
    Crash,
    TimeOut,
}
