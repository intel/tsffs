//! # SIMICS API SYS
//!
//! Low level bindings to the SIMICS API
//!
//! This crate provides raw bindings to the SIMICS api built directly from the header files of the
//! SIMICS base package using `bindgen`. In general, you should prefer to use the `simics-api`
//! crate over this one, as it provides higher level safe bindings to the SIMICS API.

mod bindings;

pub use bindings::*;
