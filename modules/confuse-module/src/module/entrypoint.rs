//! The main entrypoint of the confuse module when it is loaded in SIMICS. SIMICS first calls
//! `_simics_module_init` which is defined automatically by `generate_signature_header` and
//! included in this file. `_simics_module_init` then calls `init_local`, which is where we
//! perform our own initialization
include!(concat!(env!("OUT_DIR"), "/simics_module_header.rs"));

use super::{component::ComponentInitializer, controller::Controller};
use anyhow::Result;
use const_format::concatcp;
use inventory::{collect, iter};
use std::{env::var, str::FromStr};

pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");

/// `init_local` is automatically called as the entrypoint of the module when it is loaded by
/// SIMICS. Components register initializers that are called by this function.
#[no_mangle]
pub extern "C" fn init_local() {
    let mut controller = Controller::get().expect("Could not get controller");
}
