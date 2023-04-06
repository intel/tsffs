//! Implements the state machine for this module. This state machine represents the different
//! states the module (and through it, simics) can be in and the transitions between those states

use anyhow::{bail, Result};
use log::{error, info};
use rust_fsm::*;

use crate::module::controller::messages::{client::ClientMessage, module::ModuleMessage};

state_machine! {
    derive(Debug, Clone)
    pub ConfuseModule(Uninitialized)

    Uninitialized => {
        Initialize => HalfInitialized,
        Exit => Done,
    },
    HalfInitialized => {
        Initialized => Initialized,
        Exit => Done,
    },
    Initialized => {
        Reset => HalfReady,
        Exit => Done,
    },
    HalfReady => {
        Ready => Ready,
        Exit => Done,
    },
    Ready => {
        Run => Running,
        Exit => Done,
    },
    Running(Stopped) => Stopped,
    Stopped => {
        Reset => HalfReady,
        Exit => Done,
    }
}

impl From<&ClientMessage> for ConfuseModuleInput {
    fn from(value: &ClientMessage) -> Self {
        match value {
            ClientMessage::Initialize(_) => Self::Initialize,
            ClientMessage::Run(_) => Self::Run,
            ClientMessage::Reset => Self::Reset,
            ClientMessage::Exit => Self::Exit,
        }
    }
}

impl From<&ModuleMessage> for ConfuseModuleInput {
    fn from(value: &ModuleMessage) -> Self {
        match value {
            ModuleMessage::Initialized(_) => Self::Initialized,
            ModuleMessage::Ready => Self::Ready,
            ModuleMessage::Stopped(_) => Self::Stopped,
        }
    }
}

pub struct State {
    machine: StateMachine<ConfuseModule>,
}

impl State {
    pub fn new() -> Self {
        Self {
            machine: StateMachine::new(),
        }
    }

    /// Consume a client or module message to trigger state transitions in the machine
    ///
    /// This function should be called whenever a message is sent or received, with the message
    /// as an argument. Inconsistent state will occur if this rule isn't followed, so be sure
    /// to use the `send` and `recv` methods on the `Client` and `Controller` respectively to
    /// keep this consistent, as they will call this method automatically
    pub fn consume<M: Into<ConfuseModuleInput>>(&mut self, message: M) -> Result<Option<()>> {
        let input: ConfuseModuleInput = message.into();
        let pre_state = self.machine.state().clone();
        let result = self.machine.consume(&input);
        let post_state = self.machine.state().clone();

        match result {
            Ok(r) => {
                info!(
                    "Consumed {:?}: Transitioned from {:?} -> {:?}",
                    input, pre_state, post_state
                );
                Ok(r)
            }
            Err(e) => {
                error!(
                    "Tried to consume {:?}: Failed to transition from {:?}: {}",
                    input, pre_state, e
                );
                bail!(
                    "Tried to consume {:?}: Failed to transition from {:?}: {}",
                    input,
                    pre_state,
                    e
                );
            }
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
