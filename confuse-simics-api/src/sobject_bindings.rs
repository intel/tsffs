use crate::{sclass_t, sobject_t};

#[no_mangle]
pub unsafe extern "C" fn sobject_class(sobj: *const sobject_t) -> *mut sclass_t {
    unsafe { *sobj }.isa
}
