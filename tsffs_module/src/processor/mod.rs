// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Implements generic processor operations on the simulated CPU or CPUs
use anyhow::{anyhow, bail, Error, Result};

use libafl::prelude::CmpValues;
use simics_api::{
    attr_string, get_attribute, read_byte, write_byte, AttrValue, CachedInstructionHandle,
    ConfObject, CpuCachedInstruction, CpuInstructionQuery, CpuInstrumentationSubscribe, Cycle,
    InstructionHandle, IntRegister, ProcessorInfoV2,
};
use std::{collections::HashMap, ffi::c_void, mem::size_of};
use tracing::{error, trace};

pub(crate) mod disassembler;

use disassembler::x86_64::Disassembler as X86_64Disassembler;

use crate::traits::TracerDisassembler;

use self::disassembler::CmpExpr;

#[derive(Debug)]
pub enum CmpValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Expr(Box<CmpExpr>),
}

impl TryFrom<&CmpExpr> for CmpValue {
    type Error = Error;
    fn try_from(value: &CmpExpr) -> Result<Self> {
        Ok(match value {
            CmpExpr::U8(u) => CmpValue::U8(*u),
            CmpExpr::I8(i) => CmpValue::I8(*i),
            CmpExpr::U16(u) => CmpValue::U16(*u),
            CmpExpr::I16(i) => CmpValue::I16(*i),
            CmpExpr::U32(u) => CmpValue::U32(*u),
            CmpExpr::I32(i) => CmpValue::I32(*i),
            CmpExpr::U64(u) => CmpValue::U64(*u),
            CmpExpr::I64(i) => CmpValue::I64(*i),
            _ => bail!("Can't convert directly from non-integral expr"),
        })
    }
}

#[derive(Default, Debug)]
pub struct TraceResult {
    pub edge: Option<u64>,
    pub cmp: Option<(u64, CmpValues)>,
}

impl TraceResult {
    fn from_pc(value: Option<u64>) -> Self {
        Self {
            edge: value,
            cmp: None,
        }
    }

    fn from_pc_and_cmp_value(pc: u64, value: CmpValues) -> Self {
        Self {
            edge: None,
            cmp: Some((pc, value)),
        }
    }
}

pub struct Processor {
    number: i32,
    cpu: *mut ConfObject,
    arch: String,
    disassembler: Box<dyn TracerDisassembler>,
    cpu_instrumentation_subscribe: Option<CpuInstrumentationSubscribe>,
    cpu_instruction_query: Option<CpuInstructionQuery>,
    cpu_cached_instruction: Option<CpuCachedInstruction>,
    processor_info_v2: Option<ProcessorInfoV2>,
    int_register: Option<IntRegister>,
    cycle: Option<Cycle>,
    reg_numbers: HashMap<String, i32>,
}

impl Processor {
    pub fn number(&self) -> i32 {
        self.number
    }

    pub fn cpu(&self) -> *mut ConfObject {
        self.cpu
    }

    pub fn arch(&self) -> String {
        self.arch.clone()
    }
}

impl Processor {
    pub fn try_new(number: i32, cpu: *mut ConfObject) -> Result<Self> {
        let arch = attr_string(get_attribute(cpu, "architecture")?)?;

        let disassembler = match arch.as_str() {
            "x86-64" => Box::new(X86_64Disassembler::new()),
            _ => {
                bail!("Unsupported architecture {}", arch)
            }
        };

        Ok(Self {
            number,
            cpu,
            arch,
            disassembler,
            cpu_instrumentation_subscribe: None,
            cpu_instruction_query: None,
            cpu_cached_instruction: None,
            processor_info_v2: None,
            int_register: None,
            cycle: None,
            reg_numbers: HashMap::new(),
        })
    }

    pub fn try_with_cpu_instrumentation_subscribe(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_instrumentation_subscribe =
            Some(CpuInstrumentationSubscribe::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_instruction_query(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_instruction_query = Some(CpuInstructionQuery::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cpu_cached_instruction(
        mut self,
        processor_attr: *mut AttrValue,
    ) -> Result<Self> {
        self.cpu_cached_instruction = Some(CpuCachedInstruction::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_processor_info_v2(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.processor_info_v2 = Some(ProcessorInfoV2::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_int_register(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.int_register = Some(IntRegister::try_new(processor_attr)?);
        Ok(self)
    }

    pub fn try_with_cycle(mut self, processor_attr: *mut AttrValue) -> Result<Self> {
        self.cycle = Some(Cycle::try_new(processor_attr)?);
        Ok(self)
    }
}

impl Processor {
    pub fn register_instruction_before_cb<D>(
        &mut self,
        // cpu: *mut ConfObject,
        cb: unsafe extern "C" fn(
            *mut ConfObject,
            *mut ConfObject,
            *mut InstructionHandle,
            *mut c_void,
        ),
        user_data: Option<D>,
    ) -> Result<()>
    where
        D: Into<*mut c_void>,
    {
        if let Some(cpu_instrumentation_subscribe) = self.cpu_instrumentation_subscribe.as_mut() {
            cpu_instrumentation_subscribe
                .register_instruction_before_cb(self.cpu, cb, user_data)?;
        }

        Ok(())
    }

    pub fn register_cached_instruction_cb<D>(
        &mut self,
        // cpu: *mut ConfObject,
        cb: unsafe extern "C" fn(
            *mut ConfObject,
            *mut ConfObject,
            *mut CachedInstructionHandle,
            *mut InstructionHandle,
            *mut c_void,
        ),
        user_data: Option<D>,
    ) -> Result<()>
    where
        D: Into<*mut c_void>,
    {
        if let Some(cpu_instrumentation_subscribe) = self.cpu_instrumentation_subscribe.as_mut() {
            cpu_instrumentation_subscribe
                .register_cached_instruction_cb(self.cpu, cb, user_data)?;
        }

        Ok(())
    }

    /// This expression can only be nested approximately 4 deep, so we do this
    /// reduction recursively
    ///
    /// We don't implement this as try_from because we need to read mem and regs
    pub fn simplify(&mut self, expr: &CmpExpr) -> Result<CmpValue> {
        trace!("Reducing {:?}", expr);
        match expr {
            CmpExpr::Deref((expr, width)) => {
                let v = self.simplify(expr)?;

                match v {
                    CmpValue::U64(a) => {
                        let casted = match width {
                            Some(1) => CmpValue::U8(u8::from_le_bytes(
                                self.read_bytes(a, size_of::<u8>())
                                    .map_err(|e| {
                                        error!("Error reading bytes from {:#x}: {}", a, e);
                                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                                    })?
                                    .try_into()
                                    .map_err(|v| {
                                        error!("Error converting u64 vec {:?} to byte array", v);
                                        anyhow!("Error converting u64 vec {:?} to byte array", v)
                                    })?,
                            )),
                            Some(2) => CmpValue::U16(u16::from_le_bytes(
                                self.read_bytes(a, size_of::<u16>())
                                    .map_err(|e| {
                                        error!("Error reading bytes from {:#x}: {}", a, e);
                                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                                    })?
                                    .try_into()
                                    .map_err(|v| {
                                        error!("Error converting u64 vec {:?} to byte array", v);
                                        anyhow!("Error converting u64 vec {:?} to byte array", v)
                                    })?,
                            )),
                            Some(4) => CmpValue::U32(u32::from_le_bytes(
                                self.read_bytes(a, size_of::<u32>())
                                    .map_err(|e| {
                                        error!("Error reading bytes from {:#x}: {}", a, e);
                                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                                    })?
                                    .try_into()
                                    .map_err(|v| {
                                        error!("Error converting u64 vec {:?} to byte array", v);
                                        anyhow!("Error converting u64 vec {:?} to byte array", v)
                                    })?,
                            )),
                            Some(8) => CmpValue::U64(u64::from_le_bytes(
                                self.read_bytes(a, size_of::<u64>())
                                    .map_err(|e| {
                                        error!("Error reading bytes from {:#x}: {}", a, e);
                                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                                    })?
                                    .try_into()
                                    .map_err(|v| {
                                        error!("Error converting u64 vec {:?} to byte array", v);
                                        anyhow!("Error converting u64 vec {:?} to byte array", v)
                                    })?,
                            )),
                            _ => bail!("Can't cast to non-power-of-2 width {:?}", width),
                        };
                        Ok(casted)
                    }
                    _ => bail!("Can't dereference non-address"),
                }
            }
            CmpExpr::Reg((name, width)) => {
                let value = self.get_reg_value(name).map_err(|e| {
                    error!("Couldn't read register value for register {}: {}", name, e);
                    anyhow!("Couldn't read register value for register {}: {}", name, e)
                })?;

                let casted = match width {
                    1 => CmpValue::U8(value.to_le_bytes()[0]),
                    2 => CmpValue::U16(u16::from_le_bytes(
                        value.to_le_bytes()[..size_of::<u16>()]
                            .try_into()
                            .map_err(|e| {
                                error!("Error converting to u32 bytes: {}", e);
                                anyhow!("Error converting to u32 bytes: {}", e)
                            })?,
                    )),
                    4 => CmpValue::U32(u32::from_le_bytes(
                        value.to_le_bytes()[..size_of::<u32>()]
                            .try_into()
                            .map_err(|e| {
                                error!("Error converting to u32 bytes: {}", e);
                                anyhow!("Error converting to u32 bytes: {}", e)
                            })?,
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
            | CmpExpr::I64(_) => CmpValue::try_from(expr),
            CmpExpr::Addr(a) => {
                let bytes: [u8; 8] = self
                    .read_bytes(*a, size_of::<u64>())
                    .map_err(|e| {
                        error!("Error reading bytes from {:#x}: {}", a, e);
                        anyhow!("Error reading bytes from {:#x}: {}", a, e)
                    })?
                    .try_into()
                    .map_err(|v| {
                        error!("Error converting u64 vec {:?} to byte array", v);
                        anyhow!("Error converting u64 vec {:?} to byte array", v)
                    })?;
                Ok(CmpValue::U64(u64::from_le_bytes(bytes)))
            }
        }
    }

    pub fn trace(
        &mut self,
        // cpu: *mut ConfObject,
        instruction_query: *mut InstructionHandle,
    ) -> Result<TraceResult> {
        if let Some(cpu_instruction_query) = self.cpu_instruction_query.as_mut() {
            let bytes = cpu_instruction_query.get_instruction_bytes(self.cpu, instruction_query)?;
            self.disassembler.disassemble(bytes)?;

            if self.disassembler.last_was_call()?
                || self.disassembler.last_was_control_flow()?
                || self.disassembler.last_was_ret()?
            {
                if let Some(processor_info_v2) = self.processor_info_v2.as_mut() {
                    Ok(TraceResult::from_pc(
                        processor_info_v2.get_program_counter(self.cpu).ok(),
                    ))
                } else {
                    bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
                }
            } else if self.disassembler.last_was_cmp()? {
                let pc = if let Some(processor_info_v2) = self.processor_info_v2.as_mut() {
                    processor_info_v2.get_program_counter(self.cpu)?
                } else {
                    bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
                };

                let mut cmp_values = Vec::new();

                if let Ok(cmp) = self.disassembler.cmp() {
                    for expr in &cmp {
                        match self.simplify(expr) {
                            Ok(value) => cmp_values.push(value),
                            Err(e) => {
                                error!("Error reducing expression {:?}: {}", expr, e);
                            }
                        }
                    }
                }

                trace!("Got cmp values: {:?}", cmp_values);

                let cmp_value = if let (Some(l), Some(r)) = (cmp_values.get(0), cmp_values.get(1)) {
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

                Ok(TraceResult::from_pc_and_cmp_value(
                    pc,
                    cmp_value.ok_or_else(|| anyhow!("No cmp value available"))?,
                ))
            } else {
                Ok(TraceResult::default())
            }
        } else {
            bail!("No CpuInstructionQuery interface registered in processor. Try building with `try_with_cpu_instruction_query`");
        }
    }

    pub fn get_reg_value<S>(&mut self, reg: S) -> Result<u64>
    where
        S: AsRef<str>,
    {
        let int_register = if let Some(int_register) = self.int_register.as_ref() {
            int_register
        } else {
            bail!("No IntRegister interface registered in processor. Try building with `try_with_int_register`");
        };

        let reg_number = if let Some(reg_number) = self.reg_numbers.get(reg.as_ref()) {
            *reg_number
        } else {
            let reg_name = reg.as_ref().to_string();
            let reg_number = int_register.get_number(self.cpu, reg)?;
            self.reg_numbers.insert(reg_name, reg_number);
            reg_number
        };

        int_register.read(self.cpu, reg_number)
    }

    pub fn set_reg_value<S>(&mut self, reg: S, val: u64) -> Result<()>
    where
        S: AsRef<str>,
    {
        let int_register = if let Some(int_register) = self.int_register.as_ref() {
            int_register
        } else {
            bail!("No IntRegister interface registered in processor. Try building with `try_with_int_register`");
        };

        let reg_number = if let Some(reg_number) = self.reg_numbers.get(reg.as_ref()) {
            *reg_number
        } else {
            let reg_name = reg.as_ref().to_string();
            let reg_number = int_register.get_number(self.cpu, reg)?;
            self.reg_numbers.insert(reg_name, reg_number);
            reg_number
        };

        int_register.write(self.cpu, reg_number, val)
    }

    pub fn write_bytes(&self, logical_address_start: u64, bytes: &[u8]) -> Result<()> {
        let processor_info_v2 = if let Some(processor_info_v2) = self.processor_info_v2.as_ref() {
            processor_info_v2
        } else {
            bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
        };

        let physical_memory = processor_info_v2.get_physical_memory(self.cpu)?;

        for (i, byte) in bytes.iter().enumerate() {
            let logical_address = logical_address_start + i as u64;
            let physical_address =
                processor_info_v2.logical_to_physical(self.cpu, logical_address)?;
            write_byte(physical_memory, physical_address.address, *byte);
            // let written = read_byte(physical_memory, physical_address);
            // ensure!(written == *byte, "Did not read back same written byte");
        }

        Ok(())
    }

    pub fn read_bytes(&self, logical_address_start: u64, size: usize) -> Result<Vec<u8>> {
        let processor_info_v2 = if let Some(processor_info_v2) = self.processor_info_v2.as_ref() {
            processor_info_v2
        } else {
            bail!("No ProcessorInfoV2 interface registered in processor. Try building with `try_with_processor_info_v2`");
        };

        let physical_memory = processor_info_v2.get_physical_memory(self.cpu)?;

        let mut bytes = Vec::new();

        for i in 0..size {
            let logical_address = logical_address_start + i as u64;
            let physical_address =
                processor_info_v2.logical_to_physical(self.cpu, logical_address)?;
            bytes.push(read_byte(physical_memory, physical_address.address));
            // let written = read_byte(physical_memory, physical_address);
            // ensure!(written == *byte, "Did not read back same written byte");
        }

        Ok(bytes)
    }
}
