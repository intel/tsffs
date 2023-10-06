// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Definitions of various reasons the simulation can stop

use serde::{Deserialize, Serialize};

use super::{components::detector::fault::Fault, controller::magic::Magic};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Each time the simulation stops, a stop type must be used to determine whether the stop is
/// normal, a crash, or a timeout (timeouts cannot be monitored by the fuzzer because the
/// simulator does not run at wall clock speeds, they MUST be monitored by SIMICS). In all cases
/// a snapshot will be reverted to, but we need this information to inform the fuzzer objectives
pub enum StopReason {
    /// A magic instruction happened
    Magic(Magic),
    /// A (possibly) normal stop due to the simulation exiting
    SimulationExit,
    /// A crash occurred
    Crash(Fault),
    /// A timeout occurred
    TimeOut,
}
