// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Architecture-specific implementation for x86 architecture

use std::{ffi::CStr, mem::size_of, slice::from_raw_parts};

use super::{cached_instruction_interface, ArchitectureOperations};
use crate::{
    tracer::{CmpExpr, CmpType, CmpValue, DecodedComparison, DecodedInstruction, TraceEntry},
    traits::TracerDisassembler,
    CLASS_NAME,
};
use anyhow::{anyhow, bail, Error, Result};
use libafl::prelude::CmpValues;
use raw_cstr::AsRawCstr;
use simics::{
    api::{
        get_interface, get_object, read_phys_memory, sys::instruction_handle_t, Access, ConfObject,
        CpuCachedInstructionInterface, CpuInstructionQueryInterface,
        CpuInstrumentationSubscribeInterface, CycleInterface, IntRegisterInterface,
        ProcessorInfoV2Interface,
    },
    trace,
};
use yaxpeax_x86::protected_mode::{ConditionCode, InstDecoder, Instruction, Opcode, Operand};

pub(crate) struct X86ArchitectureOperations {
    cpu: *mut ConfObject,
    disassembler: Disassembler,
    int_register: IntRegisterInterface,
    processor_info_v2: ProcessorInfoV2Interface,
    cpu_instruction_query: CpuInstructionQueryInterface,
    #[allow(dead_code)]
    cpu_instrumentation_subscribe: CpuInstrumentationSubscribeInterface,
    cycle: CycleInterface,
    pub(crate) cached_instruction: Option<CpuCachedInstructionInterface>,
    cached_cmplog: bool,
}

impl ArchitectureOperations for X86ArchitectureOperations {
    const INDEX_SELECTOR_REGISTER: &'static str = "edi";
    const ARGUMENT_REGISTER_0: &'static str = "esi";
    const ARGUMENT_REGISTER_1: &'static str = "edx";
    const ARGUMENT_REGISTER_2: &'static str = "ecx";
    const POINTER_WIDTH_OVERRIDE: Option<i32> = Some(4);

    fn new(cpu: *mut ConfObject) -> Result<Self> {
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(cpu)?;

        let arch = unsafe { CStr::from_ptr(processor_info_v2.architecture()?) }
            .to_str()?
            .to_string();

        trace!(
            get_object(CLASS_NAME)?,
            "Checking whether cpu with architecutre {arch} is i386"
        );

        if arch == "x86-64" {
            // Check if the arch is actually x86-64, some x86-64 processors are actually
            // i386 under the hood
            let mut int_register: IntRegisterInterface = get_interface(cpu)?;
            let regs: Vec<u32> = int_register.all_registers()?.try_into()?;
            let reg_names: Vec<String> = regs
                .iter()
                .map(|r| {
                    int_register
                        .get_name(*r as i32)
                        .map_err(|e| anyhow!("Failed to get register name: {e}"))
                        .and_then(|n| {
                            unsafe { CStr::from_ptr(n) }
                                .to_str()
                                .map(|s| s.to_string())
                                .map_err(|e| anyhow!("Failed to convert string: {e}"))
                        })
                })
                .collect::<Result<Vec<_>>>()?;

            if reg_names.iter().any(|n| {
                [
                    "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "rip", "rsp", "rbp", "r8", "r9",
                    "r10", "r11", "r12", "r14", "r15",
                ]
                .contains(&n.to_ascii_lowercase().as_str())
            }) {
                bail!("Architecture is x86-64")
            } else if reg_names.iter().all(|n| {
                ![
                    "rax", "rbx", "rcx", "rdx", "rdi", "rsi", "rip", "rsp", "rbp", "r8", "r9",
                    "r10", "r11", "r12", "r14", "r15",
                ]
                .contains(&n.to_ascii_lowercase().as_str())
            }) {
                trace!(
                    get_object(CLASS_NAME)?,
                    "Architecture name is x86-64, but no 'r' registers found. Assuming i386"
                );
                Ok(Self {
                    cpu,
                    disassembler: Disassembler::new(),
                    int_register,
                    processor_info_v2,
                    cpu_instruction_query: get_interface(cpu)?,
                    cpu_instrumentation_subscribe: get_interface(cpu)?,
                    cycle: get_interface(cpu)?,
                    cached_instruction: None,
                    cached_cmplog: false,
                })
            } else {
                unreachable!("Register set must either contain a 64-bit register or no registers may be 64-bit");
            }
        } else if ["i386", "i486", "i586", "i686", "x86", "ia-32"]
            .contains(&arch.to_ascii_lowercase().as_str())
        {
            // No i386 processor will actually be x86-64 under the hood
            trace!(get_object(CLASS_NAME)?, "Architecture is i386");
            Ok(Self {
                cpu,
                disassembler: Disassembler::new(),
                int_register: get_interface(cpu)?,
                processor_info_v2,
                cpu_instruction_query: get_interface(cpu)?,
                cpu_instrumentation_subscribe: get_interface(cpu)?,
                cycle: get_interface(cpu)?,
                cached_instruction: None,
                cached_cmplog: false,
            })
        } else {
            bail!("Unsupported architecture {arch}");
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
            cached_instruction: None,
            cached_cmplog: false,
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
        if self.disassembler.last_was_cmp() {
            self.trace_decoded_cmp(&DecodedComparison {
                expressions: self.disassembler.cmp(),
                types: self.disassembler.cmp_type(),
            })
        } else {
            Ok(TraceEntry::default())
        }
    }
}
impl X86ArchitectureOperations {
    pub(crate) fn enable_cached_instrumentation(&mut self, cmplog: bool) -> Result<()> {
        self.cached_instruction = Some(cached_instruction_interface(self.cpu, cmplog)?);
        self.cached_cmplog = cmplog;
        Ok(())
    }

    pub(crate) fn decode_cached_instruction(
        &mut self,
        handle: *mut instruction_handle_t,
    ) -> Result<DecodedInstruction> {
        let bytes = self.cpu_instruction_query.get_instruction_bytes(handle)?;
        self.disassembler.classify(
            unsafe { from_raw_parts(bytes.data, bytes.size) },
            self.cached_cmplog,
        )
    }

    pub(crate) fn trace_decoded_cmp(
        &mut self,
        comparison: &DecodedComparison,
    ) -> Result<TraceEntry> {
        let pc = self.processor_info_v2.get_program_counter()?;
        let mut cmp_values = Vec::new();

        for expr in &comparison.expressions {
            if let Ok(value) = self.simplify(expr) {
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
                comparison.types.clone(),
                cmp_value.ok_or_else(|| anyhow!("No cmp value available"))?,
            ))
            .build())
    }
}

impl X86ArchitectureOperations {
    fn simplify(&mut self, expr: &CmpExpr) -> Result<CmpValue> {
        match expr {
            CmpExpr::Deref((expr, width)) => {
                let v = self.simplify(expr)?;

                match v {
                    CmpValue::U64(a) => {
                        let address = self
                            .processor_info_v2
                            .logical_to_physical(a, Access::Sim_Access_Read)?;
                        let casted = match width {
                            Some(1) => CmpValue::U8(
                                read_phys_memory(self.cpu, address.address, size_of::<u8>() as i32)
                                    .map_err(|e| {
                                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                                    })?
                                    .to_le_bytes()[0],
                            ),
                            Some(2) => CmpValue::U16(u16::from_le_bytes(
                                read_phys_memory(
                                    self.cpu,
                                    address.address,
                                    size_of::<u16>() as i32,
                                )
                                .map_err(|e| anyhow!("Error reading bytes from {:#x}: {}", a, e))?
                                .to_le_bytes()[0..size_of::<u16>()]
                                    .try_into()?,
                            )),
                            Some(4) => CmpValue::U32(u32::from_le_bytes(
                                read_phys_memory(
                                    self.cpu,
                                    address.address,
                                    size_of::<u32>() as i32,
                                )
                                .map_err(|e| anyhow!("Error reading bytes from {:#x}: {}", a, e))?
                                .to_le_bytes()[0..size_of::<u32>()]
                                    .try_into()?,
                            )),
                            Some(8) => CmpValue::U64(u64::from_le_bytes(
                                read_phys_memory(
                                    self.cpu,
                                    address.address,
                                    size_of::<u64>() as i32,
                                )
                                .map_err(|e| anyhow!("Error reading bytes from {:#x}: {}", a, e))?
                                .to_le_bytes(),
                            )),
                            _ => bail!("Can't cast to non-power-of-2 width {:?}", width),
                        };
                        Ok(casted)
                    }
                    _ => bail!("Can't dereference non-address"),
                }
            }
            CmpExpr::Reg((name, width)) => {
                let reg_number = self.int_register.get_number(name.as_raw_cstr()?)?;
                let value = self.int_register.read(reg_number).map_err(|e| {
                    anyhow!("Couldn't read register value for register {}: {}", name, e)
                })?;

                let casted = match width {
                    1 => CmpValue::U8(value.to_le_bytes()[0]),
                    2 => CmpValue::U16(u16::from_le_bytes(
                        value.to_le_bytes()[..size_of::<u16>()]
                            .try_into()
                            .map_err(|e| anyhow!("Error converting to u32 bytes: {}", e))?,
                    )),
                    4 => CmpValue::U32(u32::from_le_bytes(
                        value.to_le_bytes()[..size_of::<u32>()]
                            .try_into()
                            .map_err(|e| anyhow!("Error converting to u32 bytes: {}", e))?,
                    )),
                    8 => CmpValue::U64(u64::from_le_bytes(value.to_le_bytes())),
                    _ => bail!("Can't cast to non-power-of-2 width {}", width),
                };
                Ok(casted)
            }
            CmpExpr::Mul((l, r)) => {
                let lv = self.simplify(l)?;
                let rv = self.simplify(r)?;

                match (lv, rv) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => Ok(CmpValue::U8(lu.wrapping_mul(ru))),
                    (CmpValue::U8(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U8((lu as i32).wrapping_mul(ru as i32) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U8((lu as u16).wrapping_mul(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U8((lu as i32).wrapping_mul(ru as i32) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U8((lu as u32).wrapping_mul(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U8((lu as i32).wrapping_mul(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U8((lu as u64).wrapping_mul(ru) as u8))
                    }
                    (CmpValue::U8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U8((lu as i64).wrapping_mul(ru) as u8))
                    }
                    (CmpValue::I8(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_mul(ru as i16) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I8(ru)) => Ok(CmpValue::I8(lu.wrapping_mul(ru))),
                    (CmpValue::I8(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I8((lu as i32).wrapping_mul(ru as i32) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I8((lu as i16).wrapping_mul(ru) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_mul(ru as i64) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_mul(ru as i64) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_mul(ru as i64) as i8))
                    }
                    (CmpValue::I8(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I8((lu as i64).wrapping_mul(ru) as i8))
                    }
                    (CmpValue::U16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_mul(ru as u16)))
                    }
                    (CmpValue::U16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U16((lu as i32).wrapping_mul(ru as i32) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U16(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::U16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U16((lu as i32).wrapping_mul(ru as i32) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U16((lu as u32).wrapping_mul(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U16((lu as i32).wrapping_mul(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U16((lu as u64).wrapping_mul(ru) as u16))
                    }
                    (CmpValue::U16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U16((lu as i64).wrapping_mul(ru) as u16))
                    }
                    (CmpValue::I16(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_mul(ru as i16)))
                    }
                    (CmpValue::I16(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_mul(ru as i16)))
                    }
                    (CmpValue::I16(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_mul(ru as i32) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I16(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::I16(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_mul(ru as i64) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I16((lu as i32).wrapping_mul(ru) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_mul(ru as i64) as i16))
                    }
                    (CmpValue::I16(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I16((lu as i64).wrapping_mul(ru) as i16))
                    }
                    (CmpValue::U32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_mul(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U32((lu as i64).wrapping_mul(ru as i64) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_mul(ru as u32)))
                    }
                    (CmpValue::U32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U32((lu as i64).wrapping_mul(ru as i64) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U32(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::U32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U32((lu as i64).wrapping_mul(ru as i64) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U32((lu as u64).wrapping_mul(ru) as u32))
                    }
                    (CmpValue::U32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U32((lu as i64).wrapping_mul(ru) as u32))
                    }
                    (CmpValue::I32(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru as i32)))
                    }
                    (CmpValue::I32(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_mul(ru as i64) as i32))
                    }
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I32(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::I32(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_mul(ru as i64) as i32))
                    }
                    (CmpValue::I32(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I32((lu as i64).wrapping_mul(ru) as i32))
                    }
                    (CmpValue::U64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_mul(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::U64((lu as i64).wrapping_mul(ru as i64) as u64))
                    }
                    (CmpValue::U64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_mul(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::U64((lu as i64).wrapping_mul(ru as i64) as u64))
                    }
                    (CmpValue::U64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_mul(ru as u64)))
                    }
                    (CmpValue::U64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::U64((lu as i64).wrapping_mul(ru as i64) as u64))
                    }
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::U64(lu.wrapping_mul(ru)))
                    }
                    (CmpValue::U64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::U64((lu as i64).wrapping_mul(ru) as u64))
                    }
                    (CmpValue::I64(lu), CmpValue::U8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I8(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I16(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I32(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::U64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru as i64)))
                    }
                    (CmpValue::I64(lu), CmpValue::I64(ru)) => {
                        Ok(CmpValue::I64(lu.wrapping_mul(ru)))
                    }
                    _ => bail!("Cannot multiply non-integral types"),
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
            CmpExpr::U8(_)
            | CmpExpr::I8(_)
            | CmpExpr::U16(_)
            | CmpExpr::I16(_)
            | CmpExpr::U32(_)
            | CmpExpr::I32(_)
            | CmpExpr::U64(_)
            | CmpExpr::I64(_) => Ok(CmpValue::try_from(expr)?),
            CmpExpr::Addr(a) => {
                let address = self
                    .processor_info_v2
                    .logical_to_physical(*a, Access::Sim_Access_Read)?;
                let bytes: [u8; 8] =
                    read_phys_memory(self.cpu, address.address, size_of::<u64>() as i32)?
                        .to_le_bytes();
                Ok(CmpValue::U64(u64::from_le_bytes(bytes)))
            }
            _ => {
                // There are other types but they are never emitted on x86
                bail!("Unsupported expression type");
            }
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

impl Disassembler {
    pub(crate) fn classify(&mut self, bytes: &[u8], cmplog: bool) -> Result<DecodedInstruction> {
        self.disassemble(bytes)?;
        Ok(DecodedInstruction {
            branch: self.last_was_call() || self.last_was_control_flow() || self.last_was_ret(),
            comparison: (cmplog && self.last_was_cmp()).then(|| DecodedComparison {
                expressions: self.cmp(),
                types: self.cmp_type(),
            }),
        })
    }
}

#[cfg(test)]
mod cached_instruction_tests {
    use super::*;

    #[test]
    fn selects_32bit_branches_and_configured_comparisons_only() -> Result<()> {
        let mut decoder = Disassembler::new();
        for bytes in [
            &[0x75, 0x02][..],       // jne
            &[0xe8, 0, 0, 0, 0][..], // call
            &[0xc3][..],             // ret
            &[0xff, 0xe0][..],       // jmp eax
            &[0xe2, 0xfe][..],       // loop
            &[0xcd, 0x80][..],       // int 0x80
        ] {
            let instruction = decoder.classify(bytes, true)?;
            assert!(instruction.branch, "{bytes:x?}");
            assert!(instruction.comparison.is_none());
        }
        for bytes in [&[0x90][..], &[0x89, 0xd8][..], &[0x01, 0xd8][..]] {
            let instruction = decoder.classify(bytes, true)?;
            assert!(!instruction.branch, "{bytes:x?}");
            assert!(instruction.comparison.is_none());
        }
        for bytes in [
            &[0x39, 0xd8][..],
            &[0x85, 0xc0][..],
            &[0x83, 0x38, 0x07][..],
        ] {
            let instruction = decoder.classify(bytes, true)?;
            assert!(!instruction.branch, "{bytes:x?}");
            assert!(instruction.comparison.is_some(), "{bytes:x?}");
            assert!(decoder.classify(bytes, false)?.comparison.is_none());
        }
        assert!(decoder.classify(&[], true).is_err());
        Ok(())
    }

    #[test]
    fn sib_operand_multiplies_index_by_scale() -> Result<()> {
        let mut decoder = Disassembler::new();
        let scaled_index = || {
            Box::new(CmpExpr::Mul((
                Box::new(CmpExpr::Reg(("ecx".into(), 4))),
                Box::new(CmpExpr::U8(4)),
            )))
        };
        let base = || Box::new(CmpExpr::Reg(("esi".into(), 4)));
        let first_expression = |decoder: &mut Disassembler, bytes: &[u8]| {
            decoder
                .classify(bytes, true)?
                .comparison
                .and_then(|c| c.expressions.into_iter().next())
                .ok_or_else(|| anyhow!("Expected comparison"))
        };

        // cmp dword [esi + ecx * 4], eax
        assert_eq!(
            first_expression(&mut decoder, &[0x39, 0x04, 0x8e])?,
            CmpExpr::Deref((Box::new(CmpExpr::Add((base(), scaled_index()))), Some(4)))
        );
        // cmp dword [esi + ecx * 4 + 0x10], eax
        assert_eq!(
            first_expression(&mut decoder, &[0x39, 0x44, 0x8e, 0x10])?,
            CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Add((base(), scaled_index()))),
                    Box::new(CmpExpr::I32(0x10)),
                ))),
                Some(4)
            ))
        );
        Ok(())
    }

    #[test]
    fn comparison_metadata_survives_later_decodes() -> Result<()> {
        let mut decoder = Disassembler::new();
        let first = decoder
            .classify(&[0x83, 0x38, 0x07], true)? // cmp dword [eax], 7
            .comparison
            .ok_or_else(|| anyhow!("Expected comparison"))?;
        decoder.classify(&[0x39, 0xd1], true)?; // cmp ecx, edx
        let fresh = decoder
            .classify(&[0x83, 0x38, 0x07], true)?
            .comparison
            .ok_or_else(|| anyhow!("Expected comparison"))?;
        assert_eq!(first.expressions, fresh.expressions);
        assert!(matches!(first.expressions.first(), Some(CmpExpr::Deref(_))));
        Ok(())
    }
}

impl Default for Disassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<(&Operand, Option<u8>)> for CmpExpr {
    type Error = Error;

    fn try_from(value: (&Operand, Option<u8>)) -> Result<Self> {
        let width = value.1;
        let value = value.0;

        let expr = match value {
            Operand::ImmediateI8 { imm } => CmpExpr::I8(*imm),
            Operand::ImmediateU8 { imm } => CmpExpr::U8(*imm),
            Operand::ImmediateI16 { imm } => CmpExpr::I16(*imm),
            Operand::ImmediateU16 { imm } => CmpExpr::U16(*imm),
            Operand::ImmediateI32 { imm } => CmpExpr::I32(*imm),
            Operand::ImmediateU32 { imm } => CmpExpr::U32(*imm),
            Operand::Register { reg } => CmpExpr::Reg((reg.name().to_string(), reg.width())),
            Operand::AbsoluteU32 { addr } => CmpExpr::Addr(*addr as u64),
            Operand::MemDeref { base } => CmpExpr::Deref((
                Box::new(CmpExpr::Reg((base.name().to_string(), base.width()))),
                width,
            )),
            Operand::Disp { base, disp } => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Reg((base.name().to_string(), base.width()))),
                    Box::new(CmpExpr::I32(*disp)),
                ))),
                width,
            )),
            Operand::MemIndexScale { index, scale } => CmpExpr::Deref((
                Box::new(CmpExpr::Mul((
                    Box::new(CmpExpr::Reg((index.name().to_string(), index.width()))),
                    Box::new(CmpExpr::U8(*scale)),
                ))),
                width,
            )),
            Operand::MemIndexScaleDisp { index, scale, disp } => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Mul((
                        Box::new(CmpExpr::Reg((index.name().to_string(), index.width()))),
                        Box::new(CmpExpr::U8(*scale)),
                    ))),
                    Box::new(CmpExpr::I32(*disp)),
                ))),
                width,
            )),
            Operand::MemBaseIndexScale { base, index, scale } => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Reg((base.name().to_string(), base.width()))),
                    Box::new(CmpExpr::Mul((
                        Box::new(CmpExpr::Reg((index.name().to_string(), index.width()))),
                        Box::new(CmpExpr::U8(*scale)),
                    ))),
                ))),
                width,
            )),
            Operand::MemBaseIndexScaleDisp {
                base,
                index,
                scale,
                disp,
            } => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Add((
                        Box::new(CmpExpr::Reg((base.name().to_string(), base.width()))),
                        Box::new(CmpExpr::Mul((
                            Box::new(CmpExpr::Reg((index.name().to_string(), index.width()))),
                            Box::new(CmpExpr::U8(*scale)),
                        ))),
                    ))),
                    Box::new(CmpExpr::I32(*disp)),
                ))),
                width,
            )),
            _ => {
                bail!("Unsupported operand type for cmplog");
            }
        };
        Ok(expr)
    }
}

impl TracerDisassembler for Disassembler {
    /// Check if an instruction is a control flow instruction
    fn last_was_control_flow(&self) -> bool {
        if let Some(last) = self.last {
            return matches!(
                last.opcode(),
                Opcode::JMP
                    | Opcode::JA
                    | Opcode::JB
                    | Opcode::JG
                    | Opcode::JGE
                    | Opcode::JL
                    | Opcode::JLE
                    | Opcode::JNA
                    | Opcode::JNB
                    | Opcode::JNO
                    | Opcode::JNP
                    | Opcode::JNS
                    | Opcode::JNZ
                    | Opcode::JO
                    | Opcode::JP
                    | Opcode::JS
                    | Opcode::JZ
                    | Opcode::LOOP
                    | Opcode::LOOPNZ
                    | Opcode::LOOPZ
            );
        }

        false
    }

    /// Check if an instruction is a call instruction
    fn last_was_call(&self) -> bool {
        if let Some(last) = self.last {
            return matches!(
                last.opcode(),
                Opcode::CALL
                    | Opcode::CALLF
                    | Opcode::INT
                    | Opcode::INTO
                    | Opcode::SYSCALL
                    | Opcode::SYSENTER
            );
        }

        false
    }

    /// Check if an instruction is a ret instruction
    fn last_was_ret(&self) -> bool {
        if let Some(last) = self.last {
            return matches!(
                last.opcode(),
                Opcode::RETF
                    | Opcode::RETURN
                    | Opcode::IRET
                    | Opcode::IRETD
                    | Opcode::IRETQ
                    | Opcode::SYSRET
                    | Opcode::SYSEXIT
            );
        }

        false
    }

    /// Check if an instruction is a cmp instruction
    fn last_was_cmp(&self) -> bool {
        if let Some(last) = self.last {
            return matches!(
                last.opcode(),
                Opcode::CMP
                    | Opcode::CMPPD
                    | Opcode::CMPS
                    | Opcode::CMPSD
                    | Opcode::CMPSS
                    | Opcode::CMPXCHG16B
                    | Opcode::COMISD
                    | Opcode::COMISS
                    | Opcode::FCOM
                    | Opcode::FCOMI
                    | Opcode::FCOMIP
                    | Opcode::FCOMP
                    | Opcode::FCOMPP
                    | Opcode::FICOM
                    | Opcode::FICOMP
                    | Opcode::FTST
                    | Opcode::FUCOM
                    | Opcode::FUCOMI
                    | Opcode::FUCOMIP
                    | Opcode::FUCOMP
                    | Opcode::FXAM
                    | Opcode::PCMPEQB
                    | Opcode::PCMPEQD
                    | Opcode::PCMPEQW
                    | Opcode::PCMPGTB
                    | Opcode::PCMPGTD
                    | Opcode::PCMPGTQ
                    | Opcode::PCMPGTW
                    | Opcode::PMAXSB
                    | Opcode::PMAXSD
                    | Opcode::PMAXUD
                    | Opcode::PMAXUW
                    | Opcode::PMINSB
                    | Opcode::PMINSD
                    | Opcode::PMINUD
                    | Opcode::PMINUW
                    | Opcode::TEST
                    | Opcode::UCOMISD
                    | Opcode::UCOMISS
                    | Opcode::VPCMPB
                    | Opcode::VPCMPD
                    | Opcode::VPCMPQ
                    | Opcode::VPCMPUB
                    | Opcode::VPCMPUD
                    | Opcode::VPCMPUQ
                    | Opcode::VPCMPUW
                    | Opcode::VPCMPW
            );
        }

        false
    }

    fn disassemble(&mut self, bytes: &[u8]) -> Result<()> {
        if let Ok(insn) = self.decoder.decode_slice(bytes) {
            self.last = Some(insn);
        } else {
            bail!("Could not disassemble {:?}", bytes);
        }

        Ok(())
    }

    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String> {
        if let Ok(insn) = self.decoder.decode_slice(bytes) {
            Ok(insn.to_string())
        } else {
            bail!("Could not disassemble {:?}", bytes);
        }
    }

    fn cmp(&self) -> Vec<CmpExpr> {
        let mut cmp_exprs = Vec::new();
        if self.last_was_cmp() {
            if let Some(last) = self.last {
                for op_idx in 0..last.operand_count() {
                    let op = last.operand(op_idx);
                    let width = if let Some(width) = op.width() {
                        Some(width)
                    } else if let Some(width) = last.mem_size() {
                        width.bytes_size()
                    } else {
                        None
                    };
                    if let Ok(expr) = CmpExpr::try_from((&op, width)) {
                        cmp_exprs.push(expr);
                    }
                }
            }
        }
        cmp_exprs
    }

    fn cmp_type(&self) -> Vec<CmpType> {
        if self.last_was_cmp() {
            if let Some(last) = self.last {
                if let Some(condition) = last.opcode().condition() {
                    return match condition {
                        // Overflow
                        ConditionCode::O => vec![],
                        // No Overflow
                        ConditionCode::NO => vec![],
                        // Below
                        ConditionCode::B => vec![CmpType::Lesser],
                        // Above or Equal
                        ConditionCode::AE => vec![CmpType::Greater, CmpType::Equal],
                        // Zero
                        ConditionCode::Z => vec![],
                        // Not Zero
                        ConditionCode::NZ => vec![],
                        // Above
                        ConditionCode::A => vec![CmpType::Greater],
                        // Below or Equal
                        ConditionCode::BE => vec![CmpType::Lesser, CmpType::Equal],
                        // Signed
                        ConditionCode::S => vec![],
                        // Not Signed
                        ConditionCode::NS => vec![],
                        // Parity
                        ConditionCode::P => vec![],
                        // No Parity
                        ConditionCode::NP => vec![],
                        // Less
                        ConditionCode::L => vec![CmpType::Lesser],
                        // Greater or Equal
                        ConditionCode::GE => vec![CmpType::Greater, CmpType::Equal],
                        // Greater
                        ConditionCode::G => vec![CmpType::Greater],
                        // Less or Equal
                        ConditionCode::LE => vec![CmpType::Lesser, CmpType::Equal],
                    };
                }
            }
        }

        vec![]
    }
}
