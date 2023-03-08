//! Confuse module interface to simics -- this just defines the boilerplate needed for it to
//! be loaded as a SIMICs module
include!(concat!(env!("OUT_DIR"), "/simics_module_header.rs"));

use env_logger::init as init_logging;
use log::info;

use crate::context::CTX;

#[no_mangle]
pub extern "C" fn init_local() {
    init_logging();
    let mut ctx = CTX.lock().expect("Could not lock context!");
    ctx.init().expect("Could not initialize context");
    info!("Initialized context for {}", CLASS_NAME);
}
