//! Raw bindings to the SIMICS API

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]

#[cfg(feature = "auto")]
include!(concat!(env!("OUT_DIR"), "/bindings-auto.rs"));

#[cfg(feature = "auto")]
pub const SIMICS_API_BINDINGS: &str = include_str!(concat!(env!("OUT_DIR"), "/bindings-auto.rs"));

#[cfg(feature = "6.0.163")]
include!("bindings-6.0.163.rs");
#[cfg(feature = "6.0.163")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.163.rs");

#[cfg(feature = "6.0.164")]
include!("bindings-6.0.164.rs");
#[cfg(feature = "6.0.164")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.164.rs");

#[cfg(feature = "6.0.165")]
include!("bindings-6.0.165.rs");
#[cfg(feature = "6.0.165")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.165.rs");

#[cfg(feature = "6.0.166")]
include!("bindings-6.0.166.rs");
#[cfg(feature = "6.0.166")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.166.rs");

#[cfg(feature = "6.0.167")]
include!("bindings-6.0.167.rs");
#[cfg(feature = "6.0.167")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.167.rs");

#[cfg(feature = "6.0.168")]
include!("bindings-6.0.168.rs");
#[cfg(feature = "6.0.168")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.168.rs");

#[cfg(feature = "6.0.169")]
include!("bindings-6.0.169.rs");
#[cfg(feature = "6.0.169")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.169.rs");

#[cfg(feature = "6.0.170")]
include!("bindings-6.0.170.rs");
#[cfg(feature = "6.0.170")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.170.rs");

#[cfg(feature = "6.0.171")]
include!("bindings-6.0.171.rs");
#[cfg(feature = "6.0.171")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.171.rs");

#[cfg(feature = "6.0.172")]
include!("bindings-6.0.172.rs");
#[cfg(feature = "6.0.172")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.172.rs");

#[cfg(feature = "6.0.173")]
include!("bindings-6.0.173.rs");
#[cfg(feature = "6.0.173")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.173.rs");

#[cfg(feature = "6.0.174")]
include!("bindings-6.0.174.rs");
#[cfg(feature = "6.0.173")]
pub const SIMICS_API_BINDINGS: &str = include_str!("bindings-6.0.174.rs");
