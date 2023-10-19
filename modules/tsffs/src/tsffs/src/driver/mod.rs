// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use getters::Getters;
use simics::{
    api::{
        AsAttrValueType, AsConfObject, ConfObject, CoreMagicInstructionHap, GenericAddress, Hap,
        HapHandle,
    },
    info,
};
use simics_macro::AsAttrValueType;
use typed_builder::TypedBuilder;

use crate::Tsffs;

#[derive(TypedBuilder, Getters, Clone, Debug, AsAttrValueType)]
#[getters(mutable)]
pub struct DriverConfiguration {
    #[builder(default = false)]
    start_on_harness: bool,
    #[builder(default = false)]
    stop_on_harness: bool,
    #[builder(default = false)]
    use_snapshots: bool,
}

impl Default for DriverConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters, Clone, Debug, AsAttrValueType)]
#[getters(mutable)]
pub struct BufferInformation {
    #[builder(default)]
    testcase_address: Option<GenericAddress>,
    #[builder(default)]
    size_address: Option<GenericAddress>,
    #[builder(default)]
    maximum_size: Option<usize>,
    #[builder(default)]
    virt: Option<bool>,
}

impl Default for BufferInformation {
    fn default() -> Self {
        Self::builder().build()
    }
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
    fn add_magic_hap_if_needed(&mut self) -> Result<()> {
        if (*self.configuration().start_on_harness() || *self.configuration().stop_on_harness())
            && self.magic_hap_handle().is_none()
        {
            let parent = self.parent().as_conf_object();

            let callback = Box::new(move |trigger_obj, magic_number| {
                let tsffs: &'static mut Tsffs = (parent as *mut ConfObject).into();
                tsffs
                    .driver_mut()
                    .on_magic_instruction(trigger_obj, magic_number)
                    .expect("Error calling magic instruction callback");
            });

            *self.magic_hap_handle_mut() = Some(CoreMagicInstructionHap::add_callback(callback)?);
        } else if let Some(handle) = self.magic_hap_handle_mut().take() {
            CoreMagicInstructionHap::delete_callback_id(handle)?;
        }

        Ok(())
    }

    pub fn set_start_on_harness(&mut self, start_on_harness: bool) -> Result<()> {
        *self.configuration_mut().start_on_harness_mut() = start_on_harness;
        self.add_magic_hap_if_needed()
    }

    pub fn set_stop_on_harness(&mut self, stop_on_harness: bool) -> Result<()> {
        *self.configuration_mut().stop_on_harness_mut() = stop_on_harness;
        self.add_magic_hap_if_needed()
    }

    /// Callback on magic instruction
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
