// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use anyhow::{anyhow, bail, Result};
use getters::Getters;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use simics::{
    api::{
        continue_simulation, discard_future, get_attribute, get_object, object_is_processor, quit,
        restore_micro_checkpoint, restore_snapshot, run_alone, save_micro_checkpoint,
        save_snapshot, set_log_level, AsConfObject, ConfObject, CoreMagicInstructionHap, Hap,
        HapHandle, LogLevel, MicroCheckpointFlags,
    },
    info,
};
use simics_macro::{TryFromAttrValueTypeList, TryIntoAttrValueTypeDict};
use typed_builder::TypedBuilder;

use crate::{
    arch::{Architecture, ArchitectureOperations},
    state::StopReason,
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
pub struct MagicStartBuffer {
    /// The physical address of the buffer
    pub physical_address: u64,
    /// Whether the address that translated to this physical address was virtual
    pub virt: bool,
}

#[derive(TypedBuilder, Getters, Clone, Debug)]
pub struct MagicStartSize {
    /// The address of the magic start size value
    pub physical_address: u64,
    // NOTE: There is no need to save the size fo the size, it must be pointer-sized.
    /// The initial size of the magic start size
    pub initial_size: u64,
    /// Whether the address that translated to this physical address was virtual
    pub virt: bool,
}

#[derive(TypedBuilder, Default, Getters, Clone, Debug)]
#[getters(mutable)]
pub struct MagicStartInformation {
    #[builder(default)]
    buffer: Option<MagicStartBuffer>,
    #[builder(default)]
    size: Option<MagicStartSize>,
}
#[derive(TypedBuilder, Getters, Debug)]
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
    magic_core_architecture: Option<Architecture>,
    #[builder(default)]
    /// The name of the fuzz snapshot, if saved
    snapshot_name: Option<String>,
    #[builder(default)]
    /// The index of the micro checkpoint saved for the fuzzer. Only present if not using
    /// snapshots.
    micro_checkpoint_index: Option<i32>,
    #[builder(default)]
    /// The buffer and size information, if saved
    magic_start_information: MagicStartInformation,
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

    /// Called on the first magic start if the driver is configured to use the magic start
    /// harness.
    ///
    /// We will:
    /// * Save the passed buffer information (architecture dependent)
    /// * Save a snapshot or micro checkpoint
    pub fn on_first_magic_start(&mut self, cpu: *mut ConfObject) -> Result<()> {
        // Collect the architecture for the triggering CPU. This is saved, as we presume
        // we will not have the cpu running this code change architectures from under us
        let mut arch = Architecture::get(cpu)?;
        let magic_start_buffer = arch.get_magic_start_buffer()?;
        let magic_start_size = arch.get_magic_start_size()?;

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Completed first magic start setup with architecture {arch:?}: {magic_start_buffer:?} {magic_start_size:?}"
        );

        *self.magic_start_information_mut().buffer_mut() = Some(magic_start_buffer);
        *self.magic_start_information_mut().size_mut() = Some(magic_start_size);

        *self.magic_core_architecture_mut() = Some(arch);
        *self.start_time_mut() = SystemTime::now();

        Ok(())
    }

    /// Called on magic start if the driver is configured to use the magic start harness
    ///
    pub fn on_magic_start(&mut self, cpu: *mut ConfObject) -> Result<()> {
        if self.snapshot_name().is_none() {
            self.on_first_magic_start(cpu)?;
        }

        // TODO: get a new testcase from the fuzzer
        let testcase: Vec<u8> = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(thread_rng().gen_range(0..8))
            .map(u8::from)
            .collect();

        match (
            self.magic_start_information().buffer().clone(),
            self.magic_start_information().size().clone(),
        ) {
            (Some(b), Some(s)) => {
                self.magic_core_architecture_mut()
                    .as_mut()
                    .ok_or_else(|| {
                        anyhow!("No magic core architecture set but magic instruction received")
                    })?
                    .write_magic_start(&testcase, &b, &s)?;
            }
            _ => bail!("No buffer or no size set but magic instruction received"),
        }

        info!(
            self.parent_mut().as_conf_object_mut(),
            "Completed magic start setup"
        );

        self.parent_mut().stop_simulation(StopReason::MagicStart)?;

        Ok(())
    }

    /// Called on magic stop if the driver is configured to use the magic stop harness
    ///
    /// This method only performs actions that do not require global context. The
    pub fn on_magic_stop(&mut self, _cpu: *mut ConfObject) -> Result<()> {
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

        self.parent_mut().stop_simulation(StopReason::MagicStop)?;

        Ok(())
    }

    pub fn on_simulation_stopped_magic_start(&mut self) -> Result<()> {
        if self.snapshot_name().is_none() {
            self.save_initial_snapshot()?;
        }

        run_alone(|| {
            continue_simulation(0)?;
            Ok(())
        })?;

        Ok(())
    }

    /// The simulation has stopped, with the stop triggered by the driver catching a magic
    /// stop callback. This allows
    pub fn on_simulation_stopped_magic_stop(&mut self) -> Result<()> {
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
                self.parent_mut().stop_simulation(StopReason::MagicStop)?;
            }
        }

        Ok(())
    }
}

impl<'a> Component for Driver<'a> {
    fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
        match reason {
            StopReason::MagicStart => {
                self.on_simulation_stopped_magic_start()?;
            }
            StopReason::MagicStop => {
                self.on_simulation_stopped_magic_stop()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, TryFromAttrValueTypeList)]
struct MicroCheckpointInfo {
    pub name: String,
    pub pages: u64,
    pub zero: u64,
}

struct Helpers {}

impl Helpers {
    fn get_micro_checkpoints() -> Result<Vec<MicroCheckpointInfo>> {
        Ok(get_attribute(get_object("sim.rexec")?, "state_info")?.try_into()?)
    }
}
