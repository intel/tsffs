// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Traits to help implement common SIMICS structures to build SIMICS modules:
//!
//! - SIMICS classes
//! - HAPs
//! - Class and object interfaces

pub mod class;
pub mod hap;
pub mod interface;

pub use class::*;
pub use hap::*;
pub use interface::*;
