//! Tracer object

use inventory::submit;
use once_cell::sync::Lazy;

use crate::{component_initializer, module::component::ComponentInitializer};

component_initializer!(init);

/// Tracer context

/// Init function for tracer. This registers all the necessary callbacks and sets up the shared
/// memory between the client and tracer
pub fn init() {}
