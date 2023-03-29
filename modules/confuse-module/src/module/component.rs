//! A component is a part of the module that implements some discrete functionality. For example,
//! the branch tracer is one component. The error detector is another component, and so forth.
//! This module defines common traits of each component, because we need to be checkpointable which
//! introduces some constraints.

use super::{
    config::{InitializeConfig, InitializedConfig},
    stop_reason::StopReason,
};
use anyhow::Result;
use confuse_simics_api::{attr_value_t, conf_object_t};

pub trait Component {
    /// Called when a `ClientMessage::Initialize` message is received. A component can use any
    /// necessary info in `initialize_config` to initialize itself and modify the
    /// `initialized_config` as necessary, for example by adding a memory map to share with
    /// the client
    fn on_initialize(
        &mut self,
        initialize_config: &InitializeConfig,
        initialized_config: InitializedConfig,
    ) -> Result<InitializedConfig>;
    /// Called prior to running the simulator with a given input. Components do not need to do
    /// anything with this information, but they can. For example, the redqueen component needs
    /// to inspect the input to establish an I2S (Input-To-State) correspondence.
    fn pre_run(&mut self, data: &[u8]) -> Result<()>;
    /// Called when a `ClientMessage::Reset` message is received. The component should do anything
    /// it needs in order to prepare for the next run during this call.
    fn on_reset(&mut self) -> Result<()>;
    /// Called when a `ClientMessage::Stop` message is received. The component should clean itself
    /// up and do any pre-exit work it needs to do.
    fn on_stop(&mut self, reason: &Option<StopReason>) -> Result<()>;
    /// Called when a processor is added via the external interface
    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()>;
}
