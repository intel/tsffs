use crate::{
    attr_object_or_nil, get_interface, AccessType, AttrValue, ConfObject, Interface, PhysicalBlock,
};
use anyhow::{bail, ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    cached_instruction_handle_t, cpu_bytes_t, cpu_cached_instruction_interface_t,
    cpu_instruction_query_interface_t, cpu_instrumentation_subscribe_interface_t,
    cycle_interface_t, instruction_handle_t, int_register_interface_t,
    processor_info_v2_interface_t,
};
use std::{ffi::c_void, ptr::null_mut, slice::from_raw_parts};

pub type InstructionHandle = instruction_handle_t;
pub type CachedInstructionHandle = cached_instruction_handle_t;
pub type CpuInstrumentationSubscribeInterface = cpu_instrumentation_subscribe_interface_t;
pub type CpuInstructionQueryInterface = cpu_instruction_query_interface_t;
pub type CpuCachedInstructionInterface = cpu_cached_instruction_interface_t;
pub type ProcessorInfoV2Interface = processor_info_v2_interface_t;
pub type IntRegisterInterface = int_register_interface_t;
pub type CycleInterface = cycle_interface_t;
pub type CpuBytes = cpu_bytes_t;

pub struct CpuInstrumentationSubscribe {
    iface: *mut CpuInstrumentationSubscribeInterface,
}

impl CpuInstrumentationSubscribe {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<CpuInstrumentationSubscribeInterface>(
            cpu,
            Interface::CpuInstrumentationSubscribe,
        )?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuInstrumentationSubscribe.try_as_slice()?)
            )
        } else {
            Ok(Self { iface })
        }
    }

    /// Run a callback to be run before every instruction is executed
    pub fn register_instruction_before_cb<D>(
        &self,
        cpu: *mut ConfObject,
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
        let user_data = match user_data {
            Some(data) => data.into(),
            None => null_mut(),
        };

        if let Some(register) = unsafe { *self.iface }.register_instruction_before_cb {
            unsafe { register(cpu.into(), null_mut(), Some(cb), user_data) };
            Ok(())
        } else {
            bail!("Unable to register callback, no register function");
        }
    }

    /// Run a callback to be run when an instruction is cached. This means for example the
    /// instructions in a loop will be called-back on *once*, not for every execution of the loop
    pub fn register_cached_instruction_cb<D>(
        &self,
        cpu: *mut ConfObject,
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
        let user_data = match user_data {
            Some(data) => data.into(),
            None => null_mut(),
        };

        if let Some(register) = unsafe { *self.iface }.register_cached_instruction_cb {
            unsafe { register(cpu.into(), null_mut(), Some(cb), user_data) };
            Ok(())
        } else {
            bail!("Unable to register callback, no register function");
        }
    }
}

pub struct CpuInstructionQuery {
    iface: *mut CpuInstructionQueryInterface,
}

impl CpuInstructionQuery {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface =
            get_interface::<CpuInstructionQueryInterface>(cpu, Interface::CpuInstructionQuery)?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuInstructionQuery.try_as_slice()?)
            )
        } else {
            Ok(Self { iface })
        }
    }

    /// Get the bytes of an instruction given an instruction handle retrieved during an
    /// instruction callback
    pub fn get_instruction_bytes(
        &mut self,
        cpu: *mut ConfObject,
        instruction_query: *mut InstructionHandle,
    ) -> Result<&[u8]> {
        let bytes = match unsafe { *self.iface }.get_instruction_bytes {
            Some(get_instruction_bytes) => unsafe {
                get_instruction_bytes(cpu.into(), instruction_query.into())
            },
            _ => bail!("No function get_instruction_bytes in interface"),
        };

        Ok(unsafe { from_raw_parts(bytes.data, bytes.size) })
    }
}

pub struct CpuCachedInstruction {
    _iface: *mut CpuCachedInstructionInterface,
}

impl CpuCachedInstruction {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface =
            get_interface::<CpuCachedInstructionInterface>(cpu, Interface::CpuCachedInstruction)?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuCachedInstruction.try_as_slice()?)
            )
        } else {
            Ok(Self { _iface: iface })
        }
    }
}

pub struct ProcessorInfoV2 {
    iface: *mut ProcessorInfoV2Interface,
}

impl ProcessorInfoV2 {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<ProcessorInfoV2Interface>(cpu, Interface::ProcessorInfoV2)?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::ProcessorInfoV2.try_as_slice()?)
            )
        } else {
            Ok(Self { iface })
        }
    }

    /// Get the program counter of a CPU
    pub fn get_program_counter(&self, cpu: *mut ConfObject) -> Result<u64> {
        if let Some(get_program_counter) = unsafe { *self.iface }.get_program_counter {
            Ok(unsafe { get_program_counter(cpu.into()) })
        } else {
            bail!("No function get_program_counter in interface");
        }
    }

    /// Translate a logical address to a physical address
    pub fn logical_to_physical(
        &self,
        cpu: *mut ConfObject,
        logical_address: u64,
    ) -> Result<PhysicalBlock> {
        if let Some(logical_to_physical) = unsafe { *self.iface }.logical_to_physical {
            let addr = unsafe {
                logical_to_physical(cpu.into(), logical_address, AccessType::X86Vanilla as u32)
            };
            ensure!(addr.valid != 0, "Physical address is invalid");
            Ok(addr)
        } else {
            bail!("No function logical_to_physical in interface");
        }
    }

    /// Get the physical memory object associated with a CPU
    pub fn get_physical_memory(&self, cpu: *mut ConfObject) -> Result<*mut ConfObject> {
        if let Some(get_physical_memory) = unsafe { *self.iface }.get_physical_memory {
            Ok(unsafe { get_physical_memory(cpu.into()) })
        } else {
            bail!("No function get_physical_memory in interface");
        }
    }
}

pub struct IntRegister {
    iface: *mut IntRegisterInterface,
}

impl IntRegister {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<IntRegisterInterface>(cpu, Interface::IntRegister)?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::IntRegister.try_as_slice()?)
            )
        } else {
            Ok(Self { iface })
        }
    }

    /// Get the number for a register name
    pub fn get_number<S>(&self, cpu: *mut ConfObject, register: S) -> Result<i32>
    where
        S: AsRef<str>,
    {
        if let Some(get_number) = unsafe { *self.iface }.get_number {
            Ok(unsafe { get_number(cpu.into(), raw_cstr(register.as_ref())?) })
        } else {
            bail!("No function get_number in interface");
        }
    }

    /// Read a register
    pub fn read(&self, cpu: *mut ConfObject, register_number: i32) -> Result<u64> {
        if let Some(read) = unsafe { *self.iface }.read {
            Ok(unsafe { read(cpu.into(), register_number) })
        } else {
            bail!("No function read in interface");
        }
    }

    pub fn write(
        &self,
        cpu: *mut ConfObject,
        register_number: i32,
        register_value: u64,
    ) -> Result<()> {
        if let Some(write) = unsafe { *self.iface }.write {
            unsafe { write(cpu.into(), register_number, register_value) };
            Ok(())
        } else {
            bail!("No function writein interface");
        }
    }
}

pub struct Cycle {
    _iface: *mut CycleInterface,
}

impl Cycle {
    pub fn try_new(cpu: *mut AttrValue) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: *mut ConfObject = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<CycleInterface>(cpu, Interface::Cycle)?;

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::Cycle.try_as_slice()?)
            )
        } else {
            Ok(Self { _iface: iface })
        }
    }
}
