// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Traits for classes

use crate::{ConfClass, ConfObject, Result};

/// A class instance can allocated
pub trait ClassAlloc {
    /// Allocates an instance of the object using mm_zalloc and returns a pointer to the
    /// allocated object, whose first entry is the `ConfObject` instance associated with it
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn alloc<T>(_cls: *mut ConfClass) -> Result<*mut ConfObject> {
        let size = std::mem::size_of::<T>();
        let ptr = crate::simics_alloc!(crate::api::ConfObject, size)?;
        Ok(Into::into(ptr))
    }
}

/// A class can be initialized
pub trait ClassInit {
    /// Perform class specific instantiation, set default values for attributes. May allocate
    /// additional memory if co-allocation is not used. Attribute setters are called *after* this
    /// function returns. The default implementation returns the object without modification.
    /// Returning NULL from this function indicates an error
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn init(instance: *mut ConfObject) -> Result<*mut ConfObject> {
        Ok(instance)
    }
}

/// A class can be finalized
pub trait ClassFinalize {
    /// Object can do final initialization that requires attribute values, but should avoid calling
    /// interface methods on *other* objects. The default implementation of this method does
    /// nothing
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn finalize(_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }
}

/// A class instance can be finalized
pub trait ClassObjectsFinalize {
    /// Called after object is fully constructed and can participate in simulation and reverse
    /// execution. May call interface methods on other objects here as part of initialization.
    /// The default implementation of this method does nothing
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn objects_finalized(_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }
}

/// A class can be deinitialized
pub trait ClassDeinit {
    /// Called first on all objects being deleted, should do the opposite of `init` and
    /// deinitialize any additionally-allocated memory, destroy file descriptors, etc.
    /// The default implementation of this method does nothing
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn deinit(_instance: *mut ConfObject) -> Result<()> {
        Ok(())
    }
}

/// A class instance can be deallocated
pub trait ClassDealloc {
    /// Called after all objects are deinitialized, this should free the allocated object using
    /// mm_free
    ///
    /// # Safety
    ///
    /// This method is unsafe because it may dereference a raw pointer. It is up to the
    /// implementation of this method to ensure that the pointer is valid.
    unsafe fn dealloc(instance: *mut ConfObject) -> Result<()> {
        crate::api::free(instance);
        Ok(())
    }
}

/// A class can be created, which usually entails calling `create_class`
pub trait ClassCreate {
    /// Create a class and register it in SIMICS. This does not instantiate the class by creating
    /// any objects, it only creates the (python) class that is used as a blueprint to instantiate
    /// the class
    fn create() -> Result<*mut ConfClass>;
}

/// Trait for simics module objects to implement to create a threadsafe module implementation
pub trait Class:
    ClassAlloc
    + ClassInit
    + ClassFinalize
    + ClassObjectsFinalize
    + ClassDeinit
    + ClassDealloc
    + ClassCreate
{
}
