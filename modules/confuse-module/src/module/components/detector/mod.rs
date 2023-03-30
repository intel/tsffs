use self::fault::{Fault, X86_64Fault};
use crate::module::{
    component::{Component, ComponentInterface},
    config::{InitializeConfig, InitializedConfig},
    controller::{Controller, DETECTOR},
    stop_reason::StopReason,
};
use anyhow::Result;
use confuse_simics_api::{attr_value_t, conf_object_t, SIM_hap_add_callback};
use raw_cstr::raw_cstr;
use std::ffi::CString;
use std::{collections::HashSet, mem::transmute, ptr::null_mut, sync::MutexGuard};

pub mod fault;

pub struct FaultDetector {
    /// The set of faults that are considered crashes for this fuzzing campaign
    pub faults: HashSet<Fault>,
    /// The duration after the start harness to treat as a timeout, in seconds
    /// Use `set_timeout_seconds` or `set_timeout_milliseconds` instead of
    /// doing the math yourself!
    pub timeout: Option<f64>,
}

impl Default for FaultDetector {
    fn default() -> Self {
        Self {
            faults: HashSet::new(),
            timeout: None,
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

    pub fn on_exception(&mut self, exception: i64) -> Result<()> {
        // TODO: Arch independent
        if let Ok(fault) = X86_64Fault::try_from(exception) {
            let fault = Fault::X86_64(fault);
            if self.faults.contains(&fault) {
                let mut controller = Controller::get()?;
                unsafe { controller.stop_simulation(StopReason::Crash(fault)) };
            }
        }
        Ok(())
    }
}

impl Component for FaultDetector {
    fn on_initialize(
        &mut self,
        initialize_config: &InitializeConfig,
        initialized_config: InitializedConfig,
    ) -> Result<InitializedConfig> {
        unsafe {
            SIM_hap_add_callback(
                raw_cstr!("Core_Exception"),
                transmute(callbacks::core_exception_cb as unsafe extern "C" fn(_, _, _)),
                null_mut(),
            )
        };
        Ok(initialized_config)
    }

    fn pre_run(&mut self, data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn on_reset(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self, reason: Option<StopReason>) -> Result<()> {
        Ok(())
    }

    fn pre_first_run(&mut self) -> Result<()> {
        Ok(())
    }
}

impl ComponentInterface for FaultDetector {
    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()> {
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
}
