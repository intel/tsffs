use crate::{
    api::{ConfClass, ConfObject},
    Result,
};
use raw_cstr::AsRawCstr;

/// A SIMICS interface containing a number of methods that can be called on an
/// object
pub trait Interface {
    type InternalInterface: Default;
    type Name: AsRawCstr;
    const NAME: Self::Name;
    /// Create a new instance of this interface
    fn new(obj: *mut ConfObject, interface: *mut Self::InternalInterface) -> Self;
    /// Register this interface for a type
    fn register(cls: *mut ConfClass) -> Result<()>;
    /// Get this interface for an object that implements it
    fn get(obj: *mut ConfObject) -> Result<Self>
    where
        Self: Sized;
}

/// An object which has a SIMICS interface
pub trait HasInterface {
    type Interface: Interface;
}
