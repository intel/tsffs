// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! TFFS Module for SIMICS
//!
//! # Overview
//!
//! This crate provides a client and module loadable by SIMICS to enable fuzzing on the SIMICS
//! platform. The client is intended to be used by the `simics-fuzz` crate, but it can be used
//! manually to enable additional use cases.
//!
//! # Capabilities
//!
//! The Module can:
//!
//! - Trace branch hits during an execution of a target on an x86_64 processor. These branches
//!   are traced into shared memory in the format understood by the AFL family of tools.
//! - Catch exception/fault events registered in an initial configuration or dynamically using
//!   a SIMICS Python script
//! - Catch timeout events registered in an initial configuration or dynamically using a SIMICS
//!   Python script
//! - Manage the state of a target under test by taking and restoring a snapshot of its state for
//!   deterministic snapshot fuzzing

#![deny(clippy::all)]
// NOTE: We have to do this a lot, and it sucks to have all these functions be unsafe
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![deny(clippy::unwrap_used)]

use crate::{
    interface::TsffsInterfaceInternal,
    state::{Solution, SolutionKind},
};
use anyhow::{anyhow, Result};
use arch::{Architecture, ArchitectureOperations};
use configuration::Configuration;
use fuzzer::{ShutdownMessage, Testcase};
use getters2::Getters;
use libafl::prelude::ExitKind;
use libafl_bolts::{prelude::OwnedMutSlice, AsSlice};
use libafl_targets::AFLppCmpLogMap;
use serde::{Deserialize, Serialize};
#[cfg(simics_experimental_api_snapshots)]
use simics::api::{restore_snapshot, save_snapshot};
use simics::{
    api::{
        break_simulation, discard_future, free_attribute, get_class, get_interface,
        get_processor_number, object_clock, restore_micro_checkpoint, run_command,
        save_micro_checkpoint, AsConfObject, Class, ConfObject, CoreBreakpointMemopHap,
        CoreExceptionHap, CoreMagicInstructionHap, CoreSimulationStoppedHap,
        CpuInstrumentationSubscribeInterface, Event, EventClassFlag, HapHandle,
        MicroCheckpointFlags,
    },
    info, trace,
};
use simics_macro::{class, interface, AsConfObject};
use state::StopReason;
use std::{
    alloc::{alloc_zeroed, Layout},
    collections::HashMap,
    mem::size_of,
    ptr::null_mut,
    slice::from_raw_parts,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
    time::SystemTime,
};
use tracer::tsffs::{on_instruction_after, on_instruction_before};
use typed_builder::TypedBuilder;
use util::Utils;

pub mod arch;
pub mod configuration;
pub mod fuzzer;
pub mod haps;
pub mod init;
pub mod interface;
pub mod state;
pub mod tracer;
pub mod traits;
pub mod util;

/// The class name used for all operations interfacing with SIMICS

pub const CLASS_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Clone, Debug)]
pub struct StartBuffer {
    #[getters(deref)]
    /// The physical address of the buffer. Must be physical, if the input address was
    /// virtual, it should be pre-translated
    pub physical_address: u64,
    #[getters(deref)]
    /// Whether the address that translated to this physical address was virtual
    /// this should not be used or checked, it's simply informational
    pub virt: bool,
}

#[derive(TypedBuilder, Getters, Serialize, Deserialize, Clone, Debug)]
pub struct StartSize {
    #[builder(default, setter(into, strip_option))]
    /// The address of the magic start size value, and whether the address that translated
    /// to this physical address was virtual. The address must be physical.
    pub physical_address: Option<(u64, bool)>,
    #[builder(default, setter(into, strip_option))]
    // NOTE: There is no need to save the size fo the size, it must be pointer-sized.
    /// The initial size of the magic start size
    pub initial_size: Option<u64>,
}
impl Tsffs {
    pub const COVERAGE_MAP_SIZE: usize = 128 * 1024;
    pub const TIMEOUT_EVENT_NAME: &'static str = "detector_timeout_event";
    pub const SNAPSHOT_NAME: &'static str = "tsffs-origin-snapshot";
}

#[class(name = CLASS_NAME)]
#[derive(TypedBuilder, AsConfObject, Getters)]
#[getters(mutable)]
#[interface]
pub struct Tsffs {
    /// The pointer to this instance. This is a self pointer.
    instance: *mut ConfObject,
    #[builder(default)]
    #[getters(clone)]
    /// The configuration for the fuzzer
    configuration: Configuration,

    // Registered HAPs
    #[builder(default = {
        CoreSimulationStoppedHap::add_callback(
            // NOTE: Core_Simulation_Stopped is called with an object, exception and
            // error string, but the exception is always
            // SimException::SimExc_No_Exception and the error string is always
            // null_mut.
            move |_, _, _| {
                // On stops, call the module's stop callback method, which will in turn call the
                // stop callback methods on each of the module's components. The stop reason will
                // be retrieved from the module, if one is set. It is an error for the module to
                // stop itself without setting a reason
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_simulation_stopped()
                    .expect("Error calling simulation stopped callback");
            },
        )
        .expect("Failed to register core simulation stopped hap callback")
    })]
    /// Handle for the core simulation stopped hap
    stop_hap_handle: HapHandle,
    #[builder(default = {
        CoreBreakpointMemopHap::add_callback(
            move |trigger_obj, breakpoint_number, memop| {
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_breakpoint_memop(trigger_obj, breakpoint_number, memop)
                    .expect("Error calling breakpoint memop callback");
            }
        ).expect("Failed to register breakpoint memop callback")
    })]
    breakpoint_memop_hap_handle: HapHandle,
    #[builder(default = {
        CoreExceptionHap::add_callback(
            move |trigger_obj, exception_number| {
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_exception(trigger_obj, exception_number)
                    .expect("Error calling breakpoint memop callback");
            }
        ).expect("Failed to register breakpoint memop callback")
    })]
    exception_hap_handle: HapHandle,
    #[builder(default = {
        CoreMagicInstructionHap::add_callback(
            move |trigger_obj, magic_number| {
                let tsffs: &'static mut Tsffs = instance.into();

                tsffs
                    .on_magic_instruction(trigger_obj, magic_number)
                    .expect("Error calling magic instruction callback");
            },
        ).expect("Failed to register magic instruction callback")
    })]
    /// The handle for the registered magic HAP, used to
    /// listen for magic start and stop if `start_on_harness`
    /// or `stop_on_harness` are set.
    magic_hap_handle: HapHandle,

    // Fuzzer thread and channels
    #[builder(default)]
    fuzz_thread: Option<JoinHandle<Result<()>>>,
    #[builder(default)]
    fuzzer_tx: Option<Sender<ExitKind>>,
    #[builder(default)]
    fuzzer_rx: Option<Receiver<Testcase>>,
    #[builder(default)]
    fuzzer_shutdown: Option<Sender<ShutdownMessage>>,
    #[builder(default)]
    fuzzer_messages: Option<Receiver<String>>,

    // Fuzzer coverage maps
    #[builder(default = OwnedMutSlice::from(vec![0; Tsffs::COVERAGE_MAP_SIZE]))]
    /// Coverage map owned by the tracer
    coverage_map: OwnedMutSlice<'static, u8>,
    #[builder(default = unsafe {
        let layout = Layout::new::<AFLppCmpLogMap>();
        alloc_zeroed(layout) as *mut AFLppCmpLogMap
    })]
    #[getters(deref)]
    /// Comparison logging map owned by the tracer
    aflpp_cmp_map_ptr: *mut AFLppCmpLogMap,
    #[builder(default = unsafe { &mut *aflpp_cmp_map_ptr})]
    aflpp_cmp_map: &'static mut AFLppCmpLogMap,
    #[builder(default = 0)]
    #[getters(deref)]
    coverage_prev_loc: u64,

    // Registered events
    #[builder(default = Event::builder()
        .name(Tsffs::TIMEOUT_EVENT_NAME)
        .cls(get_class(CLASS_NAME).expect("Error getting class"))
        .flags(EventClassFlag::Sim_EC_No_Flags)
        .build()
    )]
    timeout_event: Event,

    // Micro checkpoint/snapshot management
    #[builder(default)]
    /// The name of the fuzz snapshot, if saved
    snapshot_name: Option<String>,
    #[builder(default)]
    #[getters(deref)]
    /// The index of the micro checkpoint saved for the fuzzer. Only present if not using
    /// snapshots.
    micro_checkpoint_index: Option<i32>,

    #[builder(default)]
    stop_reason: Option<StopReason>,
    #[builder(default)]
    /// The buffer and size information, if saved
    start_buffer: Option<StartBuffer>,
    #[builder(default)]
    start_size: Option<StartSize>,

    // Statistics
    #[builder(default = 0)]
    #[getters(deref)]
    /// The number of fuzzing iterations run. Incremented on stop
    iterations: usize,
    #[builder(default = SystemTime::now())]
    #[getters(deref)]
    /// The time the fuzzer was started at
    start_time: SystemTime,

    // State and settings
    #[builder(default = false)]
    #[getters(deref)]
    /// Whether cmplog is currently enabled
    coverage_enabled: bool,
    #[builder(default = false)]
    #[getters(deref)]
    /// Whether cmplog is currently enabled
    cmplog_enabled: bool,
    #[builder(default)]
    #[getters(deref)]
    /// The number of the processor which starts the fuzzing loop (via magic or manual methods)
    start_processor_number: Option<i32>,
    #[builder(default)]
    /// Tracked processors. This always includes the start processor, and may include
    /// additional processors that are manually added by the user
    processors: HashMap<i32, Architecture>,
    #[builder(default)]
    /// A testcase to use for repro
    repro_testcase: Option<Vec<u8>>,
    #[builder(default)]
    #[getters(deref)]
    repro_bookmark_set: bool,
    #[builder(default)]
    #[getters(deref)]
    stopped_for_repro: bool,
}

impl Class for Tsffs {
    fn init(instance: *mut ConfObject) -> simics::Result<*mut ConfObject> {
        let tsffs = Self::builder()
            .conf_object(unsafe { *instance })
            .instance(instance)
            .build();

        info!(instance, "Initialized instance");

        Ok(Tsffs::new(instance, tsffs))
    }
}

/// Implementations for controlling the simulation
impl Tsffs {
    pub fn stop_simulation(&mut self, reason: StopReason) -> Result<()> {
        let break_string = reason.to_string();
        *self.stop_reason_mut() = Some(reason);
        break_simulation(break_string)?;

        Ok(())
    }
}

/// Implementations for common functionality
impl Tsffs {
    pub fn add_processor(&mut self, cpu: *mut ConfObject, is_start: bool) -> Result<()> {
        let cpu_number = get_processor_number(cpu)?;

        if !self.processors_ref().contains_key(&cpu_number) {
            let architecture = if let Some(hint) = self
                .configuration_ref()
                .architecture_hints_ref()
                .get(&cpu_number)
            {
                hint.architecture(cpu)?
            } else {
                Architecture::new(cpu)?
            };
            self.processors_mut().insert(cpu_number, architecture);
            let mut cpu_interface: CpuInstrumentationSubscribeInterface = get_interface(cpu)?;
            cpu_interface.register_instruction_after_cb(
                null_mut(),
                Some(on_instruction_after),
                self as *mut Self as *mut _,
            )?;
            cpu_interface.register_instruction_before_cb(
                null_mut(),
                Some(on_instruction_before),
                self as *mut Self as *mut _,
            )?;
        }

        if is_start {
            *self.start_processor_number_mut() = Some(cpu_number);
        }

        Ok(())
    }

    pub fn start_processor(&mut self) -> Option<&mut Architecture> {
        self.start_processor_number_deref()
            .and_then(|n| self.processors_mut().get_mut(&n))
    }
}

impl Tsffs {
    pub fn save_initial_snapshot(&mut self) -> Result<()> {
        if self.configuration_ref().use_snapshots_deref() && self.snapshot_name_ref().is_none() {
            #[cfg(simics_experimental_api_snapshots)]
            {
                save_snapshot(Self::SNAPSHOT_NAME)?;
                *self.snapshot_name_mut() = Some(Self::SNAPSHOT_NAME.to_string());
            }
            #[cfg(not(simics_experimental_api_snapshots))]
            panic!("Snapshots cannot be used without SIMICS support from recent SIMICS versions.");
        } else if !self.configuration_ref().use_snapshots_deref()
            && self.snapshot_name_ref().is_none()
            && self.micro_checkpoint_index_ref().is_none()
        {
            save_micro_checkpoint(
                Self::SNAPSHOT_NAME,
                MicroCheckpointFlags::Sim_MC_ID_User | MicroCheckpointFlags::Sim_MC_Persistent,
            )?;

            *self.snapshot_name_mut() = Some(Self::SNAPSHOT_NAME.to_string());

            *self.micro_checkpoint_index_mut() = Some(
                Utils::get_micro_checkpoints()?
                    .iter()
                    .enumerate()
                    .find_map(|(i, c)| (c.name == Self::SNAPSHOT_NAME).then_some(i as i32))
                    .ok_or_else(|| {
                        anyhow!("No micro checkpoint with just-registered name found")
                    })?,
            );
        }

        Ok(())
    }

    pub fn restore_initial_snapshot(&mut self) -> Result<()> {
        if self.configuration_ref().use_snapshots_deref() {
            #[cfg(simics_experimental_api_snapshots)]
            restore_snapshot(Self::SNAPSHOT_NAME)?;
            #[cfg(not(simics_experimental_api_snapshots))]
            panic!("Snapshots cannot be used without SIMICS support from recent SIMICS versions.");
        } else {
            restore_micro_checkpoint(self.micro_checkpoint_index_ref().ok_or_else(|| {
                anyhow!("Not using snapshots and no micro checkpoint index present")
            })?)?;

            discard_future()?;
        }

        Ok(())
    }

    pub fn have_initial_snapshot(&self) -> bool {
        (self.snapshot_name_ref().is_some() && self.configuration_ref().use_snapshots_deref())
            || (self.snapshot_name_ref().is_some()
                && self.micro_checkpoint_index_ref().is_some()
                && !self.configuration_ref().use_snapshots_deref())
    }

    pub fn save_repro_bookmark_if_needed(&mut self) -> Result<()> {
        if self.repro_testcase_ref().is_some() && !self.repro_bookmark_set_deref() {
            free_attribute(run_command("set-bookmark start")?)?;
            *self.repro_bookmark_set_mut() = true;
        }

        Ok(())
    }
}

impl Tsffs {
    /// Get a testcase from the fuzzer and write it to memory along with, optionally, a size
    pub fn get_and_write_testcase(&mut self) -> Result<()> {
        let testcase = self.get_testcase()?;

        *self.cmplog_enabled_mut() = testcase.cmplog_deref();

        // TODO: Fix cloning - refcell?
        let start_buffer = self
            .start_buffer_ref()
            .as_ref()
            .ok_or_else(|| anyhow!("No start buffer"))?
            .clone();

        let start_size = self
            .start_size_ref()
            .as_ref()
            .ok_or_else(|| anyhow!("No start size"))?
            .clone();

        let start_processor = self
            .start_processor()
            .ok_or_else(|| anyhow!("No start processor"))?;

        start_processor.write_start(testcase.testcase_ref(), &start_buffer, &start_size)?;

        Ok(())
    }

    pub fn post_timeout_event(&mut self) -> Result<()> {
        let tsffs_ptr = self.as_conf_object_mut();
        let start_processor = self
            .start_processor()
            .ok_or_else(|| anyhow!("No start processor"))?;
        let start_processor_time = start_processor.cycle().get_time()?;
        let start_processor_cpu = start_processor.cpu();
        let start_processor_clock = object_clock(start_processor_cpu)?;
        let timeout_time = self.configuration_ref().timeout_deref() + start_processor_time;
        trace!(
            self.as_conf_object(),
            "Posting event on processor at time {} for {}s (time {})",
            start_processor_time,
            self.configuration_ref().timeout_deref(),
            timeout_time
        );
        self.timeout_event_ref().post_time(
            start_processor_cpu,
            start_processor_clock,
            self.configuration_ref().timeout_deref(),
            move |obj| {
                let tsffs: &'static mut Tsffs = tsffs_ptr.into();
                info!(tsffs.as_conf_object_mut(), "timeout({:#x})", obj as usize);
                tsffs
                    .stop_simulation(StopReason::Solution(
                        Solution::builder().kind(SolutionKind::Timeout).build(),
                    ))
                    .expect("Error calling timeout callback");
            },
        )?;

        Ok(())
    }

    pub fn cancel_timeout_event(&mut self) -> Result<()> {
        if let Some(start_processor) = self.start_processor() {
            let start_processor_time = start_processor.cycle().get_time()?;
            let start_processor_cpu = start_processor.cpu();
            let start_processor_clock = object_clock(start_processor_cpu)?;
            match self
                .timeout_event_ref()
                .find_next_time(start_processor_clock, start_processor_cpu)
            {
                Ok(next_time) => trace!(
                    self.as_conf_object(),
                    "Cancelling event with next time {} (current time {})",
                    next_time,
                    start_processor_time
                ),
                // NOTE: This is not an error, it almost always means we did not find a next
                // time, which always happens if the timeout goes off.
                Err(e) => trace!(
                    self.as_conf_object(),
                    "Not cancelling event with next time due to error: {e}"
                ),
            }
            self.timeout_event_ref()
                .cancel_time(start_processor_cpu, start_processor_clock)?;
        }
        Ok(())
    }

    pub fn coverage_hash(&self) -> u32 {
        crc32fast::hash(self.coverage_map_ref().as_slice())
    }

    pub fn cmplog_hash(&self) -> u32 {
        crc32fast::hash(unsafe {
            from_raw_parts(
                self.aflpp_cmp_map_ptr_deref() as *const u8,
                size_of::<AFLppCmpLogMap>(),
            )
        })
    }
}
