use anyhow::{anyhow, Error, Result};
use num_derive::FromPrimitive;
use simics_api_sys::{
    sim_exception_SimExc_AttrNotFound, sim_exception_SimExc_AttrNotReadable,
    sim_exception_SimExc_AttrNotWritable, sim_exception_SimExc_Attribute,
    sim_exception_SimExc_Break, sim_exception_SimExc_General, sim_exception_SimExc_IOError,
    sim_exception_SimExc_IllegalValue, sim_exception_SimExc_Index,
    sim_exception_SimExc_InquiryOutsideMemory, sim_exception_SimExc_InquiryUnhandled,
    sim_exception_SimExc_InterfaceNotFound, sim_exception_SimExc_License,
    sim_exception_SimExc_Lookup, sim_exception_SimExc_Memory, sim_exception_SimExc_No_Exception,
    sim_exception_SimExc_PythonTranslation, sim_exception_SimExc_Type,
    sim_exception_Sim_Exceptions, SIM_clear_exception, SIM_last_error,
};
use std::ffi::CStr;

#[derive(FromPrimitive)]
#[repr(u32)]
pub enum SimException {
    NoException = sim_exception_SimExc_No_Exception,
    General = sim_exception_SimExc_General,
    Lookup = sim_exception_SimExc_Lookup,
    Attribute = sim_exception_SimExc_Attribute,
    IOError = sim_exception_SimExc_IOError,
    Index = sim_exception_SimExc_Index,
    Memory = sim_exception_SimExc_Memory,
    Type = sim_exception_SimExc_Type,
    Break = sim_exception_SimExc_Break,
    PythonTranslation = sim_exception_SimExc_PythonTranslation,
    License = sim_exception_SimExc_License,
    IllegalValue = sim_exception_SimExc_IllegalValue,
    InquiryOutsideMemory = sim_exception_SimExc_InquiryOutsideMemory,
    InquiryUnhandled = sim_exception_SimExc_InquiryUnhandled,
    InterfaceNotFound = sim_exception_SimExc_InterfaceNotFound,
    AttrNotFound = sim_exception_SimExc_AttrNotFound,
    AttrNotReadable = sim_exception_SimExc_AttrNotReadable,
    AttrNotWritable = sim_exception_SimExc_AttrNotWritable,
    Exceptions = sim_exception_Sim_Exceptions,
}

impl TryFrom<u32> for SimException {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self> {
        num::FromPrimitive::from_u32(value)
            .ok_or_else(|| anyhow!("Could not convert {} to SimException", value))
    }
}

/// Get the last SIMICS error as a string
pub fn last_error() -> String {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}

pub fn clear_exception() -> Result<SimException> {
    unsafe { SIM_clear_exception() }.try_into()
}
