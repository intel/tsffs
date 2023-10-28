// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::state::StopReason;

pub trait Component {
    /// Called after the initial snapshot is taken
    fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()>;
}
