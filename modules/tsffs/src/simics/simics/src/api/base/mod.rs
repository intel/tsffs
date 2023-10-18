// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! The high level SIMICS API bindings. This module provides high-level bindings to almost all
//! SIMICS APIs, including attributes, objects, simulator APIs, interfaces, and HAPs.

pub mod attr_value;
pub mod conf_object;
pub mod event;
pub mod memory_transaction;
pub mod sim_exception;
pub mod sobject;
pub mod time;
pub mod version;

pub use attr_value::*;
pub use conf_object::*;
pub use event::*;
pub use memory_transaction::*;
pub use sim_exception::*;
pub use sobject::*;
pub use time::*;
pub use version::*;
