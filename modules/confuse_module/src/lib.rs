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
// NOTE: We have to do this a lot, and it sucks to have all these functions be unsafe
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use const_format::concatcp;

pub mod client;
pub mod config;
pub mod faults;
pub mod magic;
pub mod messages;
pub mod module;
mod processor;
pub mod state;
pub mod stops;
pub mod traits;
mod util;

pub use module::ConfuseModuleInterface;

/// The class name used for all operations interfacing with SIMICS
pub const CLASS_NAME: &str = env!("CARGO_PKG_NAME");
/// The name of the temporary socket that bootstraps the connection between the fuzzer and the
/// client
pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
/// The log level to use in the module
pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");
/// Whether to run in "test mode" or not (depending whether this variable is set or not)
pub const TESTMODE_VARNAME: &str = concatcp!(CLASS_NAME, "_TEST_MODE");
