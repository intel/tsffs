// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use ipc_shm::IpcShm;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MapType {
    Coverage(IpcShm),
}
