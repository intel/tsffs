use raw_cstr::AsRawCstr;

/// A SIMICS interface containing a number of methods that can be called on an
/// object
pub trait Interface {
    type Interface;
    type Name: AsRawCstr;
    const NAME: Self::Name;
    fn new(interface: *mut Self::Interface) -> Self;
}
