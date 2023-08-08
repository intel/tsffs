// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::OutputConfig,
    faults::{x86_64::X86_64Fault, Fault},
    module::Module,
    processor::Processor,
    stops::StopReason,
    traits::{Interface, State},
    CLASS_NAME,
};
use anyhow::{bail, Result};
use ffi_macro::{callback_wrappers, params};
use simics_api::{
    attr_object_or_nil_from_ptr, break_simulation, event::register_event, event_cancel_time,
    event_find_next_time, event_post_time, get_class, get_processor_number, hap_add_callback,
    object_clock, AttrValue, ConfObject, CoreExceptionCallback, EventClass, GenericTransaction,
    Hap, HapCallback, X86TripleFaultCallback,
};
use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
};
use tracing::{debug, error, info, trace};

#[derive(Default)]
pub struct Detector {
    pub faults: HashSet<Fault>,
    pub timeout_seconds: Option<f64>,
    pub timeout_event: Option<*mut EventClass>,
    pub processors: HashMap<i32, Processor>,
    pub stop_reason: Option<StopReason>,
    pub exception_cb_added: bool,
    pub triple_cb_added: bool,
    pub module: Option<*mut ConfObject>,
    pub breakpoints_are_faults: bool,
}

impl Detector {
    const TIMEOUT_EVENT_NAME: &str = "detector_timeout_event";

    pub fn try_new() -> Result<Self> {
        Ok(Detector::default())
    }
}

impl<'a> From<*mut std::ffi::c_void> for &'a mut Detector {
    /// Convert from a *mut Module pointer to a mutable reference to Detector
    fn from(value: *mut std::ffi::c_void) -> &'a mut Detector {
        let module_ptr: *mut Module = value as *mut Module;
        let module = unsafe { &mut *module_ptr };
        &mut module.detector
    }
}

impl State for Detector {
    fn on_initialize(
        &mut self,
        module: *mut ConfObject,
        input_config: &mut crate::config::InputConfig,
        output_config: crate::config::OutputConfig,
    ) -> Result<OutputConfig> {
        self.faults = input_config.faults.clone();
        self.timeout_seconds = Some(input_config.timeout);
        self.module = Some(module);

        for fault in self.faults.clone() {
            self.on_add_fault(fault.try_into()?)?;
        }

        info!("Initialized Detector");

        Ok(output_config)
    }

    fn pre_first_run(&mut self, module: *mut ConfObject) -> Result<()> {
        let module_cls = get_class(CLASS_NAME)?;

        let event = register_event(
            Detector::TIMEOUT_EVENT_NAME,
            module_cls,
            detector_callbacks::on_timeout_event,
            &[],
        )?;

        let _bp_handle = hap_add_callback(
            Hap::CoreBreakpointMemop,
            HapCallback::CoreBreakpointMemop(detector_callbacks::on_breakpoint_memop),
            Some(module as *mut c_void),
        );

        self.timeout_event = Some(event);

        Ok(())
    }

    fn on_ready(&mut self, _module: *mut ConfObject) -> Result<()> {
        self.stop_reason = None;
        Ok(())
    }

    fn on_run(&mut self, module: *mut ConfObject) -> Result<()> {
        trace!("Setting up Detector before run");

        self.stop_reason = None;

        if let Some(timeout_event) = self.timeout_event {
            if let Some(timeout_seconds) = self.timeout_seconds {
                for (processor_number, processor) in &self.processors {
                    let clock = object_clock(processor.cpu())?;

                    trace!(
                        "Setting up timeout event with {} seconds on processor #{}",
                        timeout_seconds,
                        processor_number
                    );

                    event_post_time(
                        clock,
                        timeout_event,
                        processor.cpu(),
                        timeout_seconds,
                        Some(module as *mut c_void),
                    );
                }
            }
        }

        trace!("Done setting up detector");

        Ok(())
    }

    fn on_stopped(&mut self, module: *mut ConfObject, reason: StopReason) -> Result<()> {
        trace!("Detector handling stop with reason {:?}", reason);

        if let Some(timeout_event) = self.timeout_event {
            if !timeout_event.is_null() {
                for (processor_number, processor) in &self.processors {
                    let clock = object_clock(processor.cpu())?;

                    if let Ok(remaining) = event_find_next_time(clock, timeout_event, module) {
                        debug!(
                            "Remaining time on stop for processor {}: {} seconds, cancelling event",
                            processor_number, remaining
                        );

                        event_cancel_time(clock, timeout_event, module);
                    } else {
                        debug!("Stopped without timeout event, unable to cancel nonexistent event");
                    }
                }
            } else {
                debug!("Timeout event is null, not initialized yet, so skipping cancellation");
            }
        }
        Ok(())
    }
}

impl Interface for Detector {
    fn on_add_processor(&mut self, processor_attr: *mut AttrValue) -> Result<()> {
        let processor_obj: *mut ConfObject = attr_object_or_nil_from_ptr(processor_attr)?;
        let processor_number = get_processor_number(processor_obj);

        // Don't need any instrumentation for the detector
        let processor = Processor::try_new(processor_number, processor_obj)?;

        self.processors.insert(processor_number, processor);

        info!("Detector added processor #{}", processor_number);

        Ok(())
    }

    fn on_add_fault(&mut self, fault: i64) -> Result<()> {
        let fault = Fault::X86_64(X86_64Fault::try_from(fault).map_err(|e| {
            error!("Failed to get fault for fault number {}", fault);
            e
        })?);

        info!("Detector adding fault {:?}", fault);
        // TODO: Arch independent
        self.faults.insert(fault);

        if let Some(module) = self.module {
            if let Fault::X86_64(X86_64Fault::Triple) = fault {
                if !self.triple_cb_added {
                    // Add the triple cb
                    let func: X86TripleFaultCallback = detector_callbacks::on_x86_triple_fault;

                    let _triple_handle = hap_add_callback(
                        Hap::X86TripleFault,
                        HapCallback::X86TripleFault(func),
                        Some(module as *mut c_void),
                    )?;
                    self.triple_cb_added = true;
                }
            } else if !self.exception_cb_added {
                // Add the standard cb
                let func: CoreExceptionCallback = detector_callbacks::on_exception;

                let _core_handle = hap_add_callback(
                    Hap::CoreException,
                    HapCallback::CoreException(func),
                    Some(module as *mut c_void),
                )?;
                self.exception_cb_added = true;
            }
        }

        Ok(())
    }

    fn on_set_breakpoints_are_faults(&mut self, breakpoints_are_faults: bool) -> Result<()> {
        self.breakpoints_are_faults = breakpoints_are_faults;
        Ok(())
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl Detector {
    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_exception(
        &mut self,
        trigger_obj: *mut ConfObject,
        exception_number: i64,
    ) -> Result<()> {
        let cpu: *mut ConfObject = trigger_obj;

        let processor_number = get_processor_number(cpu);

        if let Some(processor) = self.processors.get(&processor_number) {
            match processor.arch().as_ref() {
                "x86-64" => {
                    if let Ok(fault) = X86_64Fault::try_from(exception_number) {
                        let fault = Fault::X86_64(fault);
                        if self.faults.contains(&fault) {
                            info!("Got exception with fault: {:?}", fault);
                            self.stop_reason = Some(StopReason::Crash((fault, processor_number)));
                            break_simulation("crash")?;
                        }
                    }
                }
                _ => {
                    bail!("Unsupported architecture");
                }
            }
        }

        Ok(())
    }

    #[params(..., !slf: *mut std::ffi::c_void)]
    pub fn on_timeout_event(&mut self, _obj: *mut ConfObject) -> Result<()> {
        info!("Got timeout event");
        self.stop_reason = Some(StopReason::TimeOut);
        break_simulation("timeout")?;
        Ok(())
    }

    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_x86_triple_fault(&mut self, trigger_obj: *mut ConfObject) -> Result<()> {
        info!("Got triple fault");
        let processor_number = get_processor_number(trigger_obj);
        self.stop_reason = Some(StopReason::Crash((
            Fault::X86_64(X86_64Fault::Triple),
            processor_number,
        )));
        break_simulation("triple")?;
        Ok(())
    }

    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_breakpoint_memop(
        &mut self,
        _trigger_obj: *mut ConfObject,
        breakpoint_number: i64,
        _memop: *mut GenericTransaction,
    ) -> Result<()> {
        if self.breakpoints_are_faults {
            info!("Got breakpoint");
            // TODO: Use trigger_obj (which is cpu?) to get address of the bp directly, memory op info
            // and so forth? Or we can just have people use repro for this and save the trouble.
            self.stop_reason = Some(StopReason::Breakpoint(breakpoint_number));
            break_simulation("breakpoint")?;
        }
        Ok(())
    }
}
