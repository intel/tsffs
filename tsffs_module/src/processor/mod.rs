//! Implements generic processor operations on the simulated CPU or CPUs
use anyhow::{anyhow, bail, Error, Result};

use simics_api::{
    attr_string, get_attribute, read_byte, write_byte, AttrValue, CachedInstructionHandle,
    ConfObject, CpuCachedInstruction, CpuInstructionQuery, CpuInstrumentationSubscribe, Cycle,
    InstructionHandle, IntRegister, ProcessorInfoV2,
};
use std::{collections::HashMap, ffi::c_void, mem::size_of};
use tracing::trace;

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
    pub cmp: Option<Vec<CmpValue>>,
}

impl TraceResult {
    fn from_pc(value: Option<u64>) -> Self {
        Self {
            edge: value,
            cmp: None,
        }
    }

    fn from_cmp_values(values: Vec<CmpValue>) -> Self {
        Self {
            edge: None,
            cmp: Some(values),
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
    pub fn reduce(&mut self, expr: &CmpExpr) -> Result<CmpValue> {
        match expr {
            CmpExpr::Deref(e) => {
                let v = self.reduce(e)?;

                match v {
                    CmpValue::U64(a) => {
                        let bytes: [u8; 8] = self
                            .read_bytes(a, size_of::<u64>())?
                            .try_into()
                            .map_err(|_| anyhow!("Error converting u64 to bytes"))?;
                        Ok(CmpValue::U64(u64::from_le_bytes(bytes)))
                    }
                    _ => bail!("Can't dereference non-address"),
                }
            }
            CmpExpr::Reg(r) => Ok(CmpValue::U64(self.get_reg_value(r)?)),
            CmpExpr::Mul(l, r) => {
                let lv = self.reduce(l)?;
                let rv = self.reduce(r)?;
                match (lv, rv) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => Ok(CmpValue::U8(lu * ru)),
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => Ok(CmpValue::U16(lu * ru)),
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => Ok(CmpValue::U32(lu * ru)),
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => Ok(CmpValue::U64(lu * ru)),
                    (CmpValue::I8(lu), CmpValue::I8(ru)) => Ok(CmpValue::I8(lu * ru)),
                    (CmpValue::I16(lu), CmpValue::I16(ru)) => Ok(CmpValue::I16(lu * ru)),
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => Ok(CmpValue::I32(lu * ru)),
                    (CmpValue::I64(lu), CmpValue::I64(ru)) => Ok(CmpValue::I64(lu * ru)),
                    _ => bail!("Can't multiply non-values"),
                }
            }
            CmpExpr::Add(l, r) => {
                let lv = self.reduce(l)?;
                let rv = self.reduce(r)?;
                match (lv, rv) {
                    (CmpValue::U8(lu), CmpValue::U8(ru)) => Ok(CmpValue::U8(lu + ru)),
                    (CmpValue::U16(lu), CmpValue::U16(ru)) => Ok(CmpValue::U16(lu + ru)),
                    (CmpValue::U32(lu), CmpValue::U32(ru)) => Ok(CmpValue::U32(lu + ru)),
                    (CmpValue::U64(lu), CmpValue::U64(ru)) => Ok(CmpValue::U64(lu + ru)),
                    (CmpValue::I8(lu), CmpValue::I8(ru)) => Ok(CmpValue::I8(lu + ru)),
                    (CmpValue::I16(lu), CmpValue::I16(ru)) => Ok(CmpValue::I16(lu + ru)),
                    (CmpValue::I32(lu), CmpValue::I32(ru)) => Ok(CmpValue::I32(lu + ru)),
                    (CmpValue::I64(lu), CmpValue::I64(ru)) => Ok(CmpValue::I64(lu + ru)),
                    _ => bail!("Can't multiply non-values"),
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
                    .read_bytes(*a, size_of::<u64>())?
                    .try_into()
                    .map_err(|_| anyhow!("Error converting u64 to bytes"))?;
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
                let mut cmp_values = Vec::new();
                if let Ok(cmp) = self.disassembler.cmp() {
                    for expr in &cmp {
                        if let Ok(val) = self.reduce(expr) {
                            cmp_values.push(val);
                        }
                    }
                }
                Ok(TraceResult::from_cmp_values(cmp_values))
            } else {
                Ok(TraceResult::default())
            }
        } else {
            bail!("No CpuInstructionQuery interface registered in processor. Try building with `try_with_cpu_instruction_query`");
        }
    }

    pub fn get_reg_value<S: AsRef<str>>(&mut self, reg: S) -> Result<u64> {
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

    pub fn set_reg_value<S: AsRef<str>>(&mut self, reg: S, val: u64) -> Result<()> {
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
