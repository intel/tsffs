// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use simics_api_sys::{physical_address_t, physical_block_t, x86_access_type};

pub type PhysicalBlock = physical_block_t;
pub type PhysicalAddress = physical_address_t;

pub struct X86AccessType(x86_access_type);
