use std::hash::{Hash, Hasher};

use crate::{
    config::{InputConfig, OutputConfig},
    module::components::{detector::Detector, tracer::Tracer},
    stops::StopReason,
};
use anyhow::Result;
use simics_api::OwnedMutAttrValuePtr;

pub trait ConfuseState {
    /// Callback when the module's state is [`ConfuseModuleState::HalfInitialized`]. The
    /// input config comes from the client, and the output config is modified by each
    /// [`Component`], where the last component's output configuration is returned to
    /// the client containing any information it needs
    fn on_initialize(
        &mut self,
        input_config: &InputConfig,
        output_config: OutputConfig,
    ) -> Result<OutputConfig> {
        Ok(output_config)
    }

    /// Callback when the module is ready to run, it has hit the first [`Magic`] instruction and
    /// can be started.
    fn on_ready(&mut self) -> Result<()> {
        Ok(())
    }

    /// Callback when execution has stopped, with some reason
    fn on_stopped(&mut self, reason: StopReason) -> Result<()> {
        Ok(())
    }

    /// Callback when the module has ben signaled to exit by the client
    fn on_exit(&mut self) -> Result<()> {
        Ok(())
    }
}

pub trait ConfuseInterface {
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_add_processor(&mut self, processor: OwnedMutAttrValuePtr) -> Result<()> {
        Ok(())
    }

    fn on_add_fault(&mut self, fault: i64) -> Result<()> {
        Ok(())
    }
}

/// Trait for disassemblers of various architectures to implement to permit branch
/// and compare tracing
pub trait TracerDisassembler {
    fn disassemble(&mut self, bytes: &[u8]) -> Result<()>;
    fn last_was_control_flow(&self) -> Result<bool>;
    fn last_was_call(&self) -> Result<bool>;
    fn last_was_ret(&self) -> Result<bool>;
    fn last_was_cmp(&self) -> Result<bool>;
}
