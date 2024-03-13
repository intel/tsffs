// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Common processor types

use crate::sys::{
    generic_address_t, logical_address_t, physical_address_t, physical_block_t, read_or_write_t,
    x86_access_type,
};

/// Alias for `physical_block_t`
pub type PhysicalBlock = physical_block_t;
/// Alias for `physical_address_t`
pub type PhysicalAddress = physical_address_t;
/// Alias for `x86_access_type`
pub type X86AccessType = x86_access_type;
/// Alias for `logical_address_t`
pub type LogicalAddress = logical_address_t;
/// Alias for `read_or_write_t`
pub type ReadOrWrite = read_or_write_t;
/// Alias for `generic_address_t`
pub type GenericAddress = generic_address_t;
