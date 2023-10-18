use crate::{Tsffs, CLASS_NAME};
use simics::{
    api::{CreateClass, HasInterface, Interface},
    warn,
};

#[no_mangle]
/// Called by SIMICS when the module is loaded via `load-module tsffs` or
/// `SIM_load_module("tsffs")`
pub extern "C" fn init_local() {
    let cls =
        Tsffs::create().unwrap_or_else(|e| panic!("Failed to create class {}: {}", CLASS_NAME, e));

    warn!("Created class {}", CLASS_NAME);

    <Tsffs as HasInterface>::Interface::register(cls).unwrap_or_else(|e| {
        panic!(
            "Failed to register interface for class {}: {}",
            CLASS_NAME, e
        )
    });

    warn!("Registered interface for class {}", CLASS_NAME);
}
