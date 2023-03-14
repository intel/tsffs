//! Confuse SIMICS API
//! 
//! This crate provides low level bindings to the SIMICS API in Rust using automatically generated
//! bindings with Bindgen

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Bindings
include!(concat!(env!("OUT_DIR"), "/simics_bindings.rs"));

mod attr_value;

pub use attr_value::*;

// internal.h exports are non public but we need to use some of them
extern "C" {
    /// Discard recorded future events and forget them
    pub fn CORE_discard_future();
}