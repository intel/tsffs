// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::api::sys::{SIM_read_byte, SIM_write_byte};
use crate::api::{clear_exception, get_pending_exception, last_error, ConfObject, SimException};
use anyhow::{bail, Result};

/// Write a byte to a physical address
pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) -> Result<()> {
    unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };

    match get_pending_exception() {
        SimException::SimExc_No_Exception => Ok(()),
        exception => {
            clear_exception();
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
    let byte = unsafe { SIM_read_byte(physical_memory, physical_addr) };

    match get_pending_exception() {
        SimException::SimExc_No_Exception => Ok(byte),
        exception => {
            clear_exception();
            bail!(
                "Exception reading byte from {:#x}: {:?}({})",
                physical_addr,
                exception,
                last_error()
            );
        }
    }
}
