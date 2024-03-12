// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Traits for interfaces

use crate::{get_interface, register_interface, ConfClass, ConfObject, Result};
use raw_cstr::AsRawCstr;

/// A SIMICS interface containing a number of methods that can be called on an
/// object
pub trait Interface {
    /// The inner interface type, which is a struct of nullable extern "C" function pointers
    /// and must be default constructable as all NULL pointers (i.e. None values)
    type InternalInterface: Default;
    /// The type of the name of the interface, must be convertible to raw C string to pass to
    /// the simulator
    type Name: AsRawCstr;

    /// The name of the interface
    const NAME: Self::Name;

    /// Create a new instance of this interface
    fn new(obj: *mut ConfObject, interface: *mut Self::InternalInterface) -> Self;

    /// Register this interface for a type
    fn register(cls: *mut ConfClass) -> Result<()>
    where
        Self: Sized,
    {
        register_interface::<Self>(cls)?;
        Ok(())
    }

    /// Get this interface for an object that implements it
    fn get(obj: *mut ConfObject) -> Result<Self>
    where
        Self: Sized,
    {
        get_interface::<Self>(obj)
    }
}

/// An object which has a SIMICS interface I
pub trait HasInterface<I>
where
    I: Interface,
{
}
