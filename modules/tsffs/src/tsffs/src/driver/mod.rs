// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(irrefutable_let_patterns)]

use anyhow::{anyhow, bail, Result};
use getters::Getters;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use simics::{
    api::{
        discard_future, get_attribute, get_object, object_is_processor, quit,
        restore_micro_checkpoint, restore_snapshot, save_micro_checkpoint, save_snapshot,
        set_log_level, AsConfObject, ConfObject, CoreMagicInstructionHap, GenericAddress, Hap,
        HapHandle, LogLevel, MicroCheckpointFlags,
    },
    info,
};
use simics_macro::{TryFromAttrValueTypeList, TryIntoAttrValueTypeDict};
use std::time::SystemTime;
use typed_builder::TypedBuilder;

use crate::{
    arch::{Architecture, ArchitectureOperations},
    fuzzer::FuzzerMessage,
    state::{Start, Stop, StopReason},
    Tsffs,
};
pub const SNAPSHOT_NAME: &str = "tsffs-origin-snapshot";

#[derive(TypedBuilder, Getters, Clone, Debug)]
pub struct StartBuffer {
    /// The physical address of the buffer
    pub physical_address: u64,
    /// Whether the address that translated to this physical address was virtual
    pub virt: bool,
}

#[derive(TypedBuilder, Getters, Clone, Debug)]
pub struct StartSize {
    #[builder(default, setter(strip_option))]
    /// The address of the magic start size value
    pub physical_address: Option<u64>,
    // NOTE: There is no need to save the size fo the size, it must be pointer-sized.
    /// The initial size of the magic start size
    pub initial_size: u64,
    /// Whether the address that translated to this physical address was virtual
    pub virt: bool,
}

#[derive(TypedBuilder, Default, Getters, Clone, Debug)]
#[getters(mutable)]
pub struct StartInformation {
    #[builder(default)]
    buffer: Option<StartBuffer>,
    #[builder(default)]
    size: Option<StartSize>,
}

impl Tsffs {
    // pub fn needs_magic_hap(&self) -> bool {
    //     *self.configuration().start_on_harness() || *self.configuration().stop_on_harness()
    // }
}

impl Tsffs {
    // pub fn write_testcase(&mut self, testcase: Vec<u8>) -> Result<()> {
    //     info!(
    //         self.as_conf_object_mut(),
    //         "Running with testcase {:?}", testcase
    //     );
    //     match (
    //         self.start_information().buffer().clone(),
    //         self.start_information().size().clone(),
    //     ) {
    //         (Some(b), Some(s)) => {
    //             self.start_core_architecture_mut()
    //                 .as_mut()
    //                 .ok_or_else(|| {
    //                     anyhow!("No magic core architecture set but magic instruction received")
    //                 })?
    //                 .write_start(&testcase, &b, &s)?;
    //         }
    //         _ => bail!("No buffer or no size set but magic instruction received"),
    //     }

    //     Ok(())
    // }
}

impl Tsffs {
    // pub fn save_initial_snapshot(&mut self) -> Result<()> {
    //     if *self.configuration().use_snapshots() && self.snapshot_name().is_none() {
    //         save_snapshot(SNAPSHOT_NAME)?;
    //         *self.snapshot_name_mut() = Some(SNAPSHOT_NAME.to_string());
    //     } else if !self.configuration().use_snapshots()
    //         && self.snapshot_name().is_none()
    //         && self.micro_checkpoint_index().is_none()
    //     {
    //         save_micro_checkpoint(
    //             SNAPSHOT_NAME,
    //             MicroCheckpointFlags::Sim_MC_ID_User | MicroCheckpointFlags::Sim_MC_Persistent,
    //         )?;

    //         *self.snapshot_name_mut() = Some(SNAPSHOT_NAME.to_string());

    //         *self.micro_checkpoint_index_mut() = Some(
    //             Helpers::get_micro_checkpoints()?
    //                 .iter()
    //                 .enumerate()
    //                 .find_map(|(i, c)| (c.name == SNAPSHOT_NAME).then_some(i as i32))
    //                 .ok_or_else(|| {
    //                     anyhow!("No micro checkpoint with just-registered name found")
    //                 })?,
    //         );
    //     }

    //     Ok(())
    // }

    // pub fn restore_initial_snapshot(&mut self) -> Result<()> {
    //     if *self.configuration().use_snapshots() {
    //         restore_snapshot(SNAPSHOT_NAME)?;
    //     } else {
    //         restore_micro_checkpoint(self.micro_checkpoint_index().ok_or_else(|| {
    //             anyhow!("Not using snapshots and no micro checkpoint index present")
    //         })?)?;
    //         discard_future()?;
    //     }

    //     Ok(())
    // }

    // pub fn have_initial_snapshot(&self) -> bool {
    //     (self.snapshot_name().is_some() && *self.configuration().use_snapshots())
    //         || (self.snapshot_name().is_some()
    //             && self.micro_checkpoint_index().is_some()
    //             && !self.configuration().use_snapshots())
    // }

    // pub fn increment_iterations_and_maybe_exit(&mut self) -> Result<()> {
    //     *self.iterations_mut() += 1;

    //     if self
    //         .configuration()
    //         .iterations()
    //         .is_some_and(|i| *self.iterations() >= i)
    //     {
    //         let duration = SystemTime::now().duration_since(*self.start_time())?;

    //         // Set the log level so this message always prints
    //         set_log_level(self.as_conf_object_mut(), LogLevel::Info)?;

    //         info!(
    //             self.as_conf_object(),
    //             "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
    //             self.iterations(),
    //             duration.as_secs_f32(),
    //             *self.iterations() as f32 / duration.as_secs_f32()
    //         );

    //         self.send_shutdown()?;

    //         quit(0)?;
    //     }

    //     Ok(())
    // }
}

/// Handlers for magic start and stop instructions, which automatically starts the fuzzing loop
/// when magic instructions compiled into the target software execute
impl Tsffs {
    // Called on magic start if the driver is configured to use the magic start harness
    // pub fn on_magic_start(&mut self, cpu: *mut ConfObject) -> Result<()> {
    //     if !self.have_initial_snapshot() {
    //         let mut arch = Architecture::new(cpu)?;
    //         let magic_start_buffer = arch.get_magic_start_buffer()?;
    //         let magic_start_size = arch.get_magic_start_size()?;

    //         info!(
    //             self.as_conf_object(),
    //             "Completed first magic start setup with architecture {arch:?}: {magic_start_buffer:?} {magic_start_size:?}"
    //         );

    //         *self.start_information_mut().buffer_mut() = Some(magic_start_buffer);
    //         *self.start_information_mut().size_mut() = Some(magic_start_size);

    //         *self.start_core_architecture_mut() = Some(arch);
    //         *self.start_time_mut() = SystemTime::now();

    //         // NOTE: We do *not* actually capture the snapshot here, because we may be in cell
    //         // context. Instead, after gathering information and setting up the buffer, we
    //         // trigger a simulation stop and capture a snapshot in the resulting callback.
    //     }

    //     self.stop_simulation(StopReason::MagicStart(
    //         Start::builder().processor(cpu).build(),
    //     ))?;

    //     Ok(())
    // }

    // /// Called on magic stop if the driver is configured to use the magic stop harness
    // ///
    // /// This method only performs actions that do not require global context. The
    // pub fn on_magic_stop(&mut self, _cpu: *mut ConfObject) -> Result<()> {
    //     self.stop_simulation(StopReason::MagicStop(Stop::default()))?;

    //     Ok(())
    // }
}

/// Handlers for the start and start_with_maximum_size interface methods, which start the fuzzing
/// loop without using a magic instruction compiled into the target
impl Tsffs {
    // on_start is only called a single time, to initialize the fuzzing loop
    // pub fn on_start(
    //     &mut self,
    //     cpu: *mut ConfObject,
    //     testcase_address: GenericAddress,
    //     size_address: GenericAddress,
    //     virt: bool,
    // ) -> Result<()> {
    //     if !self.have_initial_snapshot() {
    //         // NOTE: This is the first time start is being triggered. We need to go through
    //         // the whole buffer/size collection and snapshot process
    //         let mut arch = Architecture::new(cpu)?;
    //         *self.start_information_mut().buffer_mut() = Some(
    //             StartBuffer::builder()
    //                 .physical_address(testcase_address)
    //                 .virt(virt)
    //                 .build(),
    //         );
    //         *self.start_information_mut().size_mut() =
    //             Some(arch.get_start_size(size_address, virt)?);
    //         *self.start_core_architecture_mut() = Some(arch);
    //         *self.start_time_mut() = SystemTime::now();
    //     }

    //     self.stop_simulation(StopReason::Start(Start::builder().processor(cpu).build()))?;

    //     Ok(())
    // }

    // /// on_start_with_maximum_size is only called a single time, to initialize the fuzzing loop
    // pub fn on_start_with_maximum_size(
    //     &mut self,
    //     cpu: *mut ConfObject,
    //     testcase_address: GenericAddress,
    //     maximum_size: u32,
    //     virt: bool,
    // ) -> Result<()> {
    //     if !self.have_initial_snapshot() {
    //         // NOTE: This is the first time start is being triggered. We need to go through
    //         // the whole buffer/size collection and snapshot process
    //         let arch = Architecture::new(cpu)?;

    //         *self.start_information_mut().buffer_mut() = Some(
    //             StartBuffer::builder()
    //                 .physical_address(testcase_address)
    //                 .virt(virt)
    //                 .build(),
    //         );

    //         *self.start_information_mut().size_mut() = Some(
    //             StartSize::builder()
    //                 .initial_size(maximum_size as u64)
    //                 .virt(virt)
    //                 .build(),
    //         );

    //         *self.start_core_architecture_mut() = Some(arch);
    //         *self.start_time_mut() = SystemTime::now();
    //     }

    //     self.stop_simulation(StopReason::Start(Start::builder().processor(cpu).build()))?;

    //     Ok(())
    // }

    // pub fn on_stop(&mut self) -> Result<()> {
    //     self.stop_simulation(StopReason::Stop(Stop::default()))?;

    //     Ok(())
    // }
}

// impl<'a> Component for Driver<'a> {
//     /// Triggered when the simulation is stopped, with the reason it was stopped.
//     fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
//         match reason {
//             StopReason::MagicStart(_) | StopReason::Start(_) => {
//                 if !self.have_initial_snapshot() {
//                     self.save_initial_snapshot()?;
//
//                     // NOTE: Need to write the testcase on first run. For all other runs, the testcase
//                     // is written after stop->restore sequence
//                     if let FuzzerMessage::Testcase { testcase, cmplog } =
//                         self.parent_mut().fuzzer_mut().get_message()?
//                     {
//                         *self.parent_mut().tracer_mut().cmplog_enabled_mut() = cmplog;
//                         self.write_testcase(testcase)?;
//                     } else {
//                         bail!("Expected testcase");
//                     }
//                 }
//             }
//             StopReason::MagicStop(_) | StopReason::Stop(_) | StopReason::Solution(_) => {
//                 self.increment_iterations_and_maybe_exit()?;
//                 self.restore_initial_snapshot()?;
//
//                 info!(
//                     self.parent_mut().as_conf_object_mut(),
//                     "Iterations: {}",
//                     self.iterations()
//                 );
//
//                 if let FuzzerMessage::Testcase { testcase, cmplog } =
//                     self.parent_mut().fuzzer_mut().get_message()?
//                 {
//                     *self.parent_mut().tracer_mut().cmplog_enabled_mut() = cmplog;
//                     self.write_testcase(testcase)?;
//                 } else {
//                     bail!("Expected testcase");
//                 }
//             }
//         }
//         Ok(())
//     }
// }
