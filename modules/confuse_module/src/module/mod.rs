use ipc_channel::ipc::{IpcReceiver, IpcSender};
use simics_api::{ConfObject, OwnedMutConfObjectPtr};
use simics_api_derive::module;

use crate::{
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
};

#[module]
pub struct Confuse {
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
}
