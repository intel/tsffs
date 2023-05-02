//! Memory maps used by the CONFUSE module

use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Various types of memory maps used for tracing of various events
/// (particularly Coverage)
pub enum MapType {
    Coverage(IpcShm),
}
