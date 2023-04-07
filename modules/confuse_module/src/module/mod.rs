//! The confuse module. The *module* module (ha) is what actually runs inside of SIMICS using the
//! entry point defined in `module::entrypoint::init_simics`
//!
//! This module provides communications with a `Client` as well as a controller and
//! several components that provide control and instrumentation necessary for fuzzing

pub mod component;
pub mod components;
pub mod config;
pub mod controller;
pub mod cpu;
pub mod entrypoint;
pub mod map_type;
pub mod stop_reason;
