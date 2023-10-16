pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
/// SIMICS errors, including internal and APIs used
pub enum Error {
    #[error("AttrValue is {actual:?}, expected {expected:?}")]
    AttrValueType {
        actual: crate::api::base::attr_value::AttrKind,
        expected: crate::api::base::attr_value::AttrKind,
    },
    #[error("Index {index} out of bounds of list length {length}")]
    AttrValueListIndexOutOfBounds { index: u32, length: u32 },
    #[error("Index {index} out of bounds of dictionary size {size}")]
    AttrValueDictIndexOutOfBounds { index: u32, size: u32 },
    #[error("Null data requires zero size")]
    InvalidNullDataSize,

    #[error("Failed to create class {name}: {message}")]
    CreateClass { name: String, message: String },
    #[error("Failed to register {name}: {message}")]
    RegisterInterface { name: String, message: String },
    #[error("Could not find class with name {name}")]
    ClassNotFound { name: String },
    #[error("Could not find object with name {name}")]
    ObjectNotFound { name: String },
    #[error("Could not create object: {message}")]
    CreateObject { message: String },
    #[error("No current checkpoint directory: {message}")]
    CurrentCheckpointDir { message: String },
    #[error("No matching event found")]
    NoEventFound,
    #[error("No method {method} found on interface")]
    NoInterfaceMethod { method: String },
    #[error("{exception:?}: {msg}")]
    /// An internal error that comes from the sys API.
    SimicsException {
        exception: crate::api::base::sim_exception::SimException,
        msg: String,
    },
    #[error("This registration type is not supported for this hap")]
    HapRegistrationType,
    #[error("This deletion type is not supported for this hap")]
    HapDeleteType,
    #[error("Value size {actual} is too large (expected <= {expected})")]
    ValueTooLarge { expected: usize, actual: usize },

    // Transparently wrapped errors from std
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),
    // Anyhow error type to allow wrapping any other errors (e.g. from other crates in the
    // workspace)
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
