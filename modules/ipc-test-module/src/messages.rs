use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Events the fuzzer generates that SIMICS consumes
pub enum FuzzerEvent {
    /// Initialize event, the fuzzer signals the Confuse SIMICS module to initialize itself with
    /// a given set of global campaign settings
    Initialize,
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
    Stopped,
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
