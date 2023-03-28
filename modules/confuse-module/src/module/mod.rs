//! The confuse module. This component is what actually runs inside of SIMICS using the entry
//! point defined in `module::entrypoint::init_simics`

pub mod component;
pub mod components;
pub mod config;
mod controller;
pub mod entrypoint;
mod map_type;
mod stop_reason;
