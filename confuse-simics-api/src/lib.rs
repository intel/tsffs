#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// TODO: This module has a *ton* of:
// warning: `extern` block uses type `u128`, which is not FFI-safe
// There's nothing we can do about this, so just...try not to use those functions

include!(concat!(env!("OUT_DIR"), "/simics_bindings.rs"));

// Exported by internal.h
extern "C" {
    pub fn CORE_discard_future();

}

pub fn SIM_attr_is_nil(attr: attr_value_t) -> bool {
    attr.private_kind == attr_kind_t_Sim_Val_Nil
}

pub fn SIM_attr_object(attr: attr_value_t) -> *mut conf_object_t {
    unsafe { attr.private_u.object }
}

pub fn SIM_attr_object_or_nil(attr: attr_value_t) -> Option<*mut conf_object_t> {
    if SIM_attr_is_nil(attr) {
        None
    } else {
        Some(SIM_attr_object(attr))
    }
}

pub fn SIM_make_attr_object(obj: *mut conf_object_t) -> attr_value_t {
    attr_value_t {
        private_kind: if obj.is_null() {
            attr_kind_t_Sim_Val_Nil
        } else {
            attr_kind_t_Sim_Val_Object
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { object: obj },
    }
}

pub unsafe fn SIM_attr_integer(attr: attr_value_t) -> i64 {
    unsafe { attr.private_u.integer }
}

pub fn SIM_attr_list_size(attr: attr_value_t) -> u32 {
    attr.private_size
}

pub unsafe fn SIM_attr_list_item(attr: attr_value_t, index: u32) -> attr_value_t {
    unsafe {
        *attr
            .private_u
            .list
            .offset(index.try_into().expect("Unable to convert index"))
    }
}
