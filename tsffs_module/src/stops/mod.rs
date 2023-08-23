// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Definitions of various reasons the simulation can stop

use serde::{Deserialize, Serialize};

use crate::{faults::Fault, magic::Magic};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StopError {
    UnknownFault(i64),
    NonErrorFault(Fault),
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Each time the simulation stops, a stop type must be used to determine whether the stop is
/// normal, a crash, or a timeout (timeouts cannot be monitored by the fuzzer because the
/// simulator does not run at wall clock speeds, they MUST be monitored by SIMICS). In all cases
/// a snapshot will be reverted to, but we need this information to inform the fuzzer objectives
pub enum StopReason {
    /// A magic instruction happened, save the magic type and the cpu number that hit the magic
    /// instruction. The second value is the processor number the instruction was raised on.
    Magic((Magic, i32)),
    /// A (possibly) normal stop due to the simulation exiting
    SimulationExit(i32),
    /// A crash occurred
    Crash((Fault, i32)),
    /// A timeout occurred
    TimeOut,
    /// An error occurred either during simulation or internally in the module
    Error((StopError, i32)),
    /// A breakpoint was encountered, report its number (this can be used to determine why the
    /// breakpoint is an error or some such thing)
    Breakpoint(i64),
}
