use crate::{
    config::OutputConfig,
    faults::{x86_64::X86_64Fault, Fault},
    module::Confuse,
    processor::Processor,
    stops::StopReason,
    traits::{ConfuseInterface, ConfuseState},
};
use anyhow::{bail, Result};
use log::info;
use raffl_macro::{callback_wrappers, params};
use simics_api::{
    attr_object_or_nil_from_ptr, get_processor_number, hap_add_callback, ConfClass, ConfObject,
    CoreExceptionCallback, Hap, ObjHapFunc, OwnedMutAttrValuePtr, OwnedMutConfObjectPtr,
    X86TripleFaultCallback,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Detector {
    pub faults: HashSet<Fault>,
    pub timeout_seconds: Option<f64>,
    pub timeout_event: Option<ConfClass>,
    pub processors: HashMap<i32, Processor>,
}

impl Detector {
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
        confuse: OwnedMutConfObjectPtr,
        input_config: &crate::config::InputConfig,
        output_config: crate::config::OutputConfig,
    ) -> Result<OutputConfig> {
        self.faults = input_config.faults.clone();
        self.timeout_seconds = Some(input_config.timeout);

        let func: CoreExceptionCallback = detector_callbacks::on_exception;
        let _core_handle =
            hap_add_callback(Hap::CoreException, func.into(), Some(confuse.clone()))?;

        if self.faults.contains(&Fault::X86_64(X86_64Fault::Triple)) {
            let func: X86TripleFaultCallback = detector_callbacks::on_x86_triple_fault;
            let _triple_handle =
                hap_add_callback(Hap::X86TripleFault, func.into(), Some(confuse.clone()))?;
        }

        Ok(output_config)
    }
}

impl ConfuseInterface for Detector {
    fn on_add_processor(
        &mut self,
        _confuse: OwnedMutConfObjectPtr,
        processor_attr: OwnedMutAttrValuePtr,
    ) -> Result<()> {
        let processor_obj: OwnedMutConfObjectPtr =
            attr_object_or_nil_from_ptr(processor_attr.clone())?;
        let processor_number = get_processor_number(&processor_obj);
        let mut processor = Processor::try_new(processor_number, &processor_obj)?
            .try_with_cpu_instrumentation_subscribe(processor_attr)?;

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
        let cpu: OwnedMutConfObjectPtr = trigger_obj.into();
        let processor_number = get_processor_number(&cpu);
        if let Some(processor) = self.processors.get_mut(&processor_number) {
            match processor.arch().as_ref() {
                "x86-64" => {
                    if let Ok(fault) = X86_64Fault::try_from(exception_number) {
                        let fault = Fault::X86_64(fault);
                        info!("Got exception with fault: {:?}", fault);
                        if self.faults.contains(&fault) {
                            // confuse.stop_simulation(StopReason::Crash(fault));
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
    pub fn on_timeout_event(&mut self, obj: *mut ConfObject) -> Result<()> {
        Ok(())
    }

    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn on_x86_triple_fault(&mut self, trigger_obj: *mut ConfObject) -> Result<()> {
        Ok(())
    }
}
