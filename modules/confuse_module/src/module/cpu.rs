use crate::nonnull;
use anyhow::{bail, ensure, Context, Result};
use confuse_simics_api::{
    attr_value_t, cached_instruction_handle_t, conf_object, conf_object_t, cpu_bytes_t,
    cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, cycle_interface_t, instruction_handle_t,
    int_register_interface_t, physical_block_t, processor_info_v2_interface_t,
    x86_access_type_X86_Vanilla, SIM_attr_object_or_nil, SIM_c_get_interface, SIM_read_byte,
    SIM_write_byte, CPU_CACHED_INSTRUCTION_INTERFACE, CPU_INSTRUCTION_QUERY_INTERFACE,
    CPU_INSTRUMENTATION_SUBSCRIBE_INTERFACE, CYCLE_INTERFACE, INT_REGISTER_INTERFACE,
    PROCESSOR_INFO_V2_INTERFACE,
};
use raw_cstr::raw_cstr;
use std::ffi::CString;
use std::{ffi::c_void, ptr::null_mut, slice::from_raw_parts};
use tracing::info;
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
    cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
    cpu_instruction_query: *mut cpu_instruction_query_interface_t,
    _cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
    processor_info_v2: *mut processor_info_v2_interface_t,
    int_register: *mut int_register_interface_t,
    _cycle: *mut cycle_interface_t,
}

impl Cpu {
    /// Initialize a CPU object
    ///
    /// # Safety
    ///
    /// This function dereferences `cpu` which must be a non-null pointer. The `cpu` parameter
    /// in the `add_processor` must actually be a CPU, otherwise this is unsafe and we have no
    /// way of knowing this is the case.
    pub unsafe fn try_new(cpu: *mut attr_value_t) -> Result<Self> {
        let cpu: *mut conf_object = unsafe { SIM_attr_object_or_nil(*cpu) }?;

        info!("Got CPU");

        let cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t = unsafe {
            SIM_c_get_interface(
                cpu,
                CPU_INSTRUMENTATION_SUBSCRIBE_INTERFACE.as_ptr() as *const i8,
            ) as *mut cpu_instrumentation_subscribe_interface_t
        };

        info!("Subscribed to CPU instrumentation");

        let cpu_instruction_query: *mut cpu_instruction_query_interface_t = unsafe {
            SIM_c_get_interface(cpu, CPU_INSTRUCTION_QUERY_INTERFACE.as_ptr() as *const i8)
                as *mut cpu_instruction_query_interface_t
        };

        info!("Got CPU query interface");

        let cpu_cached_instruction: *mut cpu_cached_instruction_interface_t = unsafe {
            SIM_c_get_interface(cpu, CPU_CACHED_INSTRUCTION_INTERFACE.as_ptr() as *const i8)
                as *mut cpu_cached_instruction_interface_t
        };

        info!("Subscribed to cached instructions");

        let processor_info_v2: *mut processor_info_v2_interface_t = unsafe {
            SIM_c_get_interface(cpu, PROCESSOR_INFO_V2_INTERFACE.as_ptr() as *const i8)
                as *mut processor_info_v2_interface_t
        };

        info!("Subscribed to processor info");

        let int_register: *mut int_register_interface_t = unsafe {
            SIM_c_get_interface(cpu, INT_REGISTER_INTERFACE.as_ptr() as *const i8)
                as *mut int_register_interface_t
        };

        let cycle: *mut cycle_interface_t = unsafe {
            SIM_c_get_interface(cpu, CYCLE_INTERFACE.as_ptr() as *const i8)
                as *mut cycle_interface_t
        };

        info!("Subscribed to internal register queries");
        Ok(Self {
            cpu: nonnull!(cpu),
            cpu_instrumentation_subscribe: nonnull!(cpu_instrumentation_subscribe),
            cpu_instruction_query: nonnull!(cpu_instruction_query),
            _cpu_cached_instruction: nonnull!(cpu_cached_instruction),
            processor_info_v2: nonnull!(processor_info_v2),
            int_register: nonnull!(int_register),
            _cycle: nonnull!(cycle),
        })
    }

    pub fn get_cpu(&self) -> *mut conf_object_t {
        self.cpu
    }

    pub fn get_reg_value<S: AsRef<str>>(&self, reg: S) -> Result<u64> {
        let reg_number = unsafe {
            (*self.int_register)
                .get_number
                .context("No function get_number")?(self.cpu, raw_cstr!(reg.as_ref()))
        };

        Ok(unsafe { (*self.int_register).read.context("No function read")?(self.cpu, reg_number) })
    }

    pub fn register_cached_instruction_cb(
        &self,
        cb: extern "C" fn(
            *mut conf_object_t,
            *mut conf_object_t,
            *mut cached_instruction_handle_t,
            *mut instruction_handle_t,
            *mut c_void,
        ),
    ) -> Result<()> {
        if let Some(register) =
            unsafe { *self.cpu_instrumentation_subscribe }.register_cached_instruction_cb
        {
            unsafe { register(self.cpu, null_mut(), Some(cb), null_mut()) };
        } else {
            bail!("No function register_cached_instruction_cb");
        }

        Ok(())
    }

    pub fn logical_to_physical(&self, logical_addr: u64) -> Result<physical_block_t> {
        let physical = match unsafe { *self.processor_info_v2 }.logical_to_physical {
            Some(logical_to_physical) => unsafe {
                // TODO: Is vanilla the right type?
                logical_to_physical(self.cpu, logical_addr, x86_access_type_X86_Vanilla)
            },
            _ => bail!("No function get_program_counter in interface"),
        };

        ensure!(physical.valid != 0, "Physical address is invalid");

        Ok(physical)
    }

    pub fn write_bytes(&self, logical_addr_start: &u64, bytes: &[u8]) -> Result<()> {
        // TODO: This whole function is super unoptimized, we should do 8-byte writes with SIM_write_phys_mem
        // and we should avoid querying l2p while we know we're in a contiguous block, etc.
        // but for now, it's ok :)
        let physical_memory = match unsafe { *self.processor_info_v2 }.get_physical_memory {
            Some(get_physical_memory) => unsafe { get_physical_memory(self.cpu) },
            _ => bail!("No function get_physical_memory in interface"),
        };

        for (i, byte) in bytes.iter().enumerate() {
            let logical_addr = logical_addr_start + i as u64;
            let physical_addr = self.logical_to_physical(logical_addr)?.address;
            unsafe { SIM_write_byte(physical_memory, physical_addr, *byte) };
            let written = unsafe { SIM_read_byte(physical_memory, physical_addr) };
            ensure!(written == *byte, "Did not read back same written byte");
        }

        Ok(())
    }

    /// Called in cached instruction callback to check if the current instruction is a branch and
    /// return the pc at the instruction
    ///
    /// # Safety
    ///
    /// This function is safe provided the `instruction_query` parameter comes from a SIMICS
    /// callback and the cpu object was initialized correctly
    pub unsafe fn is_branch(
        &self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<Option<u64>> {
        let instruction_bytes: cpu_bytes_t = match unsafe { *self.cpu_instruction_query }
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

    /// Checks if the current instruction is a compare and returns the set of constants it finds
    /// # Safety
    ///
    /// This function is safe provided the `instruction_query` parameter comes from a SIMICS
    /// callback and the cpu object was initialized correctly
    pub unsafe fn is_cmp(
        &self,
        cpu: *mut conf_object_t,
        instruction_query: *mut instruction_handle_t,
    ) -> Result<Option<(u64, u64)>> {
        let instruction_bytes: cpu_bytes_t = match unsafe { *self.cpu_instruction_query }
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
