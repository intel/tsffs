use crate::{
    config::OutputConfig,
    faults::{x86_64::X86_64Fault, Fault},
    module::Confuse,
    processor::Processor,
    stops::{StopError, StopReason},
    traits::{ConfuseInterface, ConfuseState},
    CLASS_NAME,
};
use anyhow::{bail, ensure, Result};
use log::{debug, info, trace};
use raffl_macro::{callback_wrappers, params};
use simics_api::{
    attr_object_or_nil_from_ptr, break_simulation, event::register_event, event_cancel_time,
    event_find_next_time, event_post_time, get_class, get_processor_number, hap_add_callback,
    object_clock, AttrValue, ConfObject, CoreExceptionCallback, EventClass, EventFlags, Hap,
    HapCallback, X86TripleFaultCallback,
};
use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr::null_mut,
};

#[derive(Default)]
pub struct Detector {
    pub faults: HashSet<Fault>,
    pub timeout_seconds: Option<f64>,
    pub timeout_event: Option<*mut EventClass>,
    pub processors: HashMap<i32, Processor>,
    pub stop_reason: Option<StopReason>,
}

impl Detector {
    const TIMEOUT_EVENT_NAME: &str = "detector_timeout_event";

    pub fn try_new() -> Result<Self> {
        Ok(Detector::default())
    }
}

impl<'a> From<*mut std::ffi::c_void> for &'a mut Detector {
    /// Convert from a *mut Confuse pointer to a mutable reference to Detector
    fn from(value: *mut std::ffi::c_void) -> &'a mut Detector {
        let confuse_ptr: *mut Confuse = value as *mut Confuse;
        let confuse = unsafe { &mut *confuse_ptr };
        &mut confuse.detector
    }
}

impl ConfuseState for Detector {
    fn on_initialize(
        &mut self,
        confuse: *mut ConfObject,
        input_config: &crate::config::InputConfig,
        output_config: crate::config::OutputConfig,
    ) -> Result<OutputConfig> {
        self.faults = input_config.faults.clone();
        self.timeout_seconds = Some(input_config.timeout);

        let func: CoreExceptionCallback = detector_callbacks::on_exception;
        let _core_handle = hap_add_callback(
            Hap::CoreException,
            HapCallback::CoreException(func),
            Some(confuse as *mut c_void),
        )?;

        if self.faults.contains(&Fault::X86_64(X86_64Fault::Triple)) {
            let func: X86TripleFaultCallback = detector_callbacks::on_x86_triple_fault;

            let _triple_handle = hap_add_callback(
                Hap::X86TripleFault,
                HapCallback::X86TripleFault(func),
                Some(confuse as *mut c_void),
            )?;
        }

        info!("Initialized Detector");

        Ok(output_config)
    }

    fn pre_first_run(&mut self, confuse: *mut ConfObject) -> Result<()> {
        let confuse_cls = get_class(CLASS_NAME)?;

        let event = register_event(
            Detector::TIMEOUT_EVENT_NAME,
            confuse_cls,
            detector_callbacks::on_timeout_event,
            &[],
        )?;

        self.timeout_event = Some(event);

        Ok(())
    }

    fn on_ready(&mut self, confuse: *mut ConfObject) -> Result<()> {
        self.stop_reason = None;
        Ok(())
    }

    fn on_run(&mut self, confuse: *mut ConfObject) -> Result<()> {
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
                        Some(confuse as *mut c_void),
                    );
                }
            }
        }

        trace!("Done setting up detector");

        Ok(())
    }

    fn on_stopped(&mut self, confuse: *mut ConfObject, reason: StopReason) -> Result<()> {
        trace!("Detector handling stop with reason {:?}", reason);

        if let Some(timeout_event) = self.timeout_event {
            if !timeout_event.is_null() {
                for (processor_number, processor) in &self.processors {
                    let clock = object_clock(processor.cpu())?;

                    if let Ok(remaining) = event_find_next_time(clock, timeout_event, confuse) {
                        debug!(
                            "Remaining time on stop for processor {}: {} seconds, cancelling event",
                            processor_number, remaining
                        );

                        event_cancel_time(clock, timeout_event, confuse);
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

impl ConfuseInterface for Detector {
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
        let fault = Fault::X86_64(X86_64Fault::try_from(fault)?);
        info!("Detector adding fault {:?}", fault);
        // TODO: Arch independent
        self.faults.insert(fault);
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
}
