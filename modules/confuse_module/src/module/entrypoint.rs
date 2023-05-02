//! The main entrypoint of the confuse module when it is loaded in SIMICS. SIMICS first calls
//! `_simics_module_init` which is defined automatically by `generate_signature_header` and
//! included in this file. `_simics_module_init` then calls `init_local`, which is where we
//! perform our own initialization

use super::controller::Controller;
use const_format::concatcp;
use log::info;

pub const CLASS_NAME: &str = "confuse_module";
pub const BOOTSTRAP_SOCKNAME: &str = concatcp!(CLASS_NAME, "_SOCK");
pub const LOGLEVEL_VARNAME: &str = concatcp!(CLASS_NAME, "_LOGLEVEL");

/// `confuse_init_local` is automatically called as the entrypoint of the module when it is loaded by
/// SIMICS. Components register initializers that are called by this function.
#[no_mangle]
pub extern "C" fn confuse_init_local() {
    let mut controller = Controller::get().expect("Could not get controller");
    info!("Initializing controller");
    controller
        .initialize()
        .expect("Could not initialize the controller");
}
