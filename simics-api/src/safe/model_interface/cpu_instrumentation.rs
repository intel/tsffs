use crate::{
    attr_object_or_nil, get_interface, AttrValue, ConfObject, Interface, OwnedMutAttrValuePtr,
    OwnedMutConfObjectPtr,
};
use anyhow::{bail, Result};
use simics_api_sys::{
    cpu_bytes_t, cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, cycle_interface_t, instruction_handle_t,
    int_register_interface_t, processor_info_v2_interface_t,
};
use std::{ffi::c_void, ptr::null_mut, slice::from_raw_parts};

pub type InstructionHandle = instruction_handle_t;
pub type CpuInstrumentationSubscribeInterface = cpu_instrumentation_subscribe_interface_t;
pub type CpuInstructionQueryInterface = cpu_instruction_query_interface_t;
pub type CpuCachedInstructionInterface = cpu_cached_instruction_interface_t;
pub type ProcessorInfoV2Interface = processor_info_v2_interface_t;
pub type IntRegisterInterface = int_register_interface_t;
pub type CycleInterface = cycle_interface_t;
pub type CpuBytes = cpu_bytes_t;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct OwnedMutInstructionHandlePtr {
    object: *mut InstructionHandle,
}

impl OwnedMutInstructionHandlePtr {
    pub fn new(object: *mut InstructionHandle) -> Self {
        Self { object }
    }

    pub fn as_const(&self) -> *const InstructionHandle {
        self.object as *const InstructionHandle
    }
}

impl From<*mut InstructionHandle> for OwnedMutInstructionHandlePtr {
    fn from(value: *mut InstructionHandle) -> Self {
        Self::new(value)
    }
}

impl From<OwnedMutInstructionHandlePtr> for *mut InstructionHandle {
    fn from(value: OwnedMutInstructionHandlePtr) -> Self {
        value.object
    }
}

impl From<&OwnedMutInstructionHandlePtr> for *mut InstructionHandle {
    fn from(value: &OwnedMutInstructionHandlePtr) -> Self {
        value.object
    }
}

pub struct CpuInstrumentationSubscribe {
    iface: *mut CpuInstrumentationSubscribeInterface,
}

impl CpuInstrumentationSubscribe {
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<CpuInstrumentationSubscribeInterface>(
            cpu,
            Interface::CpuInstrumentationSubscribe,
        );

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuInstrumentationSubscribe.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }

    pub fn register_instruction_before_cb(
        &self,
        cpu: OwnedMutConfObjectPtr,
        cb: unsafe extern "C" fn(
            *mut ConfObject,
            *mut ConfObject,
            *mut InstructionHandle,
            *mut c_void,
        ),
    ) -> Result<()> {
        if let Some(register) = unsafe { *self.iface }.register_instruction_before_cb {
            unsafe { register(cpu.into(), null_mut(), Some(cb), null_mut()) };
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
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface =
            get_interface::<CpuInstructionQueryInterface>(cpu, Interface::CpuInstructionQuery);

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuInstructionQuery.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }

    pub fn get_instruction_bytes(
        &mut self,
        cpu: OwnedMutConfObjectPtr,
        instruction_query: OwnedMutInstructionHandlePtr,
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
    iface: *mut CpuCachedInstructionInterface,
}

impl CpuCachedInstruction {
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface =
            get_interface::<CpuCachedInstructionInterface>(cpu, Interface::CpuCachedInstruction);

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::CpuCachedInstruction.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }
}

pub struct ProcessorInfoV2 {
    iface: *mut ProcessorInfoV2Interface,
}

impl ProcessorInfoV2 {
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<ProcessorInfoV2Interface>(cpu, Interface::ProcessorInfoV2);

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::ProcessorInfoV2.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }

    pub fn get_program_counter(&self, cpu: OwnedMutConfObjectPtr) -> Result<u64> {
        if let Some(get_program_counter) = unsafe { *self.iface }.get_program_counter {
            Ok(unsafe { get_program_counter(cpu.into()) })
        } else {
            bail!("No function get_program_counter in interface");
        }
    }
}

pub struct IntRegister {
    iface: *mut IntRegisterInterface,
}

impl IntRegister {
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<IntRegisterInterface>(cpu, Interface::IntRegister);

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::IntRegister.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }
}

pub struct Cycle {
    iface: *mut CycleInterface,
}

impl Cycle {
    pub fn try_new(cpu: OwnedMutAttrValuePtr) -> Result<Self> {
        let ptr: *mut AttrValue = cpu.into();

        let cpu: OwnedMutConfObjectPtr = attr_object_or_nil(unsafe { *ptr })?;

        let iface = get_interface::<CycleInterface>(cpu, Interface::Cycle);

        if iface.is_null() {
            bail!(
                "No interface {} found for cpu",
                String::from_utf8_lossy(Interface::Cycle.as_slice())
            )
        } else {
            Ok(Self { iface })
        }
    }
}
