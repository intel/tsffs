//! CONFUSE Module for SIMICS
//!
//! # Overview
//!
//! This crate provides a client and module loadable by SIMICS to enable fuzzing on the SIMICS
//! platform. The client is intended to be used by the `confuse-fuzz` crate, but it can be used
//! manually to enable additional use cases.
//!
//! # Capabilities
//!
//! The CONFUSE Module can:
//!
//! - Trace branch hits during an execution of a target on an x86_64 processor. These branches
//!   are traced into shared memory in the format understood by the AFL family of tools.
//! - Catch exception/fault events registered in an initial configuration or dynamically using
//!   a SIMICS Python script
//! - Catch timeout events registered in an initial configuration or dynamically using a SIMICS
//!   Python script
//! - Manage the state of a target under test by taking and restoring a snapshot of its state for
//!   deterministic snapshot fuzzing
#![deny(clippy::all)]

use const_format::concatcp;

pub mod client;
pub mod config;
pub mod faults;
mod interface;
pub mod magic;
pub mod maps;
pub mod messages;
mod module;
pub mod state;
pub mod stops;
mod traits;
mod util;

pub const CLASS_NAME: &str = env!("CARGO_PKG_NAME");
pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");
