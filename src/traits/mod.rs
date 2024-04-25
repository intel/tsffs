// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::tracer::{CmpExpr, CmpType};
use anyhow::Result;

/// Trait for disassemblers of various architectures to implement to permit branch
/// and compare tracing
pub trait TracerDisassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()>;
    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String>;
    fn last_was_control_flow(&self) -> bool;
    fn last_was_call(&self) -> bool;
    fn last_was_ret(&self) -> bool;
    fn last_was_cmp(&self) -> bool;
    fn cmp(&self) -> Vec<CmpExpr>;
    fn cmp_type(&self) -> Vec<CmpType>;
}
