use crate::nonnull;
use anyhow::{anyhow, bail, Result};
use confuse_simics_api::{
    conf_object_t, cpu_bytes_t, cpu_cached_instruction_interface_t,
    cpu_instruction_query_interface_t, cpu_instrumentation_subscribe_interface_t,
    exception_interface_t, instruction_handle_t, int_register_interface_t,
    processor_info_v2_interface_t,
};
use log::error;
use std::slice::from_raw_parts;
use yaxpeax_x86::amd64::{InstDecoder, Instruction, Opcode};

/// Check if an instruction is a control flow instruction
fn instr_is_control_flow(insn: Instruction) -> bool {
    matches!(
        insn.opcode(),
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
    )
}

/// Check if an instruction is a call instruction
fn instr_is_call(insn: Instruction) -> bool {
    matches!(insn.opcode(), Opcode::CALL | Opcode::CALLF)
}

/// Check if an instruction is a ret instruction
fn instr_is_ret(insn: Instruction) -> bool {
    matches!(insn.opcode(), Opcode::RETF | Opcode::RETURN)
}

/// Check if an instruction is a cmp instruction
fn instr_is_cmp(insn: Instruction) -> bool {
    matches!(
        insn.opcode(),
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
    )
}

pub struct Cpu {
    cpu: *mut conf_object_t,
    _cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
    cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
    _cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
    processor_info_v2: *mut processor_info_v2_interface_t,
    int_register: *mut int_register_interface_t,
}

impl Cpu {
    pub fn try_new(
        cpu: *mut conf_object_t,
        // For information on these interfaces, see the "Model-to-simulator interfaces" part of the
        // documentation
        cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
        cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
        cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
        processor_info_v2: *mut processor_info_v2_interface_t,
        int_register: *mut int_register_interface_t,
    ) -> Result<Self> {
        Ok(Self {
            cpu: nonnull!(cpu)?,
            _cpu_instrumentation_subscribe: nonnull!(cpu_instrumentation_subscribe)?,
            cpu_instrumentation_query: nonnull!(cpu_instrumentation_query)?,
            _cpu_cached_instruction: nonnull!(cpu_cached_instruction)?,
            processor_info_v2: nonnull!(processor_info_v2)?,
            int_register: nonnull!(int_register)?,
        })
    }

    pub fn get_cpu(&self) -> *mut conf_object_t {
        self.cpu
    }

    pub fn get_int_register(&self) -> *mut int_register_interface_t {
        self.int_register
    }

    /// Called in cached instruction callback to check if the current instruction is a branch and
    /// return the pc at the instruction
    pub fn is_branch(
        &self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<Option<u64>> {
        let instruction_bytes: cpu_bytes_t = match unsafe { *self.cpu_instrumentation_query }
            .get_instruction_bytes
        {
            Some(get_instruction_bytes) => unsafe { get_instruction_bytes(cpu, instruction_query) },
            _ => bail!("No function get_instruction_bytes in interface"),
        };

        let instruction_bytes_data =
            unsafe { from_raw_parts(instruction_bytes.data, instruction_bytes.size) };

        let decoder = InstDecoder::default();

        if let Ok(insn) = decoder.decode_slice(instruction_bytes_data) {
            if instr_is_control_flow(insn) || instr_is_call(insn) || instr_is_ret(insn) {
                let pc = match unsafe { *self.processor_info_v2 }.get_program_counter {
                    Some(get_program_counter) => unsafe { get_program_counter(cpu) },
                    _ => bail!("No function get_program_counter in interface"),
                };
                Ok(Some(pc))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// checks if the current instruction is a compare and returns the set of constants it finds
    pub fn is_cmp(
        &self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<Option<(u64, u64)>> {
        let instruction_bytes: cpu_bytes_t = match unsafe { *self.cpu_instrumentation_query }
            .get_instruction_bytes
        {
            Some(get_instruction_bytes) => unsafe { get_instruction_bytes(cpu, instruction_query) },
            _ => bail!("No function get_instruction_bytes in interface"),
        };

        let instruction_bytes_data =
            unsafe { from_raw_parts(instruction_bytes.data, instruction_bytes.size) };

        let decoder = InstDecoder::default();

        if let Ok(insn) = decoder.decode_slice(instruction_bytes_data) {
            if instr_is_cmp(insn) {
                let pc = match unsafe { *self.processor_info_v2 }.get_program_counter {
                    Some(get_program_counter) => unsafe { get_program_counter(cpu) },
                    _ => bail!("No function get_program_counter in interface"),
                };
                // TODO: Log the actual cmp value(s)
                Ok(Some((pc, 0)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

unsafe impl Send for Cpu {}
unsafe impl Sync for Cpu {}
