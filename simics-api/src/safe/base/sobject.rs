use simics_api_sys::{sclass_t, sobject_t};

pub struct SObject {
    sobj: *const sobject_t,
}

impl From<*const sobject_t> for SObject {
    fn from(value: *const sobject_t) -> Self {
        Self { sobj: value }
    }
}

impl From<SObject> for *const sobject_t {
    fn from(value: SObject) -> Self {
        value.sobj
    }
}

pub struct SClass {
    sclass: *mut sclass_t,
}

impl From<*mut sclass_t> for SClass {
    fn from(value: *mut sclass_t) -> Self {
        Self { sclass: value }
    }
}

impl From<SClass> for *mut sclass_t {
    fn from(value: SClass) -> Self {
        value.sclass
    }
}

pub fn sobject_class(sobj: SObject) -> SClass {
    let sobj: *const sobject_t = sobj.into();
    unsafe { *sobj }.isa.into()
}
