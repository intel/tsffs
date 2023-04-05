//! Individual messages that can be passed between the module and a client of it

use self::{client::ClientMessage, module::ModuleMessage};
pub mod client;
pub mod module;

pub enum Message {
    Client(ClientMessage),
    Module(ModuleMessage),
}
