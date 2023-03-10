use std::fmt::Debug;

use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Events the fuzzer generates that simics consumes
pub enum FuzzerEvent {
    Initialize,
    Run(Vec<u8>),
    Reset,
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
/// Events simics generates that the fuzzer consumes
pub enum SimicsEvent {
    Ready,
    Stopped,
    Done,
    SharedMem(IpcShm),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    FuzzerEvent(FuzzerEvent),
    SimicsEvent(SimicsEvent),
}
