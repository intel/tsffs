use crate::{
    config::OutputConfig,
    interface::{ConfuseModuleInterface, Interface},
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
    BOOTSTRAP_SOCKNAME, CLASS_NAME,
};
use anyhow::{bail, Context, Result};
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use log::trace;
use simics_api::{register_interface, ConfObject, OwnedMutAttrValuePtr, OwnedMutConfObjectPtr};
use simics_api::{Create, Module};
use simics_api_derive::module;
use std::env::var;

mod components;

#[module(class_name = CLASS_NAME)]
pub struct Confuse {
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
}

impl Module for Confuse {
    fn init(obj: OwnedMutConfObjectPtr) -> Result<OwnedMutConfObjectPtr> {
        let state = State::new();
        let sockname = var(BOOTSTRAP_SOCKNAME)?;
        let bootstrap = IpcSender::connect(sockname)?;

        let (otx, rx) = channel::<ClientMessage>()?;
        let (tx, orx) = channel::<ModuleMessage>()?;

        bootstrap.send((otx, orx))?;

        Ok(Confuse::new(obj, state, tx, rx))
    }

    fn objects_finalized(obj: OwnedMutConfObjectPtr) -> Result<()> {
        let confuse: &mut Confuse = obj.into();
        confuse.initialize()?;
        Ok(())
    }
}

impl Confuse {
    pub fn initialize(&mut self) -> Result<()> {
        let config = match self.recv_msg()? {
            ClientMessage::Initialize(config) => config,
            _ => bail!("Expected initialize command"),
        };

        let mut output_config = OutputConfig::default();

        self.send_msg(ModuleMessage::Initialized(output_config))?;

        Ok(())
    }

    /// Send a message to the module
    fn send_msg(&mut self, msg: ModuleMessage) -> Result<()> {
        trace!("Sending module message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming sent message {:?}", msg))?;
        self.tx.send(msg)?;
        Ok(())
    }

    /// Receive a message from the module
    fn recv_msg(&mut self) -> Result<ClientMessage> {
        trace!("Waiting to receive client message");
        let msg = self.rx.recv()?;
        trace!("Received client message {:?}", msg);
        self.state
            .consume(&msg)
            .context(format!("Error consuming received message {:?}", msg))?;
        Ok(msg)
    }
}

/// Implementation of the functionality accessible via the SIMICS Python or CLI interface
impl Interface for Confuse {
    /// Start the module side of the fuzzing loop
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Add a processor to the module's state
    fn add_processor(&mut self, processor: OwnedMutAttrValuePtr) -> Result<()> {
        Ok(())
    }

    /// Add a fault to the module's state
    fn add_fault(&mut self, fault: i64) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
/// Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
/// module
pub extern "C" fn confuse_init_local() {
    let cls = Confuse::create().unwrap_or_else(|_| panic!("Failed to create class {}", CLASS_NAME));
    register_interface::<_, ConfuseModuleInterface>(cls, CLASS_NAME)
        .unwrap_or_else(|_| panic!("Failed to register interface for class {}", CLASS_NAME));
}
