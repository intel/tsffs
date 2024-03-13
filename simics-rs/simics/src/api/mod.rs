// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! The SIMICS API

pub mod base;
pub mod interface;
pub mod internal;
pub mod logging;
pub mod processor;
pub mod simulator;
pub mod traits;
pub mod util;

pub use base::*;
pub use interface::*;
pub use internal::*;
pub use logging::*;
pub use processor::*;
pub use simulator::*;
pub use traits::*;
pub use util::*;

pub use simics_api_sys as sys;
