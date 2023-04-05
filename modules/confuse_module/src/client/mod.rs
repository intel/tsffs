//! The CONFUSE module client provides a common client-side controller for a fuzzer or other tool
//! to communicate with the module while keeping consistent with the state machine the module
//! implements.

use anyhow::{ensure, Result};
use confuse_simics_project::SimicsProject;
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};

use crate::{
    module::{
        controller::messages::{client::ClientMessage, module::ModuleMessage},
        entrypoint::CLASS_NAME,
    },
    state::State,
};
pub struct Client {
    /// State machine to keep track of the current state between the client and module
    state: State,
    /// Transmit end of IPC message channel between client and module
    tx: IpcSender<ClientMessage>,
    /// Receive end of IPC message channel between client and module
    rx: IpcReceiver<ModuleMessage>,
}

impl Client {
    /// Try to initialize a `Client` from a built `SimicsProject` on disk, which should include
    /// the CONFUSE module and may have additional configuration according to user needs
    pub fn try_new(project: SimicsProject) -> Result<Self> {
        // Make sure the project has our module loaded in it
        if !project.has_module(CLASS_NAME) {
            project = project.try_with_module(CLASS_NAME)?;
        }

        let (bootstrap, bootstrap_name) = IpcOneShotServer::new()?;

        Ok(Self {
            state: State::new(),
            tx,
            rx,
        })
    }

    /// Send a message to the module
    fn send_msg(&mut self, msg: ClientMessage) -> Result<()> {
        self.state.consume(&msg)?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the module
    fn recv_msg(&mut self) -> Result<ModuleMessage> {
        let msg = self.rx.recv()?;
        self.state.consume(&msg)?;
        Ok(msg)
    }
}
