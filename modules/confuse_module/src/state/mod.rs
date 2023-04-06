//! Implements the state machine for this module. This state machine represents the different
//! states the module (and through it, simics) can be in and the transitions between those states

use anyhow::Result;
use rust_fsm::*;

use crate::module::controller::messages::{client::ClientMessage, module::ModuleMessage};

state_machine! {
    derive(Debug)
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
        Reset => Ready,
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
        Ok(self.machine.consume(&input)?)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
