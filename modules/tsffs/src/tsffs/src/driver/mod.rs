// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use getters::Getters;
use simics::{
    api::{AsConfObject, ConfObject, CoreMagicInstructionHap, GenericAddress, Hap, HapHandle},
    info,
};
use simics_macro::TryIntoAttrValueType;
use typed_builder::TypedBuilder;

use crate::Tsffs;

/// The default start magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_START: i64 = 1;
/// The default stop magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_STOP: i64 = 2;

#[derive(TypedBuilder, Getters, Clone, Debug, TryIntoAttrValueType)]
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
}

impl Default for DriverConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Default, Getters, Clone, Debug, TryIntoAttrValueType)]
#[getters(mutable)]
pub struct BufferInformation {
    #[builder(default)]
    /// The address testcases should be written into.
    testcase_address: Option<GenericAddress>,
    #[builder(default)]
    size_address: Option<GenericAddress>,
    #[builder(default)]
    maximum_size: Option<u64>,
    #[builder(default)]
    virt: Option<bool>,
}

#[derive(TypedBuilder, Getters, Debug)]
#[getters(mutable)]
pub struct Driver<'a>
where
    'a: 'static,
{
    parent: &'a Tsffs,
    #[builder(default)]
    /// The driver configuration settings
    configuration: DriverConfiguration,
    #[builder(default)]
    /// The name of the fuzz snapshot, if saved
    snapshot_name: Option<String>,
    #[builder(default)]
    /// The buffer and size information, if saved
    buffer_information: BufferInformation,
    #[builder(default)]
    /// The handle for the registered magic HAP, used to
    /// listen for magic start and stop if `start_on_harness`
    /// or `stop_on_harness` are set.
    magic_hap_handle: Option<HapHandle>,
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
        if self.needs_magic_hap() && self.magic_hap_handle().is_none() {
            let parent = self.parent().as_conf_object();

            let callback = Box::new(move |trigger_obj, magic_number| {
                let tsffs: &'static mut Tsffs = (parent as *mut ConfObject).into();

                tsffs
                    .driver_mut()
                    .on_magic_instruction(trigger_obj, magic_number)
                    .expect("Error calling magic instruction callback");
            });

            *self.magic_hap_handle_mut() = Some(CoreMagicInstructionHap::add_callback(callback)?);
        } else if !self.needs_magic_hap() {
            if let Some(handle) = self.magic_hap_handle_mut().take() {
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

    /// Callback on magic instruction HAP
    pub fn on_magic_instruction(
        &mut self,
        _trigger_obj: *mut ConfObject,
        magic_number: i64,
    ) -> Result<()> {
        info!(
            self.parent().as_conf_object(),
            "on_magic_instruction({magic_number})"
        );

        Ok(())
    }
}
