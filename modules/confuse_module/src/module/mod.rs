use ipc_channel::ipc::{IpcReceiver, IpcSender};
use simics_api::ConfObject;

use crate::{
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
};

pub struct Confuse {
    conf_obj: ConfObject,
    state: State,
    tx: IpcSender<ModuleMessage>,
    rx: IpcReceiver<ClientMessage>,
}
