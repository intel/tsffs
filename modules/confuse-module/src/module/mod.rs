//! The confuse module. This component is what actually runs inside of SIMICS using the entry
//! point defined in `module::entrypoint::init_simics`

pub mod component;
pub mod components;
pub mod config;
pub mod controller;
pub mod cpu;
pub mod entrypoint;
pub mod map_type;
pub mod stop_reason;
