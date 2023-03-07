#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// TODO: This module has a *ton* of:
// warning: `extern` block uses type `u128`, which is not FFI-safe
// There's nothing we can do about this, so just...try not to use those functions

include!(concat!(env!("OUT_DIR"), "/simics_bindings.rs"));

pub fn SIM_attr_is_nil(attr: attr_value_t) -> bool {
    attr.private_kind == attr_kind_t_Sim_Val_Nil
}

pub fn SIM_attr_object(attr: attr_value_t) -> *mut conf_object_t {
    unsafe { attr.private_u.object }
}

pub fn SIM_attr_object_or_nil(attr: attr_value_t) -> Option<*const conf_object_t> {
    if SIM_attr_is_nil(attr) {
        None
    } else {
        Some(SIM_attr_object(attr))
    }
}
