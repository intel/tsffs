use std::collections::HashSet;

use self::fault::Fault;

pub mod fault;

pub struct FaultDetector {
    /// The set of faults that are considered crashes for this fuzzing campaign
    pub faults: HashSet<Fault>,
    /// The duration after the start harness to treat as a timeout, in seconds
    /// Use `set_timeout_seconds` or `set_timeout_milliseconds` instead of
    /// doing the math yourself!
    pub timeout: Option<f64>,
}

impl FaultDetector {}
