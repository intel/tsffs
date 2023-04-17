use crate::{conf_class_t, conf_object_t, sobject_class};

#[no_mangle]
pub unsafe extern "C" fn SIM_object_class(obj: *const conf_object_t) -> *mut conf_class_t {
    sobject_class(&unsafe { *obj }.sobj) as *mut conf_class_t
}
