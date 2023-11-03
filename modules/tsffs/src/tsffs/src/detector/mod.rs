// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use getters::Getters;
use simics::{
    api::{
        get_class, object_clock, AsConfObject, BreakpointId, ConfObject, CoreBreakpointMemopHap,
        CoreExceptionHap, Event, EventClassFlag, GenericTransaction, HapHandle,
    },
    error, info,
};
use simics_macro::{TryFromAttrValueTypeDict, TryIntoAttrValueTypeDict};
use std::collections::BTreeSet;
use typed_builder::TypedBuilder;

use crate::{
    state::{Solution, SolutionKind, StopReason},
    traits::Component,
    Tsffs, CLASS_NAME,
};

/// The timeout runs in virtual time, so a typical 5 second timeout is acceptable
pub const TIMEOUT_DEFAULT: f64 = 5.0;

#[derive(
    TypedBuilder, Getters, Debug, Clone, TryIntoAttrValueTypeDict, TryFromAttrValueTypeDict,
)]
#[getters(mutable)]
/// Configuration of the fuzzer of each condition that can be treated as a fault
pub struct DetectorConfiguration {
    #[builder(default = false)]
    /// Whether any breakpoint that occurs during fuzzing is treated as a fault
    all_breakpoints_are_solutions: bool,
    #[builder(default = false)]
    /// Whether any CPU exception that occurs during fuzzing is treated as a solution
    all_exceptions_are_solutions: bool,
    #[builder(default)]
    /// The set of specific exception numbers that are treated as a solution
    exceptions: BTreeSet<i64>,
    #[builder(default)]
    /// The set of breakpoints to treat as solutions
    breakpoints: BTreeSet<BreakpointId>,
    #[builder(default = TIMEOUT_DEFAULT)]
    /// The amount of time in seconds before a testcase execution is considered "timed
    /// out" and will be treated as a solution
    timeout: f64,
}

impl Default for DetectorConfiguration {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(TypedBuilder, Getters)]
#[getters(mutable)]
pub struct Detector<'a>
where
    'a: 'static,
{
    parent: &'a mut Tsffs,
    #[builder(default)]
    configuration: DetectorConfiguration,
    #[builder(default = {
        let parent_conf_object = parent.as_conf_object_mut();
        CoreBreakpointMemopHap::add_callback(
            move |trigger_obj, breakpoint_number, memop| {
                let tsffs: &'static mut Tsffs = parent_conf_object.into();
                tsffs.detector_mut()
                    .on_breakpoint_memop(trigger_obj, breakpoint_number, memop)
                    .expect("Error calling breakpoint memop callback");
            }
        ).expect("Failed to register breakpoint memop callback")
    })]
    breakpoint_memop_hap_handle: HapHandle,
    #[builder(default = {
        let parent_conf_object = parent.as_conf_object_mut();
        CoreExceptionHap::add_callback(
            move |trigger_obj, exception_number| {
                let tsffs: &'static mut Tsffs = parent_conf_object.into();
                tsffs.detector_mut()
                    .on_exception(trigger_obj, exception_number)
                    .expect("Error calling breakpoint memop callback");
            }
        ).expect("Failed to register breakpoint memop callback")
    })]
    exception_hap_handle: HapHandle,
    #[builder(default)]
    timeout_event_processor: Option<*mut ConfObject>,
    #[builder(default)]
    timeout_event: Option<Event>,
}

impl<'a> Detector<'a> {
    pub const TIMEOUT_EVENT_NAME: &str = "detector_timeout_event";
}

/// Implementations for interface methods
impl<'a> Detector<'a> {
    /// Set a solution condition
    pub fn on_solution<S>(&mut self, id: u64, message: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        info!(
            self.parent_mut().as_conf_object_mut(),
            "on_solution({}, {})",
            id,
            message.as_ref()
        );

        self.parent_mut().stop_simulation(StopReason::Solution(
            Solution::builder().kind(SolutionKind::Manual).build(),
        ))?;

        Ok(())
    }
}

impl<'a> Detector<'a>
where
    'a: 'static,
{
    pub fn on_exception(&mut self, _obj: *mut ConfObject, exception: i64) -> Result<()> {
        if *self.configuration().all_exceptions_are_solutions()
            || self.configuration().exceptions().contains(&exception)
        {
            self.parent_mut().stop_simulation(StopReason::Solution(
                Solution::builder().kind(SolutionKind::Exception).build(),
            ))?;
        }
        Ok(())
    }

    pub fn on_breakpoint_memop(
        &mut self,
        obj: *mut ConfObject,
        breakpoint: i64,
        transaction: *mut GenericTransaction,
    ) -> Result<()> {
        if *self.configuration().all_breakpoints_are_solutions()
            || self
                .configuration()
                .breakpoints()
                .contains(&(breakpoint as i32))
        {
            info!(
                self.parent_mut().as_conf_object_mut(),
                "on_breakpoint_memop({:#x}, {}, {:#x})",
                obj as usize,
                breakpoint,
                transaction as usize
            );
            self.parent_mut().stop_simulation(StopReason::Solution(
                Solution::builder().kind(SolutionKind::Breakpoint).build(),
            ))?;
        }
        Ok(())
    }

    pub fn on_timeout(&mut self, _obj: *mut ConfObject) -> Result<()> {
        self.parent_mut().stop_simulation(StopReason::Solution(
            Solution::builder().kind(SolutionKind::Timeout).build(),
        ))?;
        Ok(())
    }
}

impl<'a> Component for Detector<'a> {
    fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
        match reason {
            StopReason::MagicStart(start) | StopReason::Start(start) => {
                if self.timeout_event().is_none() {
                    *self.timeout_event_mut() = Some(
                        Event::builder()
                            .name(Detector::TIMEOUT_EVENT_NAME)
                            .cls(get_class(CLASS_NAME).expect("Error getting class"))
                            .flags(EventClassFlag::Sim_EC_No_Flags)
                            .build(),
                    );
                    *self.timeout_event_processor_mut() = Some(*start.processor());
                    info!(self.parent().as_conf_object(), "Registered timeout event");
                }
                // On start, we set a timeout event
                let parent_conf_object = self.parent_mut().as_conf_object_mut();

                if let Some(event) = self.timeout_event() {
                    event.post_time(
                        *start.processor(),
                        object_clock(*start.processor())?,
                        *self.configuration().timeout(),
                        move |obj| {
                            let tsffs: &'static mut Tsffs = parent_conf_object.into();
                            info!(tsffs.as_conf_object_mut(), "timeout({:#x})", obj as usize);
                            tsffs
                                .detector_mut()
                                .on_timeout(obj)
                                .expect("Error calling timeout callback");
                        },
                    )?;
                    info!(
                        self.parent().as_conf_object(),
                        "Posted timeout for {}s",
                        self.configuration().timeout()
                    );
                }
            }

            StopReason::MagicStop(_stop) | StopReason::Stop(_stop) => {
                // On stop, we clear the timeout event
                if let Some(timeout_event_processor) = self.timeout_event_processor() {
                    if let Some(event) = self.timeout_event() {
                        match event.find_next_time(
                            object_clock(*timeout_event_processor)?,
                            *timeout_event_processor,
                        ) {
                            Ok(next_time) => {
                                info!(
                                    self.parent().as_conf_object(),
                                    "Stopped with {next_time:.02}s remaining until timeout"
                                );
                            }
                            Err(e) => error!(
                                self.parent().as_conf_object(),
                                "Error getting next time for clock: {}", e
                            ),
                        }

                        event.cancel_time(
                            *timeout_event_processor,
                            object_clock(*timeout_event_processor)?,
                        )?;
                    }
                }
            }

            StopReason::Solution(_) => {
                if let Some(timeout_event_processor) = self.timeout_event_processor() {
                    if let Some(event) = self.timeout_event() {
                        match event.find_next_time(
                            object_clock(*timeout_event_processor)?,
                            *timeout_event_processor,
                        ) {
                            Ok(next_time) => {
                                info!(
                                    self.parent().as_conf_object(),
                                    "Stopped with {next_time:.02}s remaining until timeout"
                                );
                            }
                            Err(e) => error!(
                                self.parent().as_conf_object(),
                                "Error getting next time for clock: {}", e
                            ),
                        }

                        event.cancel_time(
                            *timeout_event_processor,
                            object_clock(*timeout_event_processor)?,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}
