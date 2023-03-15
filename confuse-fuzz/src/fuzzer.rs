use ipc_channel::ipc::IpcSender;

use crate::message::{FuzzerEvent, SimicsEvent};

pub struct Fuzzer {
    tx: IpcSender<FuzzerEvent>,
    rx: IpcSender<SimicsEvent>,
}

impl Fuzzer {
    pub fn new() -> Self {}
}
