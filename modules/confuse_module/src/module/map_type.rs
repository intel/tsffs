use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MapType {
    Coverage(IpcShm),
}
