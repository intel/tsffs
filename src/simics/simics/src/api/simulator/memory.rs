// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use simics_api_sys::{SIM_read_byte, SIM_write_byte};

use crate::api::{clear_exception, get_pending_exception, last_error, ConfObject, SimException};

/// Write a byte to a physical address
pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) -> Result<()> {
    unsafe { SIM_write_byte(physical_memory.into(), physical_addr, byte) };

    match get_pending_exception()? {
        SimException::NoException => Ok(()),
        exception => {
            clear_exception()?;
            bail!(
                "Exception reading byte from {:#x}: {:?}({})",
                physical_addr,
                exception,
                last_error()
            );
        }
    }
}

/// Read a byte from a physical address
pub fn read_byte(physical_memory: *mut ConfObject, physical_addr: u64) -> Result<u8> {
    let byte = unsafe { SIM_read_byte(physical_memory.into(), physical_addr) };

    match get_pending_exception()? {
        SimException::NoException => Ok(byte),
        exception => {
            clear_exception()?;
            bail!(
                "Exception reading byte from {:#x}: {:?}({})",
                physical_addr,
                exception,
                last_error()
            );
        }
    }
}
