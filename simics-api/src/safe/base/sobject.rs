use simics_api_sys::{sclass_t, sobject_t};

pub type SObject = sobject_t;
pub type SClass = sclass_t;

pub struct SObjectOwnedConstPtr {
    sobj: *const SObject,
}

impl From<*const SObject> for SObjectOwnedConstPtr {
    fn from(value: *const SObject) -> Self {
        Self { sobj: value }
    }
}

impl From<SObjectOwnedConstPtr> for *const SObject {
    fn from(value: SObjectOwnedConstPtr) -> Self {
        value.sobj
    }
}

pub struct SClassOwnedMutPtr {
    sclass: *mut SClass,
}

impl From<*mut SClass> for SClassOwnedMutPtr {
    fn from(value: *mut SClass) -> Self {
        Self { sclass: value }
    }
}

impl From<SClassOwnedMutPtr> for *mut SClass {
    fn from(value: SClassOwnedMutPtr) -> Self {
        value.sclass
    }
}

pub fn sobject_class(sobj: SObjectOwnedConstPtr) -> SClassOwnedMutPtr {
    let sobj: *const SObject = sobj.into();
    unsafe { *sobj }.isa.into()
}
