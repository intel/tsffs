//! Implements the state machine for this module. This state machine represents the different
//! states the module (and through it, simics) can be in and the transitions between those states
//!
//! The module starts in the `Uninitialized` state. This state is named slightly deceptively --
//! the module will do any allocations it needs, set up its components, map memory, and more
//! in this state.
//!
//! When the module is instructed to `Initialize`, it moves to a `HalfInitialized` state. In this
//! state, it has received the `InputConfig` from the client, but it has not applied it. Only
//! once the module sends back an `Initialized` message to the client is the module fully
//! `Initialized`, which means it has taken its initial snapshot and is ready to begin the fuzzing
//! process (in this state, the client has received any necessary shared information from the
//! module as well).
//!
//! Next, the module will receive a `Reset` signal and will enter the `HalfReady` state. The
//! module will internally reset the target state to the snapshot (on first run, this is
//! essentially a no-op) and send back a `Ready` message. Only then is the module ready to actually
//! run a test.
//!
//! Finally, the module will receive a `Run` signal, and will start running with the given input.
//! When the module reaches a `Stopped` state, it will signal back that it has stopped as well
//! as the reason. From this state, it can be `Reset` again before another `Run`.
//!
//! At any point, an `Exit` signal can be sent to cause the module to immediately exit cleanly.

use anyhow::{bail, Result};
use rust_fsm::*;
use tracing::{error, info};

use crate::messages::{client::ClientMessage, module::ModuleMessage};

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
