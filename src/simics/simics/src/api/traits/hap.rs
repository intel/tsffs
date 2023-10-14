use raw_cstr::AsRawCstr;

/// A SIMICS Hap and the type of callbacks associated with it
pub trait Hap<C> {
    type Callback;
    type Name: AsRawCstr;
    const NAME: Self::Name;
    const HANDLER: Self::Callback;
}
