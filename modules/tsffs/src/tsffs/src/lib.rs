// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! TFFS Module for SIMICS
//!
//! # Overview
//!
//! This crate provides a client and module loadable by SIMICS to enable fuzzing on the SIMICS
//! platform. The client is intended to be used by the `simics-fuzz` crate, but it can be used
//! manually to enable additional use cases.
//!
//! # Capabilities
//!
//! The Module can:
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
#![deny(clippy::unwrap_used)]

use crate::{
    detector::Detector, driver::Driver, fuzzer::Fuzzer, interface::TsffsInterfaceInternal,
    tracer::Tracer,
};
use getters::Getters;
use simics::{
    api::{set_log_level, Class, ConfObject, LogLevel},
    info, Result,
};
use simics_macro::{class, interface, AsConfObject};

pub mod arch;
pub mod detector;
pub mod driver;
pub mod fuzzer;
pub mod init;
pub mod interface;
pub mod tracer;
pub mod traits;

/// The class name used for all operations interfacing with SIMICS
pub const CLASS_NAME: &str = env!("CARGO_PKG_NAME");

#[class(name = CLASS_NAME)]
#[derive(AsConfObject, Getters, Debug)]
#[getters(mutable)]
#[interface]
pub struct Tsffs {
    driver: Driver<'static>,
    fuzzer: Fuzzer<'static>,
    detector: Detector<'static>,
    tracer: Tracer<'static>,
}

impl Class for Tsffs {
    fn init(instance: *mut ConfObject) -> Result<*mut ConfObject> {
        set_log_level(instance, LogLevel::Trace)?;

        info!(instance, "Initialized instance");

        Ok(Tsffs::new(
            instance,
            Driver::builder().parent(instance.into()).build(),
            Fuzzer::builder().parent(instance.into()).build(),
            Detector::builder().parent(instance.into()).build(),
            Tracer::builder().parent(instance.into()).build(),
        ))
    }
}
