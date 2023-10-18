// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::sys::{
    generic_address_t, logical_address_t, physical_address_t, physical_block_t, read_or_write_t,
    x86_access_type,
};

pub type PhysicalBlock = physical_block_t;
pub type PhysicalAddress = physical_address_t;
pub type X86AccessType = x86_access_type;
pub type LogicalAddress = logical_address_t;
pub type ReadOrWrite = read_or_write_t;
pub type GenericAddress = generic_address_t;
