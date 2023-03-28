//! A component is a part of the module that implements some discrete functionality. For example,
//! the branch tracer is one component. The error detector is another component, and so forth.
//! This module defines common traits of each component, because we need to be checkpointable which
//! introduces some constraints.

use anyhow::Result;
use once_cell::sync::Lazy;

use super::config::{InitializeConfig, InitializedConfig};

#[macro_export]
/// Submit an init function like:
/// ```text
/// component_initializer!(init);
///
/// pub fn init() {}
/// ```
/// as a component initializer such that the init function will be called when the module is
/// loaded.
macro_rules! component_initializer {
    ($i:expr) => {
        submit! {
            static initializer: Lazy<Box<dyn Fn() + Send + Sync>> = Lazy::new(|| {
                Box::new($i)
            });
            ComponentInitializer(&initializer)
        }
    };
}

pub struct ComponentInitializer(pub &'static Lazy<Box<dyn Fn() + Send + Sync>>);

impl ComponentInitializer {
    pub fn init(&self) {
        self.0();
    }
}

pub trait Component {
    /// Called when a `ClientMessage::Initialize` message is received. A component can use any
    /// necessary info in `initialize_config` to initialize itself and modify the
    /// `initialized_config` as necessary, for example by adding a memory map to share with
    /// the client
    fn on_initialize(
        &mut self,
        initialize_config: InitializeConfig,
        initialized_config: &mut InitializedConfig,
    ) -> Result<()>;
    /// Called prior to running the simulator with a given input. Components do not need to do
    /// anything with this information, but they can. For example, the redqueen component needs
    /// to inspect the input to establish an I2S (Input-To-State) correspondence.
    fn pre_run(&mut self, data: &[u8]) -> Result<()>;
    /// Called when a `ClientMessage::Reset` message is received. The component should do anything
    /// it needs in order to prepare for the next run during this call.
    fn on_reset(&mut self) -> Result<()>;
    /// Called when a `ClientMessage::Stop` message is received. The component should clean itself
    /// up and do any pre-exit work it needs to do.
    fn on_stop(&mut self) -> Result<()>;
}
