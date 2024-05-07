// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture-specific implementation for RISCV architecture

use anyhow::{anyhow, bail, Result};
use libafl::prelude::CmpValues;
use raw_cstr::AsRawCstr;
use simics::api::{
    get_interface, read_phys_memory, sys::instruction_handle_t, Access, ConfObject,
    CpuInstructionQueryInterface, CpuInstrumentationSubscribeInterface, CycleInterface,
    IntRegisterInterface, ProcessorInfoV2Interface,
};
use std::{ffi::CStr, mem::size_of, slice::from_raw_parts};
use yaxpeax_arch::{Decoder, U8Reader};
use yaxpeax_riscv::{Instruction, Opcode, Operand, RiscVDecoder};

use crate::{
    tracer::{CmpExpr, CmpType, CmpValue, TraceEntry},
    traits::TracerDisassembler,
};

use super::ArchitectureOperations;

pub(crate) struct RISCVArchitectureOperations {
    cpu: *mut ConfObject,
    disassembler: Disassembler,
    int_register: IntRegisterInterface,
    processor_info_v2: ProcessorInfoV2Interface,
    cpu_instruction_query: CpuInstructionQueryInterface,
    cpu_instrumentation_subscribe: CpuInstrumentationSubscribeInterface,
    cycle: CycleInterface,
}

impl ArchitectureOperations for RISCVArchitectureOperations {
    const INDEX_SELECTOR_REGISTER: &'static str = "x10";

    const ARGUMENT_REGISTER_0: &'static str = "x11";

    const ARGUMENT_REGISTER_1: &'static str = "x12";

    const ARGUMENT_REGISTER_2: &'static str = "x13";

    fn new(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        if arch == "risc-v" || arch == "riscv" || arch == "riscv32" || arch == "riscv64" {
            Ok(Self {
                cpu,
                disassembler: Disassembler::new(),
                int_register: get_interface(cpu)?,
                processor_info_v2,
                cpu_instruction_query: get_interface(cpu)?,
                cpu_instrumentation_subscribe: get_interface(cpu)?,
                cycle: get_interface(cpu)?,
            })
        } else {
            bail!("Architecture {} is not risc-v", arch);
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
        let instruction_bytes = self
            .cpu_instruction_query
            .get_instruction_bytes(instruction_query)?;

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
        let instruction_bytes = self
            .cpu_instruction_query
            .get_instruction_bytes(instruction_query)?;
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

impl RISCVArchitectureOperations {
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
                        Ok(CmpValue::U64(read_phys_memory(
                            self.cpu,
                            address.address,
                            size_of::<u32>() as i32,
                        )?))
                    }
                    _ => bail!("Invalid dereference size {:?}", v),
                }
            }
            CmpExpr::Reg((n, _)) => {
                let regno = self.int_register.get_number(n.as_raw_cstr()?)?;
                let value = self.int_register.read(regno)?;
                if self.processor_info_v2.get_logical_address_width()? as u32 / u8::BITS == 8 {
                    Ok(CmpValue::U64(value))
                } else {
                    Ok(CmpValue::U32(value as u32))
                }
            }
            CmpExpr::Add((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => Ok(CmpValue::U8(lu.wrapping_add(ru))),
                    (CmpValue::U8(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U8(lu.wrapping_add_signed(ru)))
                    }
                    (CmpValue::U8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U8((lu as u16).wrapping_add(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U8((lu as u16).wrapping_add_signed(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U8((lu as u32).wrapping_add(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U8((lu as u32).wrapping_add_signed(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U8((lu as u64).wrapping_add(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U8((lu as u64).wrapping_add_signed(ru) as u8))
                    }
                    (CmpValue::I8(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I8(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I8(lu), CmpValue::I8(ru)) => Ok(CmpValue::I8(lu.wrapping_add(ru))),
                    (CmpValue::I8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_add(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I8((lu as i32).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I8((lu as i32).wrapping_add(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_add(ru) as i8))
                    }
                    (CmpValue::U16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add(ru as u16)))
                    }
                    (CmpValue::U16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add_signed(ru as i16)))
                    }
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add(ru)))
                    }
                    (CmpValue::U16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add_signed(ru)))
                    }
                    (CmpValue::U16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U16((lu as u32).wrapping_add(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U16((lu as u32).wrapping_add_signed(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U16((lu as u64).wrapping_add(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U16((lu as u64).wrapping_add_signed(ru) as u16))
                    }
                    (CmpValue::I16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add_unsigned(ru as u16)))
                    }
                    (CmpValue::I16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add(ru as i16)))
                    }
                    (CmpValue::I16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add(ru)))
                    }
                    (CmpValue::I16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_add_unsigned(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_add(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_add_unsigned(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_add(ru) as i16))
                    }
                    (CmpValue::U32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(ru as i32)))
                    }
                    (CmpValue::U32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(ru as i32)))
                    }
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add(ru)))
                    }
                    (CmpValue::U32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(ru)))
                    }
                    (CmpValue::U32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U32((lu as u64).wrapping_add(ru) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U32((lu as u64).wrapping_add_signed(ru) as u32))
                    }
                    (CmpValue::I32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru as u32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru as u32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_add_unsigned(ru) as i32))
                    }
                    (CmpValue::I32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_add(ru) as i32))
                    }
                    (CmpValue::U64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add(ru)))
                    }
                    (CmpValue::U64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(ru)))
                    }
                    (CmpValue::I64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add(ru)))
                    }
                    _ => bail!("Cannot multiply non-integral types"),
                }
            }
            CmpExpr::I16(i) => Ok(CmpValue::I16(*i)),
            CmpExpr::U32(u) => Ok(CmpValue::U32(*u)),
            CmpExpr::I32(i) => Ok(CmpValue::I32(*i)),
            _ => bail!("Unsupported expression {:?}", expr),
        }
    }
}

pub(crate) struct Disassembler {
    decoder: RiscVDecoder,
    last: Option<Instruction>,
}

impl Disassembler {
    pub fn new() -> Self {
        Self {
            decoder: RiscVDecoder::default(),
            last: None,
        }
    }
}

impl Default for Disassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl TracerDisassembler for Disassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()> {
        let mut r = U8Reader::new(bytes);

        if let Ok(insn) = self.decoder.decode(&mut r) {
            self.last = Some(insn);
        } else {
            bail!("Could not disassemble {:?}", bytes);
        }

        Ok(())
    }

    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String> {
        let mut r = U8Reader::new(bytes);

        if let Ok(insn) = self.decoder.decode(&mut r) {
            Ok(insn.to_string())
        } else {
            bail!("Could not disassemble {:?}", bytes);
        }
    }

    fn last_was_control_flow(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            if matches!(last.opcode(), |Opcode::BEQ| Opcode::BNE
                | Opcode::BLT
                | Opcode::BGE
                | Opcode::BLTU
                | Opcode::BGEU)
            {
                return true;
            }
        }

        false
    }

    // TODO: Make call/ret distinction more accurate, all three can ret/call far or near, but
    // there are semantic versions based on operands:
    // https://inst.eecs.berkeley.edu/~cs61c/fa20/pdfs/lectures/lec12-bw.pdf

    fn last_was_call(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return matches!(last.opcode(), Opcode::JALR | Opcode::JAL | Opcode::AUIPC);
        }

        false
    }

    fn last_was_ret(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return matches!(last.opcode(), Opcode::JALR | Opcode::JAL | Opcode::AUIPC);
        }

        false
    }

    fn last_was_cmp(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return matches!(
                last.opcode(),
                Opcode::SLT
                    | Opcode::SLTI
                    | Opcode::SLTU
                    | Opcode::SLTIU
                    | Opcode::BEQ
                    | Opcode::BNE
                    | Opcode::BGE
                    | Opcode::BLTU
                    | Opcode::BGEU
            );
        }

        false
    }

    fn cmp(&self) -> Vec<CmpExpr> {
        let mut cmp_exprs = Vec::new();
        if self.last_was_cmp() {
            if let Some(last) = self.last.as_ref() {
                for operand in last.operands() {
                    match operand {
                        Some(Operand::Reg(r)) => {
                            let regname = format!("x{}", r);
                            // NOTE: We don't give a width to regs here, it's defined by the
                            // arch subtype in the archops
                            cmp_exprs.push(CmpExpr::Reg((regname, 0)));
                        }
                        Some(Operand::Imm(i)) => {
                            // NOTE: Not technically correct, can be 12I/S or 20U
                            cmp_exprs.push(CmpExpr::I32(i));
                        }
                        Some(Operand::BaseOffset(b, o)) => {
                            let regname = format!("x{}", b);
                            cmp_exprs.push(CmpExpr::Deref((
                                Box::new(CmpExpr::Add((
                                    Box::new(CmpExpr::Reg((regname, 0))),
                                    Box::new(CmpExpr::I16(o)),
                                ))),
                                None,
                            )))
                        }
                        Some(Operand::LongImm(u)) => cmp_exprs.push(CmpExpr::U32(u)),
                        _ => {}
                    }
                }
            }
        }

        cmp_exprs
    }

    fn cmp_type(&self) -> Vec<CmpType> {
        if self.last_was_cmp() {
            if let Some(last) = self.last.as_ref() {
                return match last.opcode() {
                    Opcode::SLT => vec![CmpType::Lesser],
                    Opcode::SLTI => vec![CmpType::Lesser],
                    Opcode::SLTU => vec![CmpType::Lesser],
                    Opcode::SLTIU => vec![CmpType::Lesser],
                    Opcode::BEQ => vec![CmpType::Equal],
                    Opcode::BNE => vec![CmpType::Equal],
                    Opcode::BGE => vec![CmpType::Greater, CmpType::Equal],
                    Opcode::BLTU => vec![CmpType::Lesser],
                    Opcode::BGEU => vec![CmpType::Greater, CmpType::Equal],
                    _ => vec![],
                };
            }
        }

        vec![]
    }
}
