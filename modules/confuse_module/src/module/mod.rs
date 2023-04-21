use crate::{
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
};
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use simics_api::Module;
use simics_api::{ConfObject, OwnedMutConfObjectPtr};
use simics_api_derive::{module, Module};

#[module(derive)]
pub struct Confuse {
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
}
