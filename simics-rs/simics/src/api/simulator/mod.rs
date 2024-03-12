// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simulator control APIs

pub mod breakpoints;
pub mod callbacks;
pub mod configuration;
pub mod control;
pub mod debugger;
pub mod embed;
pub mod hap_consumer;
pub mod host_profiling;
pub mod memory;
pub mod modules;
pub mod paths;
pub mod processor;
pub mod python;
#[cfg(not(simics_deprecated_api_rev_exec))]
pub mod rev_exec;
pub mod script;
pub mod sim_caches;
pub mod sim_conf_object;
pub mod sim_get_class;
#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
pub mod snapshots;

pub use breakpoints::*;
pub use callbacks::*;
pub use configuration::*;
pub use control::*;
pub use debugger::*;
pub use embed::*;
pub use hap_consumer::*;
pub use host_profiling::*;
pub use memory::*;
pub use modules::*;
pub use paths::*;
pub use processor::*;
pub use python::*;
#[cfg(not(simics_deprecated_api_rev_exec))]
pub use rev_exec::*;
pub use script::*;
pub use sim_caches::*;
pub use sim_conf_object::*;
pub use sim_get_class::*;
#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
pub use snapshots::*;
