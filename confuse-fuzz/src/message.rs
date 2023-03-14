use std::fmt::Debug;

pub use crate::{InitInfo, StopType};
use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Events the fuzzer generates that simics consumes
pub enum FuzzerEvent {
    Initialize(InitInfo),
    Run(Vec<u8>),
    Reset,
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
/// Events simics generates that the fuzzer consumes
pub enum SimicsEvent {
    Ready,
    Stopped(StopType),
    Done,
    SharedMem(IpcShm),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    FuzzerEvent(FuzzerEvent),
    SimicsEvent(SimicsEvent),
}
