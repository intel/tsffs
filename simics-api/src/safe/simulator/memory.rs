use simics_api_sys::{SIM_read_byte, SIM_write_byte};

use crate::ConfObject;

/// Write a byte to a physical address
pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
    unsafe { SIM_write_byte(physical_memory.into(), physical_addr, byte) };
}

/// Read a byte from a physical address
pub fn read_byte(physical_memory: *mut ConfObject, physical_addr: u64) -> u8 {
    unsafe { SIM_read_byte(physical_memory.into(), physical_addr) }
}
