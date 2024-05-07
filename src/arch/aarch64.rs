// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture-specific implementation for ARM architecture

use crate::{
    tracer::{CmpExpr, CmpExprShift, CmpType, CmpValue, TraceEntry},
    traits::TracerDisassembler,
};
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
use yaxpeax_arm::armv8::a64::{InstDecoder, Instruction, Opcode, Operand, ShiftStyle, SizeCode};

use super::ArchitectureOperations;

pub(crate) struct AArch64ArchitectureOperations {
    cpu: *mut ConfObject,
    disassembler: Disassembler,
    int_register: IntRegisterInterface,
    processor_info_v2: ProcessorInfoV2Interface,
    cpu_instruction_query: CpuInstructionQueryInterface,
    cpu_instrumentation_subscribe: CpuInstrumentationSubscribeInterface,
    cycle: CycleInterface,
}

impl ArchitectureOperations for AArch64ArchitectureOperations {
    const INDEX_SELECTOR_REGISTER: &'static str = "x10";

    const ARGUMENT_REGISTER_0: &'static str = "x9";

    const ARGUMENT_REGISTER_1: &'static str = "x8";

    const ARGUMENT_REGISTER_2: &'static str = "x7";

    fn new(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        if arch == "aarch64"
            || arch == "arm64"
            || arch == "armv8"
            || arch == "armv9"
            || arch == "armv10"
        {
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
            bail!("Architecture {} is not aarch64", arch);
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

impl AArch64ArchitectureOperations {
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
            CmpExpr::Sub((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => Ok(CmpValue::U8(lu.wrapping_sub(ru))),
                    (CmpValue::U8(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U8(lu.wrapping_add_signed(-ru)))
                    }
                    (CmpValue::U8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U8((lu as u16).wrapping_sub(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U8((lu as u16).wrapping_add_signed(-ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U8((lu as u32).wrapping_sub(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U8((lu as u32).wrapping_add_signed(-ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U8((lu as u64).wrapping_sub(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U8((lu as u64).wrapping_add_signed(-ru) as u8))
                    }
                    (CmpValue::I8(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I8(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I8(lu), CmpValue::I8(ru)) => Ok(CmpValue::I8(lu.wrapping_sub(ru))),
                    (CmpValue::I8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_sub(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I8((lu as i32).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I8((lu as i32).wrapping_sub(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_add_unsigned(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_sub(ru) as i8))
                    }
                    (CmpValue::U16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_sub(ru as u16)))
                    }
                    (CmpValue::U16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add_signed(-ru as i16)))
                    }
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::U16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_add_signed(-ru)))
                    }
                    (CmpValue::U16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U16((lu as u32).wrapping_sub(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U16((lu as u32).wrapping_add_signed(-ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U16((lu as u64).wrapping_sub(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U16((lu as u64).wrapping_add_signed(-ru) as u16))
                    }
                    (CmpValue::I16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add_unsigned(ru as u16)))
                    }
                    (CmpValue::I16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_sub(ru as i16)))
                    }
                    (CmpValue::I16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::I16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_add_unsigned(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_sub(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_add_unsigned(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_sub(ru) as i16))
                    }
                    (CmpValue::U32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_sub(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(-ru as i32)))
                    }
                    (CmpValue::U32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_sub(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(-ru as i32)))
                    }
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::U32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_add_signed(-ru)))
                    }
                    (CmpValue::U32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U32((lu as u64).wrapping_sub(ru) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U32((lu as u64).wrapping_add_signed(-ru) as u32))
                    }
                    (CmpValue::I32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru as u32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_sub(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru as u32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_sub(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_add_unsigned(ru) as i32))
                    }
                    (CmpValue::I32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_sub(ru) as i32))
                    }
                    (CmpValue::U64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_sub(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(-ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_sub(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(-ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_sub(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(-ru as i64)))
                    }
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_sub(ru)))
                    }
                    (CmpValue::U64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_add_signed(-ru)))
                    }
                    (CmpValue::I64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_sub(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_sub(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru as u64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_sub(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_add_unsigned(ru)))
                    }
                    (CmpValue::I64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_sub(ru)))
                    }
                    _ => bail!("Cannot multiply non-integral types"),
                }
            }
            CmpExpr::Shift((shiftee, shifter, typ)) => {
                let shiftee = self.simplify(shiftee)?;
                let shifter = self.simplify(shifter)?;

                match (shiftee, shifter) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U8(lu), CmpValue::I8(ri)) => {
                        let ru = ri as u8;
                        match typ {
                            CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ru as u32))),
                            CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                            CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                            CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ru as u32))),
                        }
                    }
                    (CmpValue::U8(lu), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U8(lu), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U8(lu), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ru))),
                    },
                    (CmpValue::U8(lu), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U8(lu), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U8(lu), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U8(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U8(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U8(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ru))),
                    },
                    (CmpValue::I8(li), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I8(li), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I8(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I8(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I8(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ru))),
                    },
                    (CmpValue::U16(lu), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U16(lu), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U16(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U16(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U16(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ru))),
                    },
                    (CmpValue::I16(li), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I16(li), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I16(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I16(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I16(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ru))),
                    },
                    (CmpValue::U32(lu), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U32(lu), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U32(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U32(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U32(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ru))),
                    },
                    (CmpValue::I32(li), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I32(li), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I32(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I32(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I32(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ru))),
                    },
                    (CmpValue::U64(lu), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ru as u32))),
                    },
                    (CmpValue::U64(lu), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::U64(lu.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::U64(lu.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::U64(lu.rotate_right(ri as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::U8(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::I8(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::U16(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::I16(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::U32(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ru))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ru))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ru))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ru))),
                    },
                    (CmpValue::I64(li), CmpValue::I32(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ri as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::U64(ru)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ru as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ru as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ru as u32))),
                    },
                    (CmpValue::I64(li), CmpValue::I64(ri)) => match typ {
                        CmpExprShift::Lsl => Ok(CmpValue::I64(li.wrapping_shl(ri as u32))),
                        CmpExprShift::Lsr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Asr => Ok(CmpValue::I64(li.wrapping_shr(ri as u32))),
                        CmpExprShift::Ror => Ok(CmpValue::I64(li.rotate_right(ri as u32))),
                    },
                    _ => {
                        bail!("Cannot shift non-integral types");
                    }
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
    decoder: InstDecoder,
    last: Option<Instruction>,
}

impl Disassembler {
    pub fn new() -> Self {
        Self {
            decoder: InstDecoder::default(),
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
            // NOTE: This is imprecise on ARM because PC is not restricted
            // TODO: Are there any other control flow instructions?
            return matches!(
                last.opcode,
                Opcode::B
                    | Opcode::Bcc(_)
                    | Opcode::BR
                    | Opcode::BRAA
                    | Opcode::BRAAZ
                    | Opcode::BRABZ
                    | Opcode::CBNZ
                    | Opcode::CBZ
                    | Opcode::CCMN
                    | Opcode::CCMP
                    | Opcode::CSINC
                    | Opcode::CSINV
                    | Opcode::CSNEG
                    | Opcode::CSEL
                    | Opcode::TBNZ
                    | Opcode::TBZ
            );
        }

        false
    }

    // TODO: Make call/ret distinction more accurate, all three can ret/call far or near, but
    // there are semantic versions based on operands:
    // https://inst.eecs.berkeley.edu/~cs61c/fa20/pdfs/lectures/lec12-bw.pdf

    fn last_was_call(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return matches!(
                last.opcode,
                Opcode::BL | Opcode::BLRAA | Opcode::BLRAAZ | Opcode::BLRAB | Opcode::BLRABZ
            );
        }

        false
    }

    // https://quantum5.ca/2017/10/19/arm-ways-to-return/
    fn last_was_ret(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return matches!(last.opcode, Opcode::RET | Opcode::RETAA | Opcode::RETAB);
        }

        false
    }

    fn last_was_cmp(&self) -> bool {
        if let Some(last) = self.last.as_ref() {
            return self.last_was_control_flow()
                || matches!(
                    last.opcode,
                    Opcode::CMEQ
                        | Opcode::CMGE
                        | Opcode::CMGT
                        | Opcode::CMHI
                        | Opcode::CMHS
                        | Opcode::CMLE
                        | Opcode::CMLT
                        | Opcode::CMTST
                        | Opcode::CSEL
                        | Opcode::CSINC
                        | Opcode::CSINV
                        | Opcode::CSNEG
                );
        }

        false
    }

    fn cmp(&self) -> Vec<CmpExpr> {
        let mut cmp_exprs = Vec::new();
        if self.last_was_cmp() {
            if let Some(last) = self.last.as_ref() {
                for operand in &last.operands {
                    match operand {
                        Operand::Register(s, r) => match s {
                            SizeCode::X => cmp_exprs.push(CmpExpr::Reg((format!("x{r}"), 64))),
                            SizeCode::W => cmp_exprs.push(CmpExpr::Reg((format!("w{r}"), 32))),
                        },
                        Operand::RegisterPair(s, r) => match s {
                            SizeCode::X => cmp_exprs.push(CmpExpr::Reg((format!("x{r}"), 64))),
                            SizeCode::W => cmp_exprs.push(CmpExpr::Reg((format!("w{r}"), 32))),
                        },
                        Operand::RegisterOrSP(s, r) => match s {
                            SizeCode::X => cmp_exprs.push(CmpExpr::Reg((format!("x{r}"), 64))),
                            SizeCode::W => cmp_exprs.push(CmpExpr::Reg((format!("w{r}"), 32))),
                        },
                        Operand::Immediate(i) => cmp_exprs.push(CmpExpr::U32(*i)),
                        Operand::Imm64(i) => cmp_exprs.push(CmpExpr::U64(*i)),
                        Operand::Imm16(i) => cmp_exprs.push(CmpExpr::U16(*i)),
                        Operand::ImmediateDouble(_) => {}
                        Operand::ImmShift(i, s) => {
                            cmp_exprs.push(CmpExpr::U64((*i as u64).wrapping_shl((*s).into())))
                        }
                        Operand::ImmShiftMSL(i, s) => {
                            cmp_exprs.push(CmpExpr::U64((*i as u64).wrapping_shl((*s).into())))
                        }
                        Operand::RegShift(s, a, c, r) => {
                            let reg = Box::new(match c {
                                SizeCode::X => CmpExpr::Reg((format!("x{r}"), 64)),
                                SizeCode::W => CmpExpr::Reg((format!("w{r}"), 32)),
                            });

                            let typ = match s {
                                ShiftStyle::LSL => CmpExprShift::Lsl,
                                ShiftStyle::LSR => CmpExprShift::Lsr,
                                ShiftStyle::ASR => CmpExprShift::Asr,
                                ShiftStyle::ROR => CmpExprShift::Ror,
                                _ => continue,
                            };

                            match s {
                                ShiftStyle::LSL => cmp_exprs.push(CmpExpr::Deref((
                                    Box::new(CmpExpr::Shift((reg, Box::new(CmpExpr::U8(*a)), typ))),
                                    None,
                                ))),
                                ShiftStyle::LSR => cmp_exprs.push(CmpExpr::Deref((
                                    Box::new(CmpExpr::Shift((reg, Box::new(CmpExpr::U8(*a)), typ))),
                                    None,
                                ))),
                                ShiftStyle::ASR => cmp_exprs.push(CmpExpr::Deref((
                                    Box::new(CmpExpr::Shift((reg, Box::new(CmpExpr::U8(*a)), typ))),
                                    None,
                                ))),
                                ShiftStyle::ROR => cmp_exprs.push(CmpExpr::Deref((
                                    Box::new(CmpExpr::Shift((reg, Box::new(CmpExpr::U8(*a)), typ))),
                                    None,
                                ))),
                                _ => {}
                            }
                        }
                        Operand::RegRegOffset(r0, r1, c, s, a) => {
                            let reg0 = match c {
                                SizeCode::X => Box::new(CmpExpr::Reg((format!("x{r0}"), 64))),
                                SizeCode::W => Box::new(CmpExpr::Reg((format!("w{r0}"), 32))),
                            };
                            let reg1 = match c {
                                SizeCode::X => Box::new(CmpExpr::Reg((format!("x{r1}"), 64))),
                                SizeCode::W => Box::new(CmpExpr::Reg((format!("w{r1}"), 32))),
                            };
                            let typ = match s {
                                ShiftStyle::LSL => CmpExprShift::Lsl,
                                ShiftStyle::LSR => CmpExprShift::Lsr,
                                ShiftStyle::ASR => CmpExprShift::Asr,
                                ShiftStyle::ROR => CmpExprShift::Ror,
                                _ => continue,
                            };
                            let shift = Box::new(CmpExpr::U8(*a));

                            cmp_exprs.push(CmpExpr::Deref((
                                Box::new(CmpExpr::Add((
                                    reg0,
                                    Box::new(CmpExpr::Shift((reg1, shift, typ))),
                                ))),
                                None,
                            )))
                        }
                        Operand::RegPreIndex(r, i, _) => cmp_exprs.push(CmpExpr::Deref((
                            Box::new(CmpExpr::Add((
                                Box::new(CmpExpr::Reg((format!("x{r}"), 64))),
                                Box::new(CmpExpr::I32(*i)),
                            ))),
                            None,
                        ))),
                        Operand::RegPostIndex(r, _) => cmp_exprs.push(CmpExpr::Deref((
                            Box::new(CmpExpr::Reg((format!("x{r}"), 64))),
                            None,
                        ))),
                        Operand::RegPostIndexReg(r0, _) => cmp_exprs.push(CmpExpr::Deref((
                            Box::new(CmpExpr::Reg((format!("x{r0}"), 64))),
                            None,
                        ))),
                        _ => {}
                    }
                }
            }
        }

        cmp_exprs
    }

    // NOTE: CmpType is not well suited for arm
    fn cmp_type(&self) -> Vec<CmpType> {
        if self.last_was_cmp() {
            if let Some(last) = self.last.as_ref() {
                return match last.opcode {
                    Opcode::Bcc(_) => vec![CmpType::Equal, CmpType::Greater, CmpType::Lesser],
                    Opcode::CBNZ => vec![CmpType::Equal],
                    Opcode::CBZ => vec![CmpType::Equal],
                    Opcode::CCMN => vec![CmpType::Equal, CmpType::Greater, CmpType::Lesser],
                    Opcode::CCMP => vec![CmpType::Equal],
                    Opcode::CMEQ => vec![CmpType::Equal],
                    Opcode::CMGE => vec![CmpType::Greater, CmpType::Equal],
                    Opcode::CMGT => vec![CmpType::Greater],
                    Opcode::CMHI => vec![],
                    Opcode::CMHS => vec![],
                    Opcode::CMLE => vec![CmpType::Equal, CmpType::Lesser],
                    Opcode::CMLT => vec![CmpType::Lesser],
                    Opcode::CMTST => vec![CmpType::Equal],
                    Opcode::CSEL => vec![CmpType::Equal],
                    Opcode::CSINC => vec![CmpType::Equal],
                    Opcode::CSINV => vec![CmpType::Equal],
                    Opcode::CSNEG => vec![CmpType::Equal],
                    _ => vec![],
                };
            }
        }

        vec![]
    }
}
