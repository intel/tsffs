//! Raw bindings to the SIMICS API

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]
#[cfg(feature = "6.0.163")]
include!("bindings-6.0.163.rs");
#[cfg(feature = "6.0.164")]
include!("bindings-6.0.164.rs");
#[cfg(feature = "6.0.165")]
include!("bindings-6.0.165.rs");
#[cfg(feature = "6.0.166")]
include!("bindings-6.0.166.rs");
#[cfg(feature = "6.0.167")]
include!("bindings-6.0.167.rs");
