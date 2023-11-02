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
    tracer::Tracer, traits::Component,
};
use getters::Getters;
use simics::{
    api::{break_simulation, AsConfObject, Class, ConfObject, CoreSimulationStoppedHap, HapHandle},
    info, Result,
};
use simics_macro::{class, interface, AsConfObject};
use state::StopReason;

pub mod arch;
pub mod detector;
pub mod driver;
pub mod fuzzer;
pub mod init;
pub mod interface;
pub mod state;
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
    stop_hap_handle: HapHandle,
    stop_reason: Option<StopReason>,
}

impl Class for Tsffs {
    fn init(instance: *mut ConfObject) -> Result<*mut ConfObject> {
        let stop_hap_instance = instance;

        let stop_hap_handle = CoreSimulationStoppedHap::add_callback(
            // NOTE: Core_Simulation_Stopped is called with an object, exception and
            // error string, but the exception is always
            // SimException::SimExc_No_Exception and the error string is always
            // null_mut.
            move |_, _, _| {
                // On stops, call the module's stop callback method, which will in turn call the
                // stop callback methods on each of the module's components. The stop reason will
                // be retrieved from the module, if one is set. It is an error for the module to
                // stop itself without setting a reason
                let tsffs: &'static mut Tsffs = stop_hap_instance.into();

                tsffs
                    .on_simulation_stopped()
                    .expect("Error calling simulation stopped callback");
            },
        )?;

        info!(instance, "Initialized instance");

        Ok(Tsffs::new(
            instance,
            Driver::builder().parent(instance.into()).build(),
            Fuzzer::builder().parent(instance.into()).build(),
            Detector::builder().parent(instance.into()).build(),
            Tracer::builder().parent(instance.into()).build(),
            stop_hap_handle,
            None,
        ))
    }
}

impl Tsffs {
    pub fn on_simulation_stopped(&mut self) -> Result<()> {
        if let Some(reason) = self.stop_reason_mut().take() {
            info!(
                self.as_conf_object_mut(),
                "on_simulation_stopped({reason:?})"
            );

            self.fuzzer.on_simulation_stopped(&reason)?;
            self.detector.on_simulation_stopped(&reason)?;
            self.tracer.on_simulation_stopped(&reason)?;
            self.driver.on_simulation_stopped(&reason)?;
        }

        Ok(())
    }

    pub fn stop_simulation(&mut self, reason: StopReason) -> Result<()> {
        let break_string = reason.to_string();
        *self.stop_reason_mut() = Some(reason);
        break_simulation(break_string)?;

        Ok(())
    }
}
