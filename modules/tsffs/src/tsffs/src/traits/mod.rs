// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::state::StopReason;
use anyhow::Result;

pub trait Component {
    /// Called on module initialization. Components can do one-time setup here.
    fn on_init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called whenever the simulation is stopped, with the reason it was stopped. For start
    /// and stop reasons, the driver receives this callback last, so other components can
    /// configure and take pre-run and post-run actions.
    fn on_simulation_stopped(&mut self, _reason: &StopReason) -> Result<()> {
        Ok(())
    }
}
