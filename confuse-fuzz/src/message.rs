use memfd::Memfd;
use serde::{Deserialize, Serialize};
use unix_ipc::Handle;

#[derive(Serialize, Deserialize)]
pub enum Message {
    MemFieldHandle(Handle<Memfd>),
}
