use self::fault::{Fault, X86_64Fault};
use crate::module::{
    component::{Component, ComponentInterface},
    config::{InputConfig, OutputConfig},
    controller::{instance::ControllerInstance, Controller, DETECTOR},
    cpu::Cpu,
    stop_reason::StopReason,
};
use anyhow::{ensure, Context, Result};
use confuse_simics_api::{
    attr_value_t, conf_object_t, event_class_t,
    safe::{
        common::hap_add_callback_core_exception,
        wrapper::{
            event_cancel_time, event_find_next_time, event_post_time, get_class, object_clock,
            register_event,
        },
    },
};
use log::info;
use std::{cell::RefCell, ptr::null_mut};
use std::{collections::HashSet, sync::MutexGuard};

pub mod fault;

struct TimeoutEvent {
    ptr: *mut event_class_t,
}

impl Default for TimeoutEvent {
    fn default() -> Self {
        Self { ptr: null_mut() }
    }
}

impl TimeoutEvent {
    pub unsafe fn from_ptr(ptr: *mut event_class_t) -> Self {
        Self { ptr }
    }

    pub fn _get(&self) -> *mut event_class_t {
        self.ptr
    }

    pub fn as_mut_ref(&mut self) -> &mut event_class_t {
        unsafe { &mut *self.ptr }
    }
}

#[derive(Default)]
/// Component of the Confuse module that detects faults, timeouts, and other error conditions
pub struct FaultDetector {
    /// The set of faults that are considered crashes for this fuzzing campaign
    pub faults: HashSet<Fault>,
    /// The duration after the start harness to treat as a timeout, in seconds
    /// Use `set_timeout_seconds` or `set_timeout_milliseconds` instead of
    /// doing the math yourself!
    pub timeout: Option<f64>,
    /// The registered timeout event
    timeout_event: RefCell<TimeoutEvent>,
    pub cpus: Vec<RefCell<Cpu>>,
}

unsafe impl Send for FaultDetector {}
unsafe impl Sync for FaultDetector {}

impl FaultDetector {
    /// The name of the timeout event this component uses to post events to the SIMICS event queue
    const TIMEOUT_EVENT_NAME: &str = "timeout_event";

    /// Get the global instance of this component
    pub fn get<'a>() -> Result<MutexGuard<'a, Self>> {
        let detector = DETECTOR.lock().expect("Could not lock detector");
        Ok(detector)
    }
}

impl FaultDetector {
    /// Try to instantiate a new instance of this component
    pub fn try_new() -> Result<Self> {
        Ok(FaultDetector::default())
    }
}

impl FaultDetector {
    /// Method triggered by the Core_Exception HAP. Checks if the exception is a fault we
    /// care about (registered in the `InputConfig` or via the `add_fault` interface method)
    /// and reports it to the controller if it is
    pub fn on_exception(&mut self, exception: i64) -> Result<()> {
        // TODO: Make arch independent
        if let Ok(fault) = X86_64Fault::try_from(exception) {
            let fault = Fault::X86_64(fault);
            info!("Got exception with fault: {:?}", fault);
            if self.faults.contains(&fault) {
                let mut controller = Controller::get()?;
                controller.stop_simulation(StopReason::Crash(fault));
            }
        }
        Ok(())
    }

    /// Method triggered by a timeout event expiring.
    pub fn on_timeout_event(&mut self) -> Result<()> {
        let mut controller = Controller::get()?;
        controller.stop_simulation(StopReason::TimeOut);
        Ok(())
    }
}

impl Component for FaultDetector {
    fn on_initialize(
        &mut self,
        input_config: &InputConfig,
        output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        self.faults = input_config.faults.clone();
        self.timeout = Some(input_config.timeout);

        hap_add_callback_core_exception(callbacks::core_exception_cb)?;

        Ok(output_config)
    }

    unsafe fn pre_run(
        &mut self,
        _data: &[u8],
        instance: Option<&mut ControllerInstance>,
    ) -> Result<()> {
        let clock = object_clock(
            &mut *self
                .cpus
                .first()
                .context("No cpu available")?
                .borrow()
                .get_cpu(),
        )?;

        let event = &mut self.timeout_event.borrow_mut();
        let event = event.as_mut_ref();

        event_post_time(
            clock,
            event,
            instance.context("No instance available")?.get_as_obj(),
            self.timeout.expect("No timeout set"),
        );

        Ok(())
    }

    unsafe fn on_reset(&mut self) -> Result<()> {
        Ok(())
    }

    unsafe fn on_stop(
        &mut self,
        _reason: Option<StopReason>,
        instance: Option<&mut ControllerInstance>,
    ) -> Result<()> {
        let clock = object_clock(
            &mut *self
                .cpus
                .first()
                .context("No cpu available")?
                .borrow()
                .get_cpu(),
        )?;

        let event = &mut self.timeout_event.borrow_mut();
        let event = event.as_mut_ref();
        let obj = instance.context("No instance available")?.get_as_obj();

        let remaining = event_find_next_time(clock, event, obj);

        info!("Remaining time on stop: {}", remaining);

        event_cancel_time(clock, event, obj);

        Ok(())
    }

    unsafe fn pre_first_run(&mut self) -> Result<()> {
        Ok(())
    }
}

impl ComponentInterface for FaultDetector {
    unsafe fn on_run(&mut self, _instance: &ControllerInstance) -> Result<()> {
        let cls = get_class(Controller::CLASS_NAME)?;

        self.timeout_event = RefCell::new(TimeoutEvent::from_ptr(register_event(
            FaultDetector::TIMEOUT_EVENT_NAME,
            cls,
            callbacks::timeout_event_cb,
        )?));

        Ok(())
    }

    unsafe fn on_add_processor(
        &mut self,
        _obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        ensure!(
            self.cpus.is_empty(),
            "A CPU has already been added! This module only supports 1 vCPU at this time."
        );

        self.cpus.push(RefCell::new(Cpu::try_new(processor)?));

        Ok(())
    }

    unsafe fn on_add_fault(&mut self, _obj: *mut conf_object_t, fault: i64) -> Result<()> {
        // TODO: Arch independent
        self.faults
            .insert(Fault::X86_64(X86_64Fault::try_from(fault)?));
        Ok(())
    }
}

mod callbacks {
    use super::FaultDetector;
    use confuse_simics_api::conf_object_t;
    use log::info;
    use std::ffi::c_void;

    #[no_mangle]
    pub extern "C" fn core_exception_cb(
        _data: *mut c_void,
        _trigger_obj: *mut conf_object_t,
        exception_number: i64,
    ) {
        let mut detector = FaultDetector::get().expect("Could not get detector");
        detector
            .on_exception(exception_number)
            .expect("Could not handle exception");
    }

    #[no_mangle]
    pub extern "C" fn timeout_event_cb(_obj: *mut conf_object_t, _data: *mut c_void) {
        info!("Got timeout");
        let mut detector = FaultDetector::get().expect("Could not get detector");
        detector
            .on_timeout_event()
            .expect("Could not handle timeout event");
    }
}
