use std::fmt::Debug;

use ipc_channel::ipc::IpcSharedMemory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Events the fuzzer generates that simics consumes
pub enum FuzzerEvent {
    Initialize,
    Run,
    Reset,
    Stop,
}

#[derive(Serialize, Deserialize)]
/// Events simics generates that the fuzzer consumes
pub enum SimicsEvent {
    Ready,
    Done,
    MapHandle(IpcSharedMemory),
}

// Required because Handle doesn't impl debug
impl Debug for SimicsEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimicsEvent::MapHandle(_) => write!(f, "MapHandle()"),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    FuzzerEvent(FuzzerEvent),
    SimicsEvent(SimicsEvent),
}
