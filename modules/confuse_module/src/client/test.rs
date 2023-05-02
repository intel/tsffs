//! The test client for CONFUSE, used during testing only

use crate::{
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
    traits::ConfuseClient,
};
use anyhow::{bail, Result};
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use log::info;

pub struct TestClient {
    state: State,
    tx: IpcSender<ClientMessage>,
    rx: IpcReceiver<ModuleMessage>,
}

impl TestClient {
    pub fn new(tx: IpcSender<ClientMessage>, rx: IpcReceiver<ModuleMessage>) -> Self {
        let state = State::new();
        Self { state, tx, rx }
    }

    pub fn new_boxed(
        tx: IpcSender<ClientMessage>,
        rx: IpcReceiver<ModuleMessage>,
    ) -> Box<dyn ConfuseClient> {
        let state = State::new();
        Box::new(Self { state, tx, rx })
    }
}

impl ConfuseClient for TestClient {
    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    fn tx_mut(&mut self) -> &mut IpcSender<ClientMessage> {
        &mut self.tx
    }

    fn rx_mut(&mut self) -> &mut IpcReceiver<ModuleMessage> {
        &mut self.rx
    }

    fn initialize(
        &mut self,
        config: crate::config::InputConfig,
    ) -> Result<crate::config::OutputConfig> {
        info!("Sending initialize message");
        self.send_msg(ClientMessage::Initialize(config))?;

        info!("Waiting for initialized message");
        if let ModuleMessage::Initialized(config) = self.recv_msg()? {
            Ok(config)
        } else {
            bail!("Initialization failed, received unexpected message");
        }
    }

    fn reset(&mut self) -> Result<()> {
        info!("Sending reset message");
        self.send_msg(ClientMessage::Reset)?;

        info!("Waiting for ready message");
        if let ModuleMessage::Ready = self.recv_msg()? {
            Ok(())
        } else {
            bail!("Reset failed, received unexpected message");
        }
    }

    fn run(&mut self, input: Vec<u8>) -> Result<crate::stops::StopReason> {
        info!("Sending run message");
        self.send_msg(ClientMessage::Run(input))?;

        info!("Waiting for stopped message");
        if let ModuleMessage::Stopped(reason) = self.recv_msg()? {
            Ok(reason)
        } else {
            bail!("Run failed, received unexpected message");
        }
    }

    fn exit(&mut self) -> Result<()> {
        info!("Sending exit message");
        self.send_msg(ClientMessage::Exit)?;
        Ok(())
    }
}
