// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Safe bindings to the SIMICS API
//!
//! In most cases, the SIMICS API is wrapped lightly to provide an experience familiar to SIMICS
//! model developers.

pub mod base;
pub mod interface;
pub mod internal;
pub mod logging;
pub mod processor;
pub mod simulator;
pub mod traits;
pub mod util;

pub use self::logging::*;

pub use base::*;
pub use interface::*;
pub use internal::*;
pub use processor::*;
pub use simulator::*;
pub use traits::*;
pub use util::*;

pub use simics_api_sys as sys;
