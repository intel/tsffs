// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Error types that can be returned by the Simics crate

use std::path::PathBuf;

/// Result type for fallible functions in the SIMICS API
pub type Result<T> = std::result::Result<T, Error>;

#[allow(unused)]
#[derive(thiserror::Error, Debug)]
/// SIMICS errors, including internal and APIs used
pub enum Error {
    #[error("AttrValue is {actual:?}, expected {expected:?} ({reason})")]
    /// Attribute value type mismatch
    AttrValueType {
        /// The value that could not be converted
        // value: crate::AttrValue,
        /// The actual kind of the Attrvalue
        actual: crate::AttrKind,
        /// The expected kind of the AttrValue
        expected: crate::AttrKind,
        /// The reason the conversion failed
        reason: String,
    },
    #[error("Index {index} out of bounds of list length {length}")]
    /// An attribute value list was indexed out of bounds
    AttrValueListIndexOutOfBounds {
        /// The requested index
        index: usize,
        /// The length of the list
        length: usize,
    },
    #[error("List of length {length} is too large")]
    /// A list was too large, this is rare as the list size limit is extremely large.
    AttrValueListTooLarge {
        /// The actual length of the list
        length: usize,
    },
    #[error("Index {index} out of bounds of dictionary size {size}")]
    /// An attribute value dictionary was indexed out of bounds
    AttrValueDictIndexOutOfBounds {
        /// The requested index
        index: usize,
        /// The size of the dictionary
        size: usize,
    },
    #[error("Dictionary of size {size} is too large")]
    /// An attribute value dictionary was too large. This is rare as teh size limit is extremely
    /// large.
    AttrValueDictTooLarge {
        /// The size of the dictionary
        size: usize,
    },
    #[error("Null data requires zero size")]
    /// Null attribute value data construction attempted without a zero size
    InvalidNullDataSize,
    #[error("Error converting to from {ty} to AttrValue")]
    /// Could not convert a type to an AttrValue
    ToAttrValueConversionError {
        /// The name of the type that could not be converted
        ty: String,
    },
    #[error("Error converting from to {ty}")]
    /// Could not convert from an attribute value to a type
    FromAttrValueConversionError {
        /// The attribute value that could not be converted from
        // value: crate::AttrValue,
        /// The type the value could not be converted into
        ty: String,
    },
    #[error("Error converting to from {ty} to AttrValueType")]
    /// Could not convert to an attribute value type from a type
    ToAttrValueTypeConversionError {
        /// The type that could not be converted to an attribute value type
        ty: String,
    },
    #[error("Error converting to from AttrValueType to {ty} ({reason})")]
    /// Could not convert from an attribute value type to a type
    FromAttrValueTypeConversionError {
        /// The value that could not be converted from an attribute value type
        // value: crate::AttrValueType,
        /// The type that could not be converted from an attribute value type
        ty: String,
        /// The reason the conversion failed
        reason: String,
    },
    #[error("Error converting to from {ty} to AttrValue: {source}")]
    /// Could not convert to an attribute value from a type, because of a nested error
    NestedToAttrValueConversionError {
        /// The type that could not be converted to an attribute value
        ty: String,
        /// The nested error that caused this conversion to fail
        source: Box<Error>,
    },
    #[error("Error converting AttrValue to {ty}: {source}")]
    /// Could not convert from an attribute value to a type, because of a nested error
    NestedFromAttrValueConversionError {
        /// The value that could not be converted
        // value: crate::AttrValue,
        /// The type that the value could not be converted into
        ty: String,
        /// The nested error that caused this conversion to fail
        source: Box<Error>,
    },
    #[error("Error converting to from {ty} to AttrValueType: {source}")]
    /// Could not convert to an attribute value type from a type, because of a nested error
    NestedToAttrValueTypeConversionError {
        /// The type that could not be converted to an attribute value type
        ty: String,
        /// The nested error that caused this conversion to fail
        source: Box<Error>,
    },
    #[error("Error converting to from AttrValueType to {ty}: {source}")]
    /// could not convert from an attribute value type to a type, because of a nested error
    NestedFromAttrValueTypeConversionError {
        /// The type that could not be converted from an attribute value type
        ty: String,
        /// The nested error that caused this conversion to fail
        source: Box<Error>,
    },
    #[error("Key {key} not found")]
    /// A key was not found in an attribute value dictionary
    AttrValueDictMissingKey {
        /// The key that was not found
        key: String,
    },
    #[error("AttrValue list is non-homogeneous")]
    /// An attribute value list was non-homogeneous during an operation that required a
    /// homogeneous list
    NonHomogeneousList,
    #[error("AttrValue dictionary is non-homogeneous")]
    /// An attribute value dictionary was non-homogeneous during an operation that required a
    /// homogeneous dictionary
    NonHomogeneousDict,
    #[error("Could not convert to string")]
    /// Error converting a value to a string
    ToString,
    #[error("File {file} was not found in lookup")]
    /// A file was not found
    FileLookup {
        /// The file that was not found
        file: String,
    },
    #[error("Failed to create class {name}: {message}")]
    /// A class creation operation failed
    CreateClass {
        /// The name of the class to be created
        name: String,
        /// The error message
        message: String,
    },
    #[error("Failed to register {name}: {message}")]
    /// Registration of an interface failed
    RegisterInterface {
        /// The name of the interface that failed to register
        name: String,
        /// The error message
        message: String,
    },
    #[error("Could not find class with name {name}")]
    /// A class could not be found
    ClassNotFound {
        /// The name of the class that could not be found
        name: String,
    },
    #[error("Could not find object with name {name}")]
    /// An object could not be found
    ObjectNotFound {
        /// The name of the object that could not be found
        name: String,
    },
    #[error("Could not create object: {message}")]
    /// Object creation failed
    CreateObject {
        /// The reason object creation failed
        message: String,
    },
    #[error("No current checkpoint directory: {message}")]
    /// A checkpoint directory was missing when it was required
    CurrentCheckpointDir {
        /// The source error message
        message: String,
    },
    #[error("No matching event found")]
    /// An event matching a query was not found
    NoEventFound,
    #[error("No method {method} found on interface")]
    /// An interface did not have a given method
    NoInterfaceMethod {
        /// The name of the missing method
        method: String,
    },
    #[error("{exception:?}: {msg}")]
    /// An internal error that comes from the sys API. These exceptions are wrapped in a message
    /// and reported as Rust errors
    SimicsException {
        /// The inner exception
        exception: crate::SimException,
        /// The string describing the exception
        msg: String,
    },
    #[error("This registration type is not supported for this hap")]
    /// An error attempting to register a hap with an unsupported type
    HapRegistrationType,
    #[error("This deletion type is not supported for this hap")]
    /// An error attempting to delete a hap with an unsupported type
    HapDeleteType,
    #[error("Value size {actual} is too large (expected <= {expected})")]
    /// A value was too large
    ValueTooLarge {
        /// The expected size
        expected: usize,
        /// The actual size
        actual: usize,
    },
    #[error("{path:?} is not a directory")]
    /// A path that should have been a directory was not
    NotADirectory {
        /// The path
        path: PathBuf,
    },
    #[error("Unrecognized extension for library type for file {library_type}")]
    /// An extension of a library file was not recognized
    UnrecognizedLibraryTypeExtension {
        /// The file
        library_type: String,
    },
    #[error("File matching pattern {pattern} not found in directory {directory:?}")]
    /// A file could not be found matching a given pattern
    FileNotFoundInDirectory {
        /// The directory that was searched
        directory: PathBuf,
        /// The pattern that was searched for
        pattern: String,
    },

    // Transparently wrapped errors from std
    #[error(transparent)]
    /// A wrapped std::num::TryFromIntError
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    /// A wrapped std::str::Utf8Error
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    /// A wrapped std::ffi::NulError
    NulError(#[from] std::ffi::NulError),
    #[error(transparent)]
    /// A wrapped std::io::Error
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    /// A wrapped std::path::StripPrefixError
    RegexError(#[from] regex::Error),
    // Anyhow error type to allow wrapping any other errors (e.g. from other crates in the
    // workspace)
    #[error(transparent)]
    /// A wrapped anyhow::Error
    Other(#[from] anyhow::Error),
    #[error(transparent)]
    /// A wrapped std::convert::Infallible
    Infallible(#[from] std::convert::Infallible),
}
