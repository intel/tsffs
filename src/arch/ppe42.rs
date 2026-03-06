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
    #[allow(dead_code)]
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
    // PPE42 is Big Endian (PowerPC)
    const BYTE_ORDER_BIG_ENDIAN: bool = true;

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
                // Register vs immediate forms (cmpwi, cmplwi, cmpwibc): the register operand
                // is always U32. cmpwi sign-extends SI to 32 bits; cmplwi/cmpwibc zero-extend
                // UI/UIX to 32 bits.
                (CmpValue::U32(l), CmpValue::I16(r)) => {
                    Some(CmpValues::U32((*l, *r as i32 as u32)))
                }
                (CmpValue::U32(l), CmpValue::U16(r)) => Some(CmpValues::U32((*l, *r as u32))),
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

/// Decode a big-endian 4-byte instruction word from a byte slice.
/// PPE42 instructions are always 4 bytes, big-endian.
#[inline]
fn decode_insn(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Extract the 6-bit primary opcode from a decoded instruction word.
/// Bits 0-5 (IBM bit numbering, MSB first), i.e. bits 31-26 in standard notation.
#[inline]
fn primary_opcode(insn: u32) -> u32 {
    insn >> 26
}

/// Extract the 10-bit extended opcode (XO) from bits 21-30 of the instruction word.
/// Used by opcode-31 (X-form) and opcode-19 (XL-form) instructions.
#[inline]
fn extended_opcode(insn: u32) -> u32 {
    (insn >> 1) & 0x3FF
}

/// Extract the RA field (bits 11-15, IBM numbering) as a register number 0-31.
#[inline]
fn field_ra(insn: u32) -> u32 {
    (insn >> 16) & 0x1F
}

/// Extract the RB field (bits 16-20, IBM numbering) as a register number 0-31.
#[inline]
fn field_rb(insn: u32) -> u32 {
    (insn >> 11) & 0x1F
}

/// Extract the 16-bit SI (signed immediate) field from bits 16-31, sign-extended to i32.
#[inline]
fn field_si(insn: u32) -> i32 {
    (insn & 0xFFFF) as i16 as i32
}

/// Extract the 16-bit UI (unsigned immediate) field from bits 16-31.
#[inline]
fn field_ui(insn: u32) -> u32 {
    insn & 0xFFFF
}

/// Extract the LK bit (bit 31, IBM numbering) — link bit.
#[inline]
fn lk_bit(insn: u32) -> bool {
    insn & 1 != 0
}

/// The FCB (fused compare-branch) extended opcode lives in bits 6-7 (IBM numbering),
/// i.e. bits 25-24 in standard notation.
/// cmpwbc  = FCBXO 0  (op1, bit6=0, bit7=0)
/// cmplwbc = FCBXO 1  (op1, bit6=0, bit7=1)
/// cmpwibc = FCBXO 2  (op1, bit6=1, bit7=0)
/// bnbw/bnbwi/clrbwbc/clrbwibc = FCBXO 3 (op1, bit6=1, bit7=1)
#[inline]
fn fcb_xo(insn: u32) -> u32 {
    (insn >> 24) & 0x3
}

/// Classify a PPE42 instruction word into an internal kind for the disassembler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InsnKind {
    /// `b` / `bl` / `ba` / `bla`  — unconditional branch (opcode 18)
    B,
    /// `bc` / `bcl` / …           — branch conditional (opcode 16)
    Bc,
    /// `bclr` / `blr` / …         — branch to LR (opcode 19, XO 16)
    Bclr,
    /// `bcctr` / `bctr` / …       — branch to CTR (opcode 19, XO 528)
    Bcctr,
    /// `rfi` — return from interrupt (opcode 19, XO 50)
    Rfi,
    /// `cmpw`  — signed reg-reg compare  (opcode 31, XO 0)
    Cmpw,
    /// `cmplw` — unsigned reg-reg compare (opcode 31, XO 32)
    Cmplw,
    /// `cmpwi`  — signed reg-imm compare  (opcode 11)
    Cmpwi,
    /// `cmplwi` — unsigned reg-imm compare (opcode 10)
    Cmplwi,
    /// `cmpwbc`  — PPE42 fused signed cmp+branch (opcode 1, FCBXO 0)
    Cmpwbc,
    /// `cmplwbc` — PPE42 fused unsigned cmp+branch (opcode 1, FCBXO 1)
    Cmplwbc,
    /// `cmpwibc` — PPE42 fused signed-immediate cmp+branch (opcode 1, FCBXO 2)
    Cmpwibc,
    /// Other opcode 1 instructions (bnbw, clrbwbc, etc.) — still control flow
    OtherFcb,
    /// Everything else
    Other,
}

impl InsnKind {
    fn decode(insn: u32) -> Self {
        match primary_opcode(insn) {
            18 => Self::B,
            16 => Self::Bc,
            19 => match extended_opcode(insn) {
                16  => Self::Bclr,
                528 => Self::Bcctr,
                50  => Self::Rfi,
                _   => Self::Other,
            },
            31 => match extended_opcode(insn) {
                0  => Self::Cmpw,
                32 => Self::Cmplw,
                _  => Self::Other,
            },
            11 => Self::Cmpwi,
            10 => Self::Cmplwi,
            1 => match fcb_xo(insn) {
                0 => Self::Cmpwbc,
                1 => Self::Cmplwbc,
                2 => Self::Cmpwibc,
                _ => Self::OtherFcb,
            },
            _ => Self::Other,
        }
    }
}

/// Disassembler for PPE42. Decodes the 32-bit instruction word from raw bytes and
/// classifies it into the categories required by the tsffs tracer (branch, call,
/// return, compare).
pub(crate) struct Disassembler {
    last: Option<(u32, InsnKind)>,
}

impl Disassembler {
    pub fn new() -> Self {
        Self { last: None }
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
            bail!("PPE42 instructions are always 4 bytes, got {}", bytes.len());
        }
        let insn = decode_insn(bytes);
        self.last = Some((insn, InsnKind::decode(insn)));
        Ok(())
    }

    fn disassemble_to_string(&mut self, bytes: &[u8]) -> Result<String> {
        if bytes.len() < 4 {
            bail!("PPE42 instructions are always 4 bytes, got {}", bytes.len());
        }
        let insn = decode_insn(bytes);
        let kind = InsnKind::decode(insn);
        Ok(format!("{:08x} /* {:?} */", insn, kind))
    }

    /// Control flow: any branch instruction that is not a call and not a return.
    /// For the fuzzer's edge-coverage purposes we fire on all branches that
    /// change PC non-linearly, which is every branch form PPE42 defines.
    fn last_was_control_flow(&self) -> bool {
        if let Some((insn, kind)) = self.last {
            match kind {
                // Plain unconditional branch (b/ba): always control flow when LK=0
                InsnKind::B => !lk_bit(insn),
                // Conditional branches and fused compare-branches: always control flow when LK=0
                InsnKind::Bc
                | InsnKind::Bcctr
                | InsnKind::Cmpwbc
                | InsnKind::Cmplwbc
                | InsnKind::Cmpwibc
                | InsnKind::OtherFcb => !lk_bit(insn),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Call: any branch-with-link that saves the return address into LR.
    /// Covers bl (B+LK), bcl (Bc+LK), bcctrl (Bcctr+LK), bclrl (Bclr+LK),
    /// and the fused compare-branch-link forms.
    fn last_was_call(&self) -> bool {
        if let Some((insn, kind)) = self.last {
            match kind {
                InsnKind::B
                | InsnKind::Bc
                | InsnKind::Bclr
                | InsnKind::Bcctr
                | InsnKind::Cmpwbc
                | InsnKind::Cmplwbc
                | InsnKind::Cmpwibc
                | InsnKind::OtherFcb => lk_bit(insn),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Return: `blr` / `bclr` family — branch to LR without the link bit
    /// (with the link bit it is `blrl`, which is a call-then-return and still
    /// counts as a call for coverage purposes, so we only fire here when LK=0).
    /// `rfi` (return from interrupt) is also a return; it has no LK bit.
    fn last_was_ret(&self) -> bool {
        matches!(
            self.last,
            Some((insn, InsnKind::Bclr)) if !lk_bit(insn)
        ) || matches!(self.last, Some((_, InsnKind::Rfi)))
    }

    /// Compare: any instruction whose primary purpose is a comparison and which
    /// sets CR[CR0] flags used by subsequent (or fused) branches.
    fn last_was_cmp(&self) -> bool {
        matches!(
            self.last,
            Some((
                _,
                InsnKind::Cmpw
                    | InsnKind::Cmplw
                    | InsnKind::Cmpwi
                    | InsnKind::Cmplwi
                    | InsnKind::Cmpwbc
                    | InsnKind::Cmplwbc
                    | InsnKind::Cmpwibc,
            ))
        )
    }

    /// Return the comparison operand expressions for the current instruction.
    ///
    /// - reg-reg forms  (`cmpw`, `cmplw`, `cmpwbc`, `cmplwbc`): [Reg(RA), Reg(RB)]
    /// - reg-imm forms  (`cmpwi`): [Reg(RA), I16(SI)]   (SI is sign-extended 16-bit)
    /// - reg-uimm forms (`cmplwi`): [Reg(RA), U16(UI)]  (UI is zero-extended 16-bit)
    /// - fused imm form (`cmpwibc`): [Reg(RA), U16(UIX)] (5-bit UIX, zero-extended)
    fn cmp(&self) -> Vec<CmpExpr> {
        let Some((insn, kind)) = self.last else { return vec![] };
        if !self.last_was_cmp() {
            return vec![];
        }

        let ra = format!("r{}", field_ra(insn));
        match kind {
            // reg-reg, signed compare
            InsnKind::Cmpw | InsnKind::Cmpwbc => {
                let rb = format!("r{}", field_rb(insn));
                vec![CmpExpr::Reg((ra, 4)), CmpExpr::Reg((rb, 4))]
            }
            // reg-reg, unsigned compare
            InsnKind::Cmplw | InsnKind::Cmplwbc => {
                let rb = format!("r{}", field_rb(insn));
                vec![CmpExpr::Reg((ra, 4)), CmpExpr::Reg((rb, 4))]
            }
            // reg-simm16, signed compare
            InsnKind::Cmpwi => {
                vec![CmpExpr::Reg((ra, 4)), CmpExpr::I16(field_si(insn) as i16)]
            }
            // reg-uimm16, unsigned compare
            InsnKind::Cmplwi => {
                vec![CmpExpr::Reg((ra, 4)), CmpExpr::U16(field_ui(insn) as u16)]
            }
            // cmpwibc: UIX is a 5-bit unsigned immediate (bits 16-20, i.e. RB field)
            InsnKind::Cmpwibc => {
                let uix = field_rb(insn) as u16; // 5-bit zero-extended
                vec![CmpExpr::Reg((ra, 4)), CmpExpr::U16(uix)]
            }
            _ => vec![],
        }
    }

    /// Return the comparison type(s) for the current instruction.
    ///
    /// PPE42 stand-alone compares (`cmpw`, `cmplw`, `cmpwi`, `cmplwi`) set all
    /// three CR[CR0] bits (LT, GT, EQ) so the fuzzer should consider all of
    /// them; we report the most general set here.
    ///
    /// For the fused compare-branch forms we have more information: the BIX
    /// field (bits 9-10 IBM numbering, i.e. standard bits 22-21 of the word)
    /// tells us which CR bit the branch tests:
    ///   BIX 0 → LT branch → Lesser
    ///   BIX 1 → GT branch → Greater
    ///   BIX 2 → EQ branch → Equal
    ///   BIX 3 → EQ (with register writeback) → Equal
    fn cmp_type(&self) -> Vec<CmpType> {
        let Some((insn, kind)) = self.last else { return vec![] };
        match kind {
            // Stand-alone compares: report all three comparison relations
            InsnKind::Cmpw | InsnKind::Cmplw | InsnKind::Cmpwi | InsnKind::Cmplwi => {
                vec![CmpType::Lesser, CmpType::Greater, CmpType::Equal]
            }
            // Fused compare-branch: derive type from BIX field.
            // BIX = IBM bits 9-10 = standard bits 22-21 → (insn >> 21) & 0x3
            InsnKind::Cmpwbc | InsnKind::Cmplwbc | InsnKind::Cmpwibc => {
                let bix = (insn >> 21) & 0x3;
                match bix {
                    0 => vec![CmpType::Lesser],
                    1 => vec![CmpType::Greater],
                    2 | 3 => vec![CmpType::Equal],
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }
}
