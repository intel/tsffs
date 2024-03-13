// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Derive/attribute macros for simics-api

#![deny(clippy::unwrap_used)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use attr_value::{
    from_attr_value_dict_impl, from_attr_value_list_impl, into_attr_value_dict_impl,
    into_attr_value_list_impl,
};
use class::{class_derive_impl, class_impl};
use conf_object::{as_conf_object_impl, from_conf_object_impl};
use exception::simics_exception_impl;
use init::simics_init_impl;
use interface::interface_impl;
use proc_macro::TokenStream;

mod attr_value;
mod class;
mod conf_object;
mod exception;
mod init;
mod interface;

#[allow(non_snake_case)]
#[proc_macro_derive(IntoAttrValueList, attributes(attr_value))]
/// Derive Macro for implementing conversion into an `AttrValue` list, where each struct
/// field's value is an entry in the heterogeneous list.
///
/// # Arguments
///
/// At the field level (i.e. on each field of a struct deriving this attribute), the
/// following attributes are supported:
///
/// * `#[attr_value(skip)]` - Do not include this field in the conversion.
/// * `#[attr_value(fallible)]` - If the field type does not implement `Into<AttrValue>`,
///   use its implementation of `TryInto<AttrValue>` instead. Whether this flag is necessary
///   cannot be automatically determined by this macro, so it must be specified manually.
pub fn IntoAttrValueList(input: TokenStream) -> TokenStream {
    into_attr_value_list_impl(input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(IntoAttrValueDict, attributes(attr_value))]
/// Derive Macro for implementing conversion into an `AttrValue` dictionary, where each
/// struct field's key, value pair is an entry in the dictionary.
///
/// # Arguments
///
/// At the field level (i.e. on each field of a struct deriving this attribute), the
/// following attributes are supported:
///
/// * `#[attr_value(skip)]` - Do not include this field in the conversion.
/// * `#[attr_value(fallible)]` - If the field type does not implement `Into<AttrValue>`,
///   use its implementation of `TryInto<AttrValue>` instead. Whether this flag is necessary
///   cannot be automatically determined by this macro, so it must be specified manually.
pub fn IntoAttrValueDict(input: TokenStream) -> TokenStream {
    into_attr_value_dict_impl(input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(FromAttrValueList, attributes(attr_value))]
/// Derive Macro for implementing conversion from an `AttrValue` list into a struct, where
/// each entry in the list is a struct field's value.
///
/// # Arguments
///
/// At the field level (i.e. on each field of a struct deriving this attribute), the
/// following attributes are supported:
///
/// * `#[attr_value(fallible)]` - If the field type does not implement `From<AttrValue>`,
///   use its implementation of `TryFrom<AttrValue>` instead. Whether this flag is necessary
///   cannot be automatically determined by this macro, so it must be specified manually.
pub fn FromAttrValueList(input: TokenStream) -> TokenStream {
    from_attr_value_list_impl(input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(FromAttrValueDict, attributes(attr_value))]
/// Derive Macro for implementing conversion from an `AttrValue` dict into a struct, where
/// each key, value pair in the dict is a struct field's name, value pair.
///
/// # Arguments
///
/// At the field level (i.e. on each field of a struct deriving this attribute), the
/// following attributes are supported:
///
/// * `#[attr_value(fallible)]` - If the field type does not implement `From<AttrValue>`,
///   use its implementation of `TryFrom<AttrValue>` instead. Whether this flag is necessary
///   cannot be automatically determined by this macro, so it must be specified manually.
pub fn FromAttrValueDict(input: TokenStream) -> TokenStream {
    from_attr_value_dict_impl(input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(Class, attributes(class))]
/// Derive macro for implementing the `Class` trait for the annotated type
pub fn Class(input: TokenStream) -> TokenStream {
    class_derive_impl(input)
}

#[proc_macro_attribute]
/// Attribute macro for declaring a Simics class for a Rust struct type
pub fn class(args: TokenStream, input: TokenStream) -> TokenStream {
    class_impl(args, input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(AsConfObject, attributes(conf_object))]
/// Derive macro for implementing conversion to raw `ConfObject` pointers.
/// This macro implements the `AsConfObject` trait for the annotated type.
pub fn AsConfObject(input: TokenStream) -> TokenStream {
    as_conf_object_impl(input)
}

#[allow(non_snake_case)]
#[proc_macro_derive(FromConfObject, attributes(conf_object))]
/// Derive macro for implementing conversion from raw `ConfObject` pointers.
/// This macro implements the `FromConfObject` trait for the annotated type.
///
/// # Arguments
///
/// At the item level (i.e. on the struct deriving this attribute), the following
/// attributes are supported:
///
/// * `#[conf_object(skip_from)]` - Skip also implementing `From<*_ ConfObject>` which is the
///   default behavior.
pub fn FromConfObject(input: TokenStream) -> TokenStream {
    from_conf_object_impl(input)
}

#[proc_macro_attribute]
/// Marks a function as being a SIMICS API that can throw exceptions in called FFI APIs.
/// A SIMICS exception can be generated by most APIs. This macro makes the function
/// private, wraps it, and adds the requisite code to check for and report exceptions.
/// `clear_exception` should *not* be called inside the wrapped function. `last_error`
/// may be called, however, as any exceptions will be cleared after the wrapped function
/// returns.
///
/// # Examples
///
/// Add the `#[simics_exception]` attribute to a function which calls a SIMICS API that can throw
/// exceptions. The function will be wrapped and the requisite code to check for and report
/// exceptions will be added.
///
/// ```rust,ignore
/// #[simics_exception]
/// pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
///     unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
/// }
/// ```
///
/// This expands to:
///
/// ```rust,ignore
/// fn _write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
///     unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
/// }
///
/// pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) -> Result<()> {
///     let res = _write_byte(physical_memory, physical_addr, byte);
///
///     match simics::get_pending_exception() {
///         SimException::SimExc_No_Exception => Ok(()),
///         exception => {
///             clear_exception();
///             Err(Error::from(exception))
///         }
///     }
/// }
/// ```
pub fn simics_exception(args: TokenStream, input: TokenStream) -> TokenStream {
    simics_exception_impl(args, input)
}

#[proc_macro_attribute]
/// Mark a function as being the initializer of a Simics module. This function will be called on
/// module load and should be used to initialize the module. This macro will add the requisite
/// code to call the function on module load.
pub fn simics_init(args: TokenStream, input: TokenStream) -> TokenStream {
    simics_init_impl(args, input)
}

#[proc_macro_attribute]
/// Declare a struct implementation as a SIMICS API interface. This macro will add the
/// requisite code to implement the interface which allows methods in the impl to be called
/// from Simics scripts in Python or the Simics language.
pub fn interface(args: TokenStream, input: TokenStream) -> TokenStream {
    interface_impl(args, input)
}
