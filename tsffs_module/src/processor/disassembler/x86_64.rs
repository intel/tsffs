use anyhow::{bail, Result};
use yaxpeax_x86::amd64::{InstDecoder, Instruction, Opcode};

use crate::traits::TracerDisassembler;

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

impl TracerDisassembler for Disassembler {
    /// Check if an instruction is a control flow instruction
    fn last_was_control_flow(&self) -> Result<bool> {
        if let Some(last) = self.last {
            Ok(matches!(
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
            ))
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
            Ok(matches!(
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
            ))
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
}
