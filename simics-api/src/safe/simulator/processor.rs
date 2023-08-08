// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::ConfObject;
use simics_api_sys::{SIM_get_processor, SIM_get_processor_number};

/// Get the number of a particular processor
pub fn get_processor_number(cpu: *mut ConfObject) -> i32 {
    unsafe { SIM_get_processor_number(cpu as *const ConfObject) }
}

/// Get the processor from its number
pub fn get_processor(number: i32) -> *mut ConfObject {
    unsafe { SIM_get_processor(number) }
}
