// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::api::{ConfClass, ConfObject};
use crate::error::Result;

/// Trait for simics module objects to implement to create a threadsafe module implementation
pub trait Module {
    /// Allocates an instance of the object using mm_zalloc and returns a pointer to the
    /// allocated object, whose first entry is the `ConfObject` instance associated with it
    fn alloc<T>(_module_class: *mut ConfClass) -> Result<*mut ConfObject> {
        let size = std::mem::size_of::<T>();
        Ok(Into::into(crate::simics_alloc!(crate::ConfClass, size)?))
    }

    /// Perform class specific instantiation, set default values for attributes. May allocate
    /// additional memory if co-allocation is not used. Attribute setters are called *after* this
    /// function returns. The default implementation returns the object without modification.
    /// Returning NULL from this function indicates an error
    fn init(module_instance: *mut ConfObject) -> Result<*mut ConfObject> {
        Ok(module_instance)
    }

    /// Object can do final initialization that requires attribute values, but should avoid calling
    /// interface methods on *other* objects. The default implementation of this method does
    /// nothing
    fn finalize(_module_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }

    /// Called after object is fully constructed and can participate in simulation and reverse
    /// execution. May call interface methods on other objects here as part of initialization.
    /// The default implementation of this method does nothing
    fn objects_finalized(_module_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }

    /// Called first on all objects being deleted, should do the opposite of `init` and
    /// deinitialize any additionally-allocated memory, destroy file descriptors, etc.
    /// The default implementation of this method does nothing
    fn deinit(_module_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }

    /// Called after all objects are deinitialized, this should free the allocated object using
    /// mm_free
    fn dealloc(module_instance: *mut ConfObject) -> Result<()> {
        crate::api::free(module_instance);
        Ok(())
    }
}

pub trait CreateClass {
    /// Create a class and register it in SIMICS. This does not instantiate the class by creating
    /// any objects, it only creates the (python) class that is used as a blueprint to instantiate
    /// the class
    fn create() -> Result<*mut ConfClass>;
}
