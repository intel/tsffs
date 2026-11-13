// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture-specific implementation for PPE42 (PowerPC Processor Embedded 42) architecture

use anyhow::{anyhow, bail, Result};
use libafl::prelude::CmpValues;
use raw_cstr::AsRawCstr;
use simics::api::{
    get_interface, read_phys_memory, sys::instruction_handle_t, Access, ConfObject,
    CpuInstructionQueryInterface, CpuInstrumentationSubscribeInterface, CycleInterface,
    IntRegisterInterface, ProcessorInfoV2Interface,
};
use std::{ffi::CStr, mem::size_of, slice::from_raw_parts};

use crate::{
    tracer::{CmpExpr, CmpType, CmpValue, TraceEntry},
    traits::TracerDisassembler,
};

use super::ArchitectureOperations;

pub(crate) struct PPE42ArchitectureOperations {
    cpu: *mut ConfObject,
    disassembler: Disassembler,
    int_register: IntRegisterInterface,
    processor_info_v2: ProcessorInfoV2Interface,
    cpu_instruction_query: CpuInstructionQueryInterface,
    cpu_instrumentation_subscribe: CpuInstrumentationSubscribeInterface,
    cycle: CycleInterface,
}

impl ArchitectureOperations for PPE42ArchitectureOperations {
    // PPE42 uses PowerPC register conventions
    // r3-r10 are typically used for arguments
    // We'll use r10 for index selector and r3-r5 for arguments
    const INDEX_SELECTOR_REGISTER: &'static str = "r10";
    const ARGUMENT_REGISTER_0: &'static str = "r3";
    const ARGUMENT_REGISTER_1: &'static str = "r4";
    const ARGUMENT_REGISTER_2: &'static str = "r5";
    // PPE42 is an embedded processor - treat addresses as physical (no MMU translation)
    const USE_PHYSICAL_ADDRESSES: bool = true;
    // PPE42 is 32-bit, so pointers/size values are 4 bytes
    const POINTER_WIDTH_OVERRIDE: Option<i32> = Some(4);

    fn new(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)
            .map_err(|e| anyhow!("Failed to get ProcessorInfoV2Interface for PPE42: {}", e))?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        if arch == "ppc" || arch == "powerpc" || arch == "ppe42" || arch == "ppc32" {
            let int_register: IntRegisterInterface = get_interface(cpu)
                .map_err(|e| anyhow!("Failed to get IntRegisterInterface for PPE42: {}", e))?;
            let cpu_instruction_query: CpuInstructionQueryInterface = get_interface(cpu)
                .map_err(|e| anyhow!("Failed to get CpuInstructionQueryInterface for PPE42: {}", e))?;
            let cpu_instrumentation_subscribe: CpuInstrumentationSubscribeInterface = get_interface(cpu)
                .map_err(|e| anyhow!("Failed to get CpuInstrumentationSubscribeInterface for PPE42: {}", e))?;
            let cycle: CycleInterface = get_interface(cpu)
                .map_err(|e| anyhow!("Failed to get CycleInterface for PPE42: {}", e))?;

            Ok(Self {
                cpu,
                disassembler: Disassembler::new(),
                int_register,
                processor_info_v2,
                cpu_instruction_query,
                cpu_instrumentation_subscribe,
                cycle,
            })
        } else {
            bail!("Architecture {} is not PPE42/PowerPC", arch);
        }
    }

    fn new_unchecked(cpu: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            cpu,
            disassembler: Disassembler::new(),
            int_register: get_interface(cpu)?,
            processor_info_v2: get_interface(cpu)?,
            cpu_instruction_query: get_interface(cpu)?,
            cpu_instrumentation_subscribe: get_interface(cpu)?,
            cycle: get_interface(cpu)?,
        })
    }

    fn cpu(&self) -> *mut ConfObject {
        self.cpu
    }

    fn disassembler(&mut self) -> &mut dyn TracerDisassembler {
        &mut self.disassembler
    }

    fn int_register(&mut self) -> &mut IntRegisterInterface {
        &mut self.int_register
    }

    fn processor_info_v2(&mut self) -> &mut ProcessorInfoV2Interface {
        &mut self.processor_info_v2
    }

    fn cpu_instruction_query(&mut self) -> &mut CpuInstructionQueryInterface {
        &mut self.cpu_instruction_query
    }

    fn cpu_instrumentation_subscribe(&mut self) -> &mut CpuInstrumentationSubscribeInterface {
        &mut self.cpu_instrumentation_subscribe
    }

    fn cycle(&mut self) -> &mut CycleInterface {
        &mut self.cycle
    }

    fn trace_pc(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        let instruction_bytes = self.cpu_instruction_query.get_instruction_bytes(instruction_query)?;

        self.disassembler.disassemble(unsafe {
            from_raw_parts(instruction_bytes.data, instruction_bytes.size)
        })?;

        if self.disassembler.last_was_call()
            || self.disassembler.last_was_control_flow()
            || self.disassembler.last_was_ret()
        {
            Ok(TraceEntry::builder()
                .edge(self.processor_info_v2.get_program_counter()?)
                .build())
        } else {
            Ok(TraceEntry::default())
        }
    }

    fn trace_cmp(&mut self, instruction_query: *mut instruction_handle_t) -> Result<TraceEntry> {
        let instruction_bytes = self.cpu_instruction_query.get_instruction_bytes(instruction_query)?;
        self.disassembler.disassemble(unsafe {
            from_raw_parts(instruction_bytes.data, instruction_bytes.size)
        })?;

        let pc = self.processor_info_v2.get_program_counter()?;

        let mut cmp_values = Vec::new();

        for expr in self.disassembler.cmp() {
            if let Ok(value) = self.simplify(&expr) {
                cmp_values.push(value);
            }
        }

        let cmp_value = if let (Some(l), Some(r)) = (cmp_values.first(), cmp_values.get(1)) {
            match (l, r) {
                (CmpValue::U8(l), CmpValue::U8(r)) => Some(CmpValues::U8((*l, *r))),
                (CmpValue::I8(l), CmpValue::I8(r)) => Some(CmpValues::U8((
                    u8::from_le_bytes(l.to_le_bytes()),
                    u8::from_le_bytes(r.to_le_bytes()),
                ))),
                (CmpValue::U16(l), CmpValue::U16(r)) => Some(CmpValues::U16((*l, *r))),
                (CmpValue::I16(l), CmpValue::I16(r)) => Some(CmpValues::U16((
                    u16::from_le_bytes(l.to_le_bytes()),
                    u16::from_le_bytes(r.to_le_bytes()),
                ))),
                (CmpValue::U32(l), CmpValue::U32(r)) => Some(CmpValues::U32((*l, *r))),
                (CmpValue::I32(l), CmpValue::I32(r)) => Some(CmpValues::U32((
                    u32::from_le_bytes(l.to_le_bytes()),
                    u32::from_le_bytes(r.to_le_bytes()),
                ))),
                (CmpValue::U64(l), CmpValue::U64(r)) => Some(CmpValues::U64((*l, *r))),
                (CmpValue::I64(l), CmpValue::I64(r)) => Some(CmpValues::U64((
                    u64::from_le_bytes(l.to_le_bytes()),
                    u64::from_le_bytes(r.to_le_bytes()),
                ))),
                (CmpValue::Expr(_), CmpValue::Expr(_)) => None,
                _ => None,
            }
        } else {
            None
        };

        Ok(TraceEntry::builder()
            .cmp((
                pc,
                self.disassembler.cmp_type(),
                cmp_value.ok_or_else(|| anyhow!("No cmp value available"))?,
            ))
            .build())
    }
}

impl PPE42ArchitectureOperations {
    fn simplify(&mut self, expr: &CmpExpr) -> Result<CmpValue> {
        match expr {
            CmpExpr::Deref((b, _)) => {
                let v = self.simplify(b)?;
                match v {
                    CmpValue::U64(a) => {
                        let address = self
                            .processor_info_v2
                            .logical_to_physical(a, Access::Sim_Access_Read)?;
                        Ok(CmpValue::U64(read_phys_memory(
                            self.cpu,
                            address.address,
                            size_of::<u64>() as i32,
                        )?))
                    }
                    CmpValue::U32(a) => {
                        let address = self
                            .processor_info_v2
                            .logical_to_physical(a as u64, Access::Sim_Access_Read)?;
                        Ok(CmpValue::U32(read_phys_memory(
                            self.cpu,
                            address.address,
                            size_of::<u32>() as i32,
                        )? as u32))
                    }
                    _ => bail!("Invalid dereference size {:?}", v),
                }
            }
            CmpExpr::Reg((n, _)) => {
                let regno = self.int_register.get_number(n.as_raw_cstr()?)?;
                let value = self.int_register.read(regno)?;
                // PPE42 is 32-bit
                Ok(CmpValue::U32(value as u32))
            }
            CmpExpr::Add((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add(ru)))
                    }
                    _ => bail!("Cannot add non-matching types"),
                }
            }
            CmpExpr::Sub((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_sub(ru)))
                    }
                    _ => bail!("Cannot subtract non-matching types"),
                }
            }
            CmpExpr::Mul((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru)))
                    }
                    _ => bail!("Cannot multiply non-matching types"),
                }
            }
            CmpExpr::U8(u) => Ok(CmpValue::U8(*u)),
            CmpExpr::I8(i) => Ok(CmpValue::I8(*i)),
            CmpExpr::U16(u) => Ok(CmpValue::U16(*u)),
            CmpExpr::I16(i) => Ok(CmpValue::I16(*i)),
            CmpExpr::U32(u) => Ok(CmpValue::U32(*u)),
            CmpExpr::I32(i) => Ok(CmpValue::I32(*i)),
            CmpExpr::U64(u) => Ok(CmpValue::U64(*u)),
            CmpExpr::I64(i) => Ok(CmpValue::I64(*i)),
            CmpExpr::Addr(a) => Ok(CmpValue::U32(*a as u32)),
            _ => bail!("Unsupported expression {:?}", expr),
        }
    }
}

/// Minimal disassembler for PPE42
/// Since there's no PowerPC disassembler crate available, we implement basic functionality
pub(crate) struct Disassembler {
    last_bytes: Option<Vec<u8>>,
}

impl Disassembler {
    pub fn new() -> Self {
        Self { last_bytes: None }
    }

    /// Check if instruction is a branch instruction
    /// PowerPC branch instructions have opcode in bits 0-5
    fn is_branch_instruction(bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }
        let opcode = bytes[0] >> 2; // Top 6 bits
        // Opcodes 16-19: bc, sc, b, bclr/bcctr
        matches!(opcode, 16..=19)
    }

    /// Check if instruction is a call (branch and link)
    fn is_call_instruction(bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }
        let opcode = bytes[0] >> 2;
        let lk_bit = bytes[3] & 0x01; // Link bit is the last bit
        
        // Branch with link bit set
        (opcode == 18 || opcode == 16) && lk_bit == 1
    }

    /// Check if instruction is a return (bclr with specific conditions)
    fn is_return_instruction(bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }
        let opcode = bytes[0] >> 2;
        let extended = ((bytes[1] as u16) << 8) | (bytes[2] as u16);
        let xo = (extended >> 1) & 0x3FF; // Extended opcode
        
        // bclr (opcode 19, XO 16) - Branch Conditional to Link Register
        opcode == 19 && xo == 16
    }
}

impl Default for Disassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl TracerDisassembler for Disassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() < 4 {
            bail!("PowerPC instructions must be at least 4 bytes");
        }
        self.last_bytes = Some(bytes.to_vec());
        Ok(())
    }

    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String> {
        if bytes.len() < 4 {
            bail!("PowerPC instructions must be at least 4 bytes");
        }
        // Return hex representation since we don't have a full disassembler
        Ok(format!(
            "{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        ))
    }

    fn last_was_control_flow(&self) -> bool {
        if let Some(ref bytes) = self.last_bytes {
            Self::is_branch_instruction(bytes)
        } else {
            false
        }
    }

    fn last_was_call(&self) -> bool {
        if let Some(ref bytes) = self.last_bytes {
            Self::is_call_instruction(bytes)
        } else {
            false
        }
    }

    fn last_was_ret(&self) -> bool {
        if let Some(ref bytes) = self.last_bytes {
            Self::is_return_instruction(bytes)
        } else {
            false
        }
    }

    fn last_was_cmp(&self) -> bool {
        // Check if the last instruction was a comparison
        // This is a simplified implementation
        false
    }

    fn cmp(&self) -> Vec<CmpExpr> {
        // Basic comparison detection for PowerPC
        // This is a simplified implementation
        Vec::new()
    }

    fn cmp_type(&self) -> Vec<CmpType> {
        // Return a vector of comparison types
        vec![CmpType::Equal]
    }
}
