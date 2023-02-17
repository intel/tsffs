#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// TODO: This module has a *ton* of:
// warning: `extern` block uses type `u128`, which is not FFI-safe
// There's nothing we can do about this, so just...try not to use those functions

include!(concat!(env!("OUT_DIR"), "/simics_bindings.rs"));
