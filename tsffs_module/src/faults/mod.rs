//! Defines a Fault that is interpreted by the `FaultDetector` component of the
//! module to determine what faults are "valid" i.e. are faults that we care about for a
//! given run. For example, an x86_64 edk2 UEFI application dereferncing an unmapped
//! pointer will generate a Page Fault (exception #14 on x86).
//!
//! The top-level `Fault` enum is designed to be platform-independent, and encapsulates a
//! platform-specific fault enum. In general, additional sideband faults can be defined
//! as negative numbers, although this may not be supported on all platforms

extern crate num_traits;
use self::x86_64::X86_64Fault;

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

pub mod x86_64;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Copy, Clone)]
#[repr(i64)]
/// An architecture independent container for faults on various architectures
pub enum Fault {
    X86_64(X86_64Fault),
}

impl TryInto<i64> for Fault {
    type Error = Error;
    fn try_into(self) -> Result<i64> {
        match self {
            Fault::X86_64(f) => f.try_into(),
        }
    }
}

impl TryInto<i64> for &Fault {
    type Error = Error;
    fn try_into(self) -> Result<i64> {
        match self {
            Fault::X86_64(f) => f.try_into(),
        }
    }
}
