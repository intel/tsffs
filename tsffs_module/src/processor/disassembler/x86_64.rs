// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Error, Result};
use tracing::trace;
use yaxpeax_x86::amd64::{InstDecoder, Instruction, Opcode, Operand};

use crate::traits::TracerDisassembler;

use super::CmpExpr;

pub struct Disassembler {
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

impl TryFrom<(&Operand, Option<u8>)> for CmpExpr {
    type Error = Error;

    fn try_from(value: (&Operand, Option<u8>)) -> Result<Self> {
        let width = value.1;
        let value = value.0;

        let expr = match value {
            Operand::ImmediateI8(i) => CmpExpr::I8(*i),
            Operand::ImmediateU8(u) => CmpExpr::U8(*u),
            Operand::ImmediateI16(i) => CmpExpr::I16(*i),
            Operand::ImmediateU16(u) => CmpExpr::U16(*u),
            Operand::ImmediateI32(i) => CmpExpr::I32(*i),
            Operand::ImmediateU32(u) => CmpExpr::U32(*u),
            Operand::ImmediateI64(i) => CmpExpr::I64(*i),
            Operand::ImmediateU64(u) => CmpExpr::U64(*u),
            Operand::Register(r) => CmpExpr::Reg((r.name().to_string(), r.width())),
            Operand::DisplacementU32(d) => CmpExpr::Addr(*d as u64),
            Operand::DisplacementU64(d) => CmpExpr::Addr(*d),
            Operand::RegDeref(r) => CmpExpr::Deref((
                Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                width,
            )),
            Operand::RegDisp(r, d) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                    Box::new(CmpExpr::I32(*d)),
                ))),
                width,
            )),
            Operand::RegScale(r, s) => CmpExpr::Deref((
                Box::new(CmpExpr::Mul((
                    Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                    Box::new(CmpExpr::U8(*s)),
                ))),
                width,
            )),
            Operand::RegIndexBase(r, i) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                    Box::new(CmpExpr::Reg((i.name().to_string(), i.width()))),
                ))),
                width,
            )),
            Operand::RegIndexBaseDisp(r, i, d) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Add((
                        Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                        Box::new(CmpExpr::Reg((i.name().to_string(), i.width()))),
                    ))),
                    Box::new(CmpExpr::I32(*d)),
                ))),
                width,
            )),
            Operand::RegScaleDisp(r, s, d) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Mul((
                        Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                        Box::new(CmpExpr::U8(*s)),
                    ))),
                    Box::new(CmpExpr::I32(*d)),
                ))),
                width,
            )),
            Operand::RegIndexBaseScale(r, i, s) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                    Box::new(CmpExpr::Add((
                        Box::new(CmpExpr::Reg((i.name().to_string(), i.width()))),
                        Box::new(CmpExpr::U8(*s)),
                    ))),
                ))),
                width,
            )),
            Operand::RegIndexBaseScaleDisp(r, i, s, d) => CmpExpr::Deref((
                Box::new(CmpExpr::Add((
                    Box::new(CmpExpr::Add((
                        Box::new(CmpExpr::Reg((r.name().to_string(), r.width()))),
                        Box::new(CmpExpr::Add((
                            Box::new(CmpExpr::Reg((i.name().to_string(), i.width()))),
                            Box::new(CmpExpr::U8(*s)),
                        ))),
                    ))),
                    Box::new(CmpExpr::I32(*d)),
                ))),
                width,
            )),
            _ => {
                bail!("Unsupported operand type for cmplog");
            }
        };
        trace!("Convert operand {:?} to cmpexpr {:?}", value, expr);
        Ok(expr)
    }
}

impl TracerDisassembler for Disassembler {
    /// Check if an instruction is a control flow instruction
    fn last_was_control_flow(&self) -> Result<bool> {
        if let Some(last) = self.last {
            if matches!(
                last.opcode(),
                Opcode::JA
                    | Opcode::JB
                    | Opcode::JRCXZ
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
            ) {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            bail!("No last instruction");
        }
    }

    /// Check if an instruction is a call instruction
    fn last_was_call(&self) -> Result<bool> {
        if let Some(last) = self.last {
            Ok(matches!(last.opcode(), Opcode::CALL | Opcode::CALLF))
        } else {
            bail!("No last instruction");
        }
    }

    /// Check if an instruction is a ret instruction
    fn last_was_ret(&self) -> Result<bool> {
        if let Some(last) = self.last {
            Ok(matches!(last.opcode(), Opcode::RETF | Opcode::RETURN))
        } else {
            bail!("No last instruction");
        }
    }

    /// Check if an instruction is a cmp instruction
    fn last_was_cmp(&self) -> Result<bool> {
        if let Some(last) = self.last {
            if matches!(
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
            ) {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            bail!("No last instruction");
        }
    }

    fn disassemble(&mut self, bytes: &[u8]) -> Result<()> {
        if let Ok(insn) = self.decoder.decode_slice(bytes) {
            self.last = Some(insn);
        } else {
            bail!("Could not disassemble {:?}", bytes);
        }

        Ok(())
    }

    fn cmp(&self) -> Result<Vec<CmpExpr>> {
        let mut cmp_exprs = Vec::new();
        if self.last_was_cmp()? {
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
        } else {
            bail!("Last was not a compare");
        }
        Ok(cmp_exprs)
    }
}
