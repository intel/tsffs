//! Confuse SIMICS API
//!
//! This crate provides low level bindings to the SIMICS API in Rust using automatically generated
//! bindings with Bindgen

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]

// Bindings
include!(concat!(env!("OUT_DIR"), "/simics_bindings.rs"));

mod attr_value_bindings;

pub use attr_value_bindings::*;

// internal.h exports are non public but we need to use some of them
extern "C" {
    /// Discard recorded future events and forget them
    pub fn CORE_discard_future();
}
