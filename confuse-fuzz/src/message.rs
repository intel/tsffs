//! Messages are information sent between the fuzzer and the Confuse SIMICS module and contain
//! events and corresponding information about those events. These events generally implicitly
//! define the fuzzing state machine of the SIMICS snapshot fuzzing process.

use std::fmt::Debug;

pub use crate::{InitInfo, StopType};
use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Events the fuzzer generates that SIMICS consumes
pub enum FuzzerEvent {
    /// Initialize event, the fuzzer signals the Confuse SIMICS module to initialize itself with
    /// a given set of global campaign settings
    Initialize(InitInfo),
    /// The fuzzer signals the Confuse SIMICS module to run with a given input of bytes
    Run(Vec<u8>),
    /// The fuzzer signals the Confuse SIMICS module to reset to the start snapshot
    Reset,
    /// The fuzzer signals the Confuse SIMICS module to stop execution and exit
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
/// Events SIMICS generates that the fuzzer consumes
pub enum SimicsEvent {
    /// Simics signals the fuzzer that it is ready to run
    Ready,
    /// Simics signals the fuzzer that it has stopped and why
    Stopped(StopType),
    /// Simics signals the fuzzer that it is done executing.
    Done,
    /// Simics sends the AFL map shared memory to the fuzzer
    SharedMem(IpcShm),
}

#[derive(Debug, Serialize, Deserialize)]
/// A wrapper for either a Fuzzer or SIMICS event
pub enum Message {
    FuzzerEvent(FuzzerEvent),
    SimicsEvent(SimicsEvent),
}
