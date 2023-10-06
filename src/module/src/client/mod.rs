// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! The TSFFS module client provides a common client-side controller for a fuzzer or other tool
//! to communicate with the module while keeping consistent with the state machine the module
//! implements.
//!
//! This client is designed to be used with the [`simics-fuzz`] crate, but can be used manually as
//! well to implement bespoke systems.
//!

use crate::{
    config::{InputConfig, OutputConfig, RunConfig},
    messages::{client::ClientMessage, module::ModuleMessage},
    state::ModuleStateMachine,
    stops::StopReason,
    traits::ThreadClient,
};
use anyhow::{bail, Result};
use std::sync::mpsc::{Receiver, Sender};
use tracing::{info, trace};

/// The client for the module. Allows controlling the module over IPC using the child
/// process spawned by a running project.
pub struct Client {
    /// State machine to keep track of the current state between the client and module
    state: ModuleStateMachine,
    /// Transmit end of IPC message channel between client and module
    tx: Sender<ClientMessage>,
    /// Receive end of IPC message channel between client and module
    rx: Receiver<ModuleMessage>,
}

impl Client {
    /// Try to initialize a `Client` from a built `SimicsProject` on disk, which should include
    /// the module and may have additional configuration according to user needs. Creating
    /// the client will start the SIMICS project, which should be configured as necessary *before*
    /// passing it into this constructor.
    ///
    /// The module will be added to the project for you if it is not present,
    /// so
    pub fn new(tx: Sender<ClientMessage>, rx: Receiver<ModuleMessage>) -> Self {
        Self {
            state: ModuleStateMachine::new(),
            tx,
            rx,
        }
    }
}

impl ThreadClient for Client {
    /// Initialize the client with a configuration. The client will return an output
    /// configuration which contains various information the SIMICS module needs to
    /// inform the client of, including memory maps for coverage. Changes the
    /// internal state from `Uninitialized` to `HalfInitialized` and then from
    /// `HalfInitialized` to `ModuleState::Initialized`.
    fn initialize(&mut self, config: InputConfig) -> Result<OutputConfig> {
        info!("Sending initialize message");
        self.send_msg(ClientMessage::Initialize(config))?;

        info!("Waiting for initialized message");
        if let ModuleMessage::Initialized(config) = self.recv_msg()? {
            Ok(config)
        } else {
            bail!("Initialization failed, received unexpected message");
        }
    }

    /// Reset the module to the beginning of the fuzz loop (the state as snapshotted).
    /// Changes the internal state from `Stopped` or `Initialized` to `HalfReady`, then
    /// from `HalfReady` to `Ready`.
    fn reset(&mut self) -> Result<()> {
        trace!("Sending reset message");
        self.send_msg(ClientMessage::Reset)?;

        trace!("Waiting for ready message");
        if let ModuleMessage::Ready = self.recv_msg()? {
            Ok(())
        } else {
            bail!("Reset failed, received unexpected message");
        }
    }

    /// Signal the module to run the target software. Changes the intenal state from `Ready` to
    /// `Running`, then once the run finishes either with a normal stop, a timeout, or a crash,
    /// from `Running` to `Stopped`. This function blocks until the target software stops and the
    /// module detects it, so it may take a long time or if there is an unexpected bug it may
    /// hang.
    fn run(&mut self, input: Vec<u8>, config: RunConfig) -> Result<StopReason> {
        trace!("Sending run message");
        self.send_msg(ClientMessage::Run((input, config)))?;

        trace!("Waiting for stopped message");
        if let ModuleMessage::Stopped(reason) = self.recv_msg()? {
            Ok(reason)
        } else {
            bail!("Run failed, received unexpected message");
        }
    }

    /// Signal the module to exit SIMICS, stopping the fuzzing process. Changes the internal state
    /// from any state to `Done`.
    fn exit(&mut self) -> Result<()> {
        info!("Sending exit message");
        self.send_msg(ClientMessage::Exit)?;

        Ok(())
    }

    fn state_mut(&mut self) -> &mut ModuleStateMachine {
        &mut self.state
    }

    fn rx_mut(&mut self) -> &mut Receiver<ModuleMessage> {
        &mut self.rx
    }

    fn tx_mut(&mut self) -> &mut Sender<ClientMessage> {
        &mut self.tx
    }
}
