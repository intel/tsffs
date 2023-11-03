// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state::StopReason,
    tracer::{CmpExpr, CmpType},
};
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

/// Trait for disassemblers of various architectures to implement to permit branch
/// and compare tracing
pub trait TracerDisassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()>;
    fn last_was_control_flow(&self) -> Result<bool>;
    fn last_was_call(&self) -> Result<bool>;
    fn last_was_ret(&self) -> Result<bool>;
    fn last_was_cmp(&self) -> Result<bool>;
    fn cmp(&self) -> Result<Vec<CmpExpr>>;
    fn cmp_type(&self) -> Result<Vec<CmpType>>;
}
