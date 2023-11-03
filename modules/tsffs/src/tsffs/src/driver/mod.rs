// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(irrefutable_let_patterns)]

use anyhow::{anyhow, bail, Result};
use getters::Getters;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use simics::{
    api::{
        continue_simulation, discard_future, get_attribute, get_object, object_is_processor, quit,
        restore_micro_checkpoint, restore_snapshot, run_alone, save_micro_checkpoint,
        save_snapshot, set_log_level, AsConfObject, ConfObject, CoreMagicInstructionHap,
        GenericAddress, Hap, HapHandle, LogLevel, MicroCheckpointFlags,
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
    traits::Component,
    Tsffs,
};

/// The default start magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_START: i64 = 1;
/// The default stop magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_STOP: i64 = 2;
pub const SNAPSHOT_NAME: &str = "tsffs-origin-snapshot";

#[derive(TypedBuilder, Getters, Clone, Debug, TryIntoAttrValueTypeDict)]
#[getters(mutable)]
pub struct DriverConfiguration {
    #[builder(default = false)]
    start_on_harness: bool,
    #[builder(default = false)]
    stop_on_harness: bool,
    #[builder(default = false)]
    use_snapshots: bool,
    #[builder(default = DEFAULT_MAGIC_START)]
    magic_start: i64,
    #[builder(default = DEFAULT_MAGIC_STOP)]
    magic_stop: i64,
    #[builder(default, setter(strip_option))]
    iterations: Option<usize>,
}

impl Default for DriverConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

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
#[derive(TypedBuilder, Getters)]
#[getters(mutable)]
pub struct Driver<'a>
where
    'a: 'static,
{
    parent: &'a mut Tsffs,
    #[builder(default)]
    /// The driver configuration settings
    configuration: DriverConfiguration,
    #[builder(default)]
    start_core_architecture: Option<Architecture>,
    #[builder(default)]
    /// The name of the fuzz snapshot, if saved
    snapshot_name: Option<String>,
    #[builder(default)]
    /// The index of the micro checkpoint saved for the fuzzer. Only present if not using
    /// snapshots.
    micro_checkpoint_index: Option<i32>,
    #[builder(default)]
    /// The buffer and size information, if saved
    start_information: StartInformation,
    #[builder(default)]
    /// The handle for the registered magic HAP, used to
    /// listen for magic start and stop if `start_on_harness`
    /// or `stop_on_harness` are set.
    magic_hap_handle: Option<HapHandle>,
    #[builder(default = 0)]
    /// The number of fuzzing iterations run
    iterations: usize,
    #[builder(default = SystemTime::now())]
    /// The time the fuzzer was started at
    start_time: SystemTime,
}

impl<'a> Driver<'a>
where
    'a: 'static,
{
    fn needs_magic_hap(&self) -> bool {
        *self.configuration().start_on_harness() || *self.configuration().stop_on_harness()
    }

    /// Adds the magic hap callback if it is needed and not installed, or remove it if it is
    /// installed and not needed
    fn add_or_remove_magic_hap_if_needed(&mut self) -> Result<()> {
        let parent = self.parent_mut();
        let parent_conf_object = parent.as_conf_object_mut();

        if self.needs_magic_hap() && self.magic_hap_handle().is_none() {
            info!("Parent pointer {:#x}", parent_conf_object as usize);

            *self.magic_hap_handle_mut() = Some(CoreMagicInstructionHap::add_callback(
                move |trigger_obj, magic_number| {
                    let tsffs: &'static mut Tsffs = parent_conf_object.into();

                    tsffs
                        .driver_mut()
                        .on_magic_instruction(trigger_obj, magic_number)
                        .expect("Error calling magic instruction callback");
                },
            )?);
            info!(self.parent().as_conf_object(), "Adding magic HAP");
        } else if !self.needs_magic_hap() {
            if let Some(handle) = self.magic_hap_handle_mut().take() {
                info!(
                    self.parent().as_conf_object(),
                    "Removing magic HAP with ID {handle}"
                );
                CoreMagicInstructionHap::delete_callback_id(handle)?;
            }
        }

        Ok(())
    }

    /// Interface method, called when configuring the state of whether the driver should start
    /// the fuzzing loop on encountering a harness.
    pub fn set_start_on_harness(&mut self, start_on_harness: bool) -> Result<()> {
        *self.configuration_mut().start_on_harness_mut() = start_on_harness;
        self.add_or_remove_magic_hap_if_needed()
    }

    /// Interface method, called when configuring the state of whether the driver should stop
    /// the fuzzing loop on encountering a harness.
    pub fn set_stop_on_harness(&mut self, stop_on_harness: bool) -> Result<()> {
        *self.configuration_mut().stop_on_harness_mut() = stop_on_harness;
        self.add_or_remove_magic_hap_if_needed()
    }
}

impl<'a> Driver<'a> {
    pub fn write_testcase(&mut self, testcase: Vec<u8>) -> Result<()> {
        info!(
            self.parent_mut().as_conf_object_mut(),
            "Running with testcase {:?}", testcase
        );
        match (
            self.start_information().buffer().clone(),
            self.start_information().size().clone(),
        ) {
            (Some(b), Some(s)) => {
                self.start_core_architecture_mut()
                    .as_mut()
                    .ok_or_else(|| {
                        anyhow!("No magic core architecture set but magic instruction received")
                    })?
                    .write_start(&testcase, &b, &s)?;
            }
            _ => bail!("No buffer or no size set but magic instruction received"),
        }

        Ok(())
    }
}

impl<'a> Driver<'a> {
    pub fn save_initial_snapshot(&mut self) -> Result<()> {
        if *self.configuration().use_snapshots() {
            save_snapshot(SNAPSHOT_NAME)?;
            *self.snapshot_name_mut() = Some(SNAPSHOT_NAME.to_string());
        } else {
            save_micro_checkpoint(
                SNAPSHOT_NAME,
                MicroCheckpointFlags::Sim_MC_ID_User | MicroCheckpointFlags::Sim_MC_Persistent,
            )?;

            *self.snapshot_name_mut() = Some(SNAPSHOT_NAME.to_string());

            *self.micro_checkpoint_index_mut() = Some(
                Helpers::get_micro_checkpoints()?
                    .iter()
                    .enumerate()
                    .find_map(|(i, c)| (c.name == SNAPSHOT_NAME).then_some(i as i32))
                    .ok_or_else(|| {
                        anyhow!("No micro checkpoint with just-registered name found")
                    })?,
            );
        }

        Ok(())
    }

    pub fn restore_initial_snapshot(&mut self) -> Result<()> {
        if *self.configuration().use_snapshots() {
            restore_snapshot(SNAPSHOT_NAME)?;
        } else {
            restore_micro_checkpoint(self.micro_checkpoint_index().ok_or_else(|| {
                anyhow!("Not using snapshots and no micro checkpoint index present")
            })?)?;
            discard_future()?;
        }

        Ok(())
    }

    pub fn have_initial_snapshot(&self) -> bool {
        (self.snapshot_name().is_some() && *self.configuration().use_snapshots())
            || (self.snapshot_name().is_some()
                && self.micro_checkpoint_index().is_some()
                && !self.configuration().use_snapshots())
    }

    pub fn increment_iterations_and_maybe_exit(&mut self) -> Result<()> {
        *self.iterations_mut() += 1;

        if self
            .configuration()
            .iterations()
            .is_some_and(|i| *self.iterations() >= i)
        {
            let duration = SystemTime::now().duration_since(*self.start_time())?;

            // Set the log level so this message always prints
            set_log_level(self.parent_mut().as_conf_object_mut(), LogLevel::Info)?;

            info!(
                self.parent_mut().as_conf_object_mut(),
                "Configured iteration count {} reached. Stopping after {} seconds ({} exec/s).",
                self.iterations(),
                duration.as_secs_f32(),
                *self.iterations() as f32 / duration.as_secs_f32()
            );
            quit(0)?;
        }

        Ok(())
    }
}

impl<'a> Driver<'a> {
    /// Called on magic start if the driver is configured to use the magic start harness
    pub fn on_magic_start(&mut self, cpu: *mut ConfObject) -> Result<()> {
        if !self.have_initial_snapshot() {
            let mut arch = Architecture::new(cpu)?;
            let magic_start_buffer = arch.get_magic_start_buffer()?;
            let magic_start_size = arch.get_magic_start_size()?;

            info!(
                self.parent_mut().as_conf_object_mut(),
                "Completed first magic start setup with architecture {arch:?}: {magic_start_buffer:?} {magic_start_size:?}"
            );

            *self.start_information_mut().buffer_mut() = Some(magic_start_buffer);
            *self.start_information_mut().size_mut() = Some(magic_start_size);

            *self.start_core_architecture_mut() = Some(arch);
            *self.start_time_mut() = SystemTime::now();

            // NOTE: We do *not* actually capture the snapshot here, because we may be in cell
            // context. Instead, after gathering information and setting up the buffer, we
            // trigger a simulation stop and capture a snapshot in the resulting callback.
        }

        self.parent_mut().stop_simulation(StopReason::MagicStart(
            Start::builder().processor(cpu).build(),
        ))?;

        Ok(())
    }

    /// Called on magic stop if the driver is configured to use the magic stop harness
    ///
    /// This method only performs actions that do not require global context. The
    pub fn on_magic_stop(&mut self, _cpu: *mut ConfObject) -> Result<()> {
        self.parent_mut()
            .stop_simulation(StopReason::MagicStop(Stop::default()))?;

        Ok(())
    }

    /// Callback on magic instruction HAP. Checks whether the received magic number is registered
    /// as the start or stop number and acts accordingly by
    pub fn on_magic_instruction(
        &mut self,
        trigger_obj: *mut ConfObject,
        magic_number: i64,
    ) -> Result<()> {
        info!(
            self.parent().as_conf_object(),
            "on_magic_instruction({magic_number})"
        );

        if object_is_processor(trigger_obj)? {
            if *self.configuration().start_on_harness()
                && magic_number == *self.configuration().magic_start()
            {
                self.on_magic_start(trigger_obj)?;
            } else if *self.configuration().stop_on_harness()
                && magic_number == *self.configuration().magic_stop()
            {
                self.on_magic_stop(trigger_obj)?;
            }
        }

        Ok(())
    }
}

impl<'a> Driver<'a> {
    pub fn on_start(
        &mut self,
        cpu: *mut ConfObject,
        testcase_address: GenericAddress,
        size_address: GenericAddress,
        virt: bool,
    ) -> Result<()> {
        if !self.have_initial_snapshot() {
            // NOTE: This is the first time start is being triggered. We need to go through
            // the whole buffer/size collection and snapshot process
            let mut arch = Architecture::new(cpu)?;
            *self.start_information_mut().buffer_mut() = Some(
                StartBuffer::builder()
                    .physical_address(testcase_address)
                    .virt(virt)
                    .build(),
            );
            *self.start_information_mut().size_mut() =
                Some(arch.get_start_size(size_address, virt)?);
            *self.start_core_architecture_mut() = Some(arch);
            *self.start_time_mut() = SystemTime::now();
        }

        // TODO: get a new testcase from the fuzzer
        let testcase: Vec<u8> = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(thread_rng().gen_range(0..8))
            .map(u8::from)
            .collect();

        self.write_testcase(testcase)?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Completed start setup"
        );

        self.parent_mut()
            .stop_simulation(StopReason::Start(Start::builder().processor(cpu).build()))?;

        Ok(())
    }

    pub fn on_start_with_maximum_size(
        &mut self,
        cpu: *mut ConfObject,
        testcase_address: GenericAddress,
        maximum_size: u32,
        virt: bool,
    ) -> Result<()> {
        if self.snapshot_name().is_none() {
            // NOTE: This is the first time start is being triggered. We need to go through
            // the whole buffer/size collection and snapshot process
            let arch = Architecture::new(cpu)?;

            *self.start_information_mut().buffer_mut() = Some(
                StartBuffer::builder()
                    .physical_address(testcase_address)
                    .virt(virt)
                    .build(),
            );

            *self.start_information_mut().size_mut() = Some(
                StartSize::builder()
                    .initial_size(maximum_size as u64)
                    .virt(virt)
                    .build(),
            );

            *self.start_core_architecture_mut() = Some(arch);
            *self.start_time_mut() = SystemTime::now();
        }

        // TODO: get a new testcase from the fuzzer
        let testcase: Vec<u8> = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(thread_rng().gen_range(0..8))
            .map(u8::from)
            .collect();

        self.write_testcase(testcase)?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Completed start setup"
        );

        self.parent_mut()
            .stop_simulation(StopReason::Start(Start::builder().processor(cpu).build()))?;

        Ok(())
    }

    pub fn on_stop(&mut self) -> Result<()> {
        self.parent_mut()
            .stop_simulation(StopReason::Stop(Stop::default()))?;

        Ok(())
    }
}

impl<'a> Driver<'a> {
    /// Triggered when the simulation is stopped with a [`StopReason::MagicStart`] reason
    pub fn on_simulation_stopped_magic_start(&mut self) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.save_initial_snapshot()?;
        }

        if let FuzzerMessage::Testcase { testcase, cmplog } =
            self.parent_mut().fuzzer_mut().get_message()?
        {
            *self.parent_mut().tracer_mut().cmplog_enabled_mut() = cmplog;
            self.write_testcase(testcase)?;
        } else {
            bail!("Expected testcase");
        }

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Completed magic start setup"
        );

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Triggered when the simulation is stopped with a [`StopReason::Start`] reason
    pub fn on_simulation_stopped_start(&mut self) -> Result<()> {
        if !self.have_initial_snapshot() {
            self.save_initial_snapshot()?;
        }

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Triggered when the simulation is stopped with a [`StopReason::MagicStop`] reason
    /// The simulation has stopped, with the stop triggered by the driver catching a magic
    /// stop callback. This allows
    pub fn on_simulation_stopped_magic_stop(&mut self) -> Result<()> {
        self.increment_iterations_and_maybe_exit()?;
        // On a magic stop, we restore our initial snapshot and resume execution
        self.restore_initial_snapshot()?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Iterations: {}",
            self.iterations()
        );

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Triggered when the simulation is stopped with a [`StopReason::Stop`] reason
    pub fn on_simulation_stopped_stop(&mut self) -> Result<()> {
        self.increment_iterations_and_maybe_exit()?;
        self.restore_initial_snapshot()?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Iterations: {}",
            self.iterations()
        );

        if let FuzzerMessage::Testcase { testcase, cmplog } =
            self.parent_mut().fuzzer_mut().get_message()?
        {
            *self.parent_mut().tracer_mut().cmplog_enabled_mut() = cmplog;
            self.write_testcase(testcase)?;
        } else {
            bail!("Expected testcase");
        }

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Triggered when the simulation is stopped with a [`StopReason::Solution`] reason
    pub fn on_simulation_stopped_solution(&mut self) -> Result<()> {
        self.increment_iterations_and_maybe_exit()?;
        self.restore_initial_snapshot()?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Iterations: {}",
            self.iterations()
        );

        if let FuzzerMessage::Testcase { testcase, cmplog } =
            self.parent_mut().fuzzer_mut().get_message()?
        {
            *self.parent_mut().tracer_mut().cmplog_enabled_mut() = cmplog;
            self.write_testcase(testcase)?;
        } else {
            bail!("Expected testcase");
        }

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }
}

impl<'a> Component for Driver<'a> {
    /// Triggered when the simulation is stopped, with the reason it was stopped.
    fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
        match reason {
            StopReason::MagicStart(_) => self.on_simulation_stopped_magic_start()?,
            StopReason::MagicStop(_) => self.on_simulation_stopped_magic_stop()?,
            StopReason::Start(_) => self.on_simulation_stopped_start()?,
            StopReason::Stop(_) => self.on_simulation_stopped_stop()?,
            StopReason::Solution(_) => self.on_simulation_stopped_solution()?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, TryFromAttrValueTypeList)]
struct MicroCheckpointInfo {
    pub name: String,
    #[allow(unused)]
    pub pages: u64,
    #[allow(unused)]
    pub zero: u64,
}

struct Helpers {}

impl Helpers {
    fn get_micro_checkpoints() -> Result<Vec<MicroCheckpointInfo>> {
        Ok(get_attribute(get_object("sim.rexec")?, "state_info")?.try_into()?)
    }
}
