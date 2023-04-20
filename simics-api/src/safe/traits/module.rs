use crate::{ConfObject, OwnedMutConfClassPtr, OwnedMutConfObjectPtr};
use anyhow::Result;
use std::ffi::c_void;

/// Trait for simics module objects to implement to create a threadsafe module implementation
pub trait Module {
    /// Allocates an instance of the object using mm_zalloc and returns a pointer to the
    /// allocated object, whose first entry is the `ConfObject` instance associated with it
    fn alloc<T>(_cls: OwnedMutConfClassPtr) -> Result<OwnedMutConfObjectPtr> {
        let size = std::mem::size_of::<T>();
        Ok(Into::into(crate::simics_alloc!(crate::ConfClass, size)?))
    }

    /// Perform class specific instantiation, set default values for attributes. May allocate
    /// additional memory if co-allocation is not used. Attribute setters are called *after* this
    /// function returns. The default implementation returns the object without modification.
    /// Returning NULL from this function indicates an error
    fn init(obj: OwnedMutConfObjectPtr) -> OwnedMutConfObjectPtr {
        obj
    }

    /// Object can do final initialization that requires attribute values, but should avoid calling
    /// interface methods on *other* objects. The default implementation of this method does
    /// nothing
    fn finalize(_obj: OwnedMutConfObjectPtr) {}

    /// Called after object is fully constructed and can participate in simulation and reverse
    /// execution. May call interface methods on other objects here as part of initialization.
    /// The default implementation of this method does nothing
    fn objects_finalized(_obj: OwnedMutConfObjectPtr) {}

    /// Called first on all objects being deleted, should do the opposite of `init` and
    /// deinitialize any additionally-allocated memory, destroy file descriptors, etc.
    /// The default implementation of this method does nothing
    fn deinit(_obj: OwnedMutConfObjectPtr) {}

    /// Called after all objects are deinitialized, this should free the allocated object using
    /// mm_free
    fn dealloc(obj: OwnedMutConfObjectPtr) {
        crate::free(obj.into())
    }
}
