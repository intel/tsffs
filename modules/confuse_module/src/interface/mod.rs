use anyhow::Result;
use const_format::concatcp;
use simics_api::{AttrValue, OwnedMutAttrValuePtr, OwnedMutConfObjectPtr};

use crate::CLASS_NAME;

pub struct ConfuseModuleInterface {
    start: unsafe extern "C" fn(obj: OwnedMutConfObjectPtr),
    add_processor:
        unsafe extern "C" fn(obj: OwnedMutConfObjectPtr, processor: OwnedMutAttrValuePtr),
    add_fault: unsafe extern "C" fn(obj: OwnedMutConfObjectPtr, fault: i64),
}

impl ConfuseModuleInterface {
    pub const INTERFACE_NAME: &str = CLASS_NAME;
    pub const INTERFACE_TYPENAME: &str =
        concatcp!(ConfuseModuleInterface::INTERFACE_NAME, "_interface_t");
}

impl Default for ConfuseModuleInterface {
    fn default() -> Self {
        Self {
            start: callbacks::start,
            add_processor: callbacks::add_processor,
            add_fault: callbacks::add_fault,
        }
    }
}

pub trait Interface {
    /// Called by the SIMICS Python or CLI interface's `start` function, indicates that the module
    /// has been fully configured and should enter the fuzzing loop.
    fn start(&mut self) -> Result<()>;

    /// Add a processor to the module's context. This method is called by the interface's
    /// `add_processor` function, which must be called for each processor running in the
    /// simulation before the fuzzer can be started
    fn add_processor(&mut self, processor: OwnedMutAttrValuePtr) -> Result<()>;

    /// Add a known fault to the module's context. This method is called by the interface's
    /// `add_fault` function, which must be called for each fault that indicates a crash when it
    /// occurs during fuzzing
    fn add_fault(&mut self, fault: i64) -> Result<()>;
}

mod callbacks {
    use crate::module::Confuse;
    use simics_api::{OwnedMutAttrValuePtr, OwnedMutConfObjectPtr};

    use super::Interface;

    #[no_mangle]
    /// Invoked by SIMICs through the interface binding. This function signals the module to run
    pub extern "C" fn start(obj: OwnedMutConfObjectPtr) {
        let confuse: &mut Confuse = obj.into();
        confuse
            .start()
            .unwrap_or_else(|e| panic!("Confuse failed to start: {}", e));
    }

    #[no_mangle]
    pub extern "C" fn add_processor(obj: OwnedMutConfObjectPtr, processor: OwnedMutAttrValuePtr) {
        let confuse: &mut Confuse = obj.into();
        confuse
            .add_processor(processor)
            .unwrap_or_else(|e| panic!("Failed to add processor: {}", e));
    }

    #[no_mangle]
    pub extern "C" fn add_fault(obj: OwnedMutConfObjectPtr, fault: i64) {
        let confuse: &mut Confuse = obj.into();
        confuse
            .add_fault(fault)
            .unwrap_or_else(|e| panic!("Failed to add fault: {}", e));
    }
}
