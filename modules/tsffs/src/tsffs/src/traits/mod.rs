// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::tracer::{CmpExpr, CmpType};
use anyhow::Result;

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
