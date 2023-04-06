//! A component is a part of the module that implements some discrete functionality. For example,
//! the branch tracer is one component. The error detector is another component, and so forth.
//! This module defines common traits of each component, because we need to be checkpointable which
//! introduces some constraints.

use super::{
    config::{InputConfig, OutputConfig},
    controller::instance::ControllerInstance,
    stop_reason::StopReason,
};
use anyhow::Result;
use confuse_simics_api::{attr_value_t, conf_class_t, conf_object_t};

/// A trait defining the functions a component needs to implement so it can initialize itself
/// from the global configuration and react to events that happen
pub trait Component {
    /// Called when a `ClientMessage::Initialize` message is received. A component can use any
    /// necessary info in `input_config` to initialize itself and modify the
    /// `output_config` as necessary, for example by adding a memory map to share with
    /// the client
    fn on_initialize(
        &mut self,
        input_config: &InputConfig,
        output_config: OutputConfig,
        controller_cls: Option<*mut conf_class_t>,
    ) -> Result<OutputConfig>;
    /// Called prior to the first time run of the simulator. This function allows components to
    /// do any last-minute configuration that depends on possible user configurations. For example
    /// the fault detector may need the list of faults to be fully set up before registering
    /// various additional functionality with SIMICS. A snapshot is not taken until after all
    /// components `pre_first_run` functions have been run.
    unsafe fn pre_first_run(&mut self, controller_instance: &ControllerInstance) -> Result<()>;
    /// Called prior to running the simulator with a given input. Components do not need to do
    /// anything with this information, but they can. For example, the redqueen component needs
    /// to inspect the input to establish an I2S (Input-To-State) correspondence. This function
    /// is called before every run.
    unsafe fn pre_run(
        &mut self,
        controller_instance: &ControllerInstance,
        data: &[u8],
    ) -> Result<()>;
    /// Called when a `ClientMessage::Reset` message is received. The component should do anything
    /// it needs in order to prepare for the next run during this call.
    unsafe fn on_reset(&mut self, controller_instance: &ControllerInstance) -> Result<()>;
    /// Called when a `ClientMessage::Stop` message is received. The component should clean itself
    /// up and do any pre-exit work it needs to do.
    unsafe fn on_stop(
        &mut self,
        controller_instance: &ControllerInstance,
        reason: Option<StopReason>,
    ) -> Result<()>;
}

/// A trait defining the functions a component needs to implement to react to functions called
/// on the component interface with the outside world.
pub trait ComponentInterface {
    /// Called when a processor is added via the external interface
    unsafe fn on_add_processor(
        &mut self,
        obj: *mut conf_object_t,
        processor: *mut attr_value_t,
    ) -> Result<()>;
    /// Called when a fault is added via the external interface
    unsafe fn on_add_fault(&mut self, obj: *mut conf_object_t, fault: i64) -> Result<()>;
}
