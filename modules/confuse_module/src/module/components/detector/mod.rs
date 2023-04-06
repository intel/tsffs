use self::fault::{Fault, X86_64Fault};
use crate::{
    module::{
        component::{Component, ComponentInterface},
        config::{InputConfig, OutputConfig},
        controller::{instance::ControllerInstance, Controller, DETECTOR},
        cpu::Cpu,
        stop_reason::StopReason,
    },
    nonnull,
};
use anyhow::{bail, ensure, Context, Result};
use confuse_simics_api::{
    attr_value_t, conf_class_t, conf_object_t, event_class_flag_t_Sim_EC_Notsaved, event_class_t,
    SIM_event_cancel_time, SIM_event_find_next_time, SIM_event_post_time, SIM_hap_add_callback,
    SIM_object_class, SIM_object_clock, SIM_register_event,
};
use log::{info, trace};
use raw_cstr::raw_cstr;
use std::{
    cell::RefCell,
    ffi::CString,
    sync::{Arc, Mutex},
};
use std::{collections::HashSet, mem::transmute, ptr::null_mut, sync::MutexGuard};

pub mod fault;

pub struct FaultDetector {
    /// The set of faults that are considered crashes for this fuzzing campaign
    pub faults: HashSet<Fault>,
    /// The duration after the start harness to treat as a timeout, in seconds
    /// Use `set_timeout_seconds` or `set_timeout_milliseconds` instead of
    /// doing the math yourself!
    pub timeout: Option<f64>,
    /// The registered timeout event
    pub timeout_event: *mut event_class_t,
    pub cpus: Vec<RefCell<Cpu>>,
    pub processor_cb_obj: *mut conf_object_t,
    pub processor_cb_cls: *mut conf_class_t,
}

unsafe impl Send for FaultDetector {}
unsafe impl Sync for FaultDetector {}

impl Default for FaultDetector {
    fn default() -> Self {
        Self {
            faults: HashSet::new(),
            timeout: None,
            timeout_event: null_mut(),
            cpus: vec![],
            processor_cb_obj: null_mut(),
            processor_cb_cls: null_mut(),
        }
    }
}

impl FaultDetector {
    pub fn get<'a>() -> Result<MutexGuard<'a, Self>> {
        let detector = DETECTOR.lock().expect("Could not lock detector");
        Ok(detector)
    }
}

impl FaultDetector {
    pub fn try_new() -> Result<Self> {
        Ok(FaultDetector::default())
    }
}

impl FaultDetector {
    pub fn on_exception(&mut self, exception: i64) -> Result<()> {
        // TODO: Arch independent
        if let Ok(fault) = X86_64Fault::try_from(exception) {
            let fault = Fault::X86_64(fault);
            info!("Got exception with fault: {:?}", fault);
            if self.faults.contains(&fault) {
                let mut controller = Controller::get()?;
                unsafe { controller.stop_simulation(StopReason::Crash(fault)) };
            }
        }
        Ok(())
    }

    pub fn on_timeout_event(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Component for FaultDetector {
    fn on_initialize(
        &mut self,
        input_config: &InputConfig,
        output_config: OutputConfig,
        controller_cls: Option<*mut conf_class_t>,
    ) -> Result<OutputConfig> {
        self.faults = input_config.faults.clone();
        self.timeout = Some(input_config.timeout);

        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Exception"),
                transmute(callbacks::core_exception_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };

        Ok(output_config)
    }

    unsafe fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        Ok(())
    }

    unsafe fn on_reset(&mut self) -> Result<()> {
        Ok(())
    }

    unsafe fn on_stop(&mut self, reason: Option<StopReason>) -> Result<()> {
        Ok(())
    }

    unsafe fn pre_first_run(&mut self) -> Result<()> {
        Ok(())
    }
}

impl ComponentInterface for FaultDetector {
    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
        ensure!(
            self.cpus.is_empty(),
            "A CPU has already been added! This module only supports 1 vCPU at this time."
        );

        info!("Adding processor for fault detector");
        let cls = nonnull!(unsafe { SIM_object_class(obj) });
        self.processor_cb_obj = obj;
        self.processor_cb_cls = cls;

        self.cpus.push(RefCell::new(Cpu::try_new(processor)?));

        Ok(())
    }

    unsafe fn on_add_fault(&mut self, obj: *mut conf_object_t, fault: i64) -> Result<()> {
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
