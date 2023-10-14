// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Safe wrappers for attr_value_t operations
//!
//! `attr_value_t` instances are basically Python objects as tagged unions (like an `enum`), these
//! functions convert the objects back and forth between anonymous `attr_value_t` and actual data
//! types like `bool`, `String`, etc.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    api::{
        sys::{
            attr_kind_t, attr_value__bindgen_ty_1, attr_value_t, SIM_alloc_attr_dict,
            SIM_alloc_attr_list, SIM_attr_dict_resize, SIM_attr_dict_set_item, SIM_attr_free,
            SIM_attr_list_resize, SIM_attr_list_set_item, SIM_free_attribute,
        },
        ConfObject,
    },
    Error, Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::{
    ffi::CStr,
    mem::size_of,
    ptr::{addr_of_mut, null_mut},
};

macro_rules! ok_if_kind {
    ($a:ident, $k:expr, $e:expr) => {
        if $a.private_kind == $k {
            Ok($e)
        } else {
            Err(Error::AttrValueType {
                actual: $a.private_kind,
                expected: $k,
            })
        }
    };
}

pub type AttrValue = attr_value_t;
pub type AttrKind = attr_kind_t;

/// Create a new invalid [`AttrValue`]
pub fn make_attr_invalid() -> AttrValue {
    AttrValue {
        private_kind: AttrKind::Sim_Val_Invalid,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    }
}

/// Create a new nil [`AttrValue`]
pub fn make_attr_nil() -> AttrValue {
    AttrValue {
        private_kind: AttrKind::Sim_Val_Nil,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    }
}

/// Create a new uint64 [`AttrValue`] with a value of `i`
pub fn make_attr_uint64(i: u64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Integer,
        private_size: 0, /* unsigned */
        private_u: attr_value__bindgen_ty_1 {
            integer: i64::try_from(i)?,
        },
    })
}

/// Create a new int64 [`AttrValue`] with a value of `i`
pub fn make_attr_int64(i: i64) -> AttrValue {
    AttrValue {
        private_kind: AttrKind::Sim_Val_Integer,
        private_size: 1, /* signed */
        private_u: attr_value__bindgen_ty_1 { integer: i },
    }
}

/// Create a new boolean [`AttrValue`] with a value of `b`
pub fn make_attr_boolean(b: bool) -> AttrValue {
    AttrValue {
        private_kind: AttrKind::Sim_Val_Boolean,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { boolean: b },
    }
}

/// Create a newly allocated string [`AttrValue`] with a value of `s`
pub fn make_attr_string_adopt<S>(s: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    let string = raw_cstr(s)?;

    Ok(AttrValue {
        private_kind: if string.is_null() {
            AttrKind::Sim_Val_Nil
        } else {
            AttrKind::Sim_Val_String
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { string },
    })
}

/// Create a new floating point [`AttrValue`] with a value of `d`
pub fn make_attr_floating(d: f64) -> AttrValue {
    AttrValue {
        private_kind: AttrKind::Sim_Val_Floating,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { floating: d },
    }
}

/// Create a new object [`AttrValue`] with a value of `obj`
pub fn make_attr_object(obj: *mut ConfObject) -> AttrValue {
    AttrValue {
        private_kind: if (obj as *const ConfObject).is_null() {
            AttrKind::Sim_Val_Nil
        } else {
            AttrKind::Sim_Val_Object
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { object: obj },
    }
}

/// Create a new data [`AttrValue`], which is effectively a fat pointer to the data, with a given
/// size. The data will be moved into a [`Box`], which will be converted to a raw pointer.
pub fn make_attr_data_adopt<T>(data: T) -> Result<AttrValue> {
    let data = Box::new(data);
    let data_raw = Box::into_raw(data);

    debug_assert!(
        std::mem::size_of_val(&data_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    let data_size = u32::try_from(size_of::<*mut T>())?;

    if !data_raw.is_null() || data_size == 0 {
        Err(Error::InvalidNullDataSize)
    } else {
        Ok(attr_value_t {
            private_kind: AttrKind::Sim_Val_Data,
            private_size: data_size,
            private_u: attr_value__bindgen_ty_1 {
                data: data_raw as *mut u8,
            },
        })
    }
}

/// NOTE: We do not implement a vararg version, this is rust.

/// Create a new attribute list
pub fn make_attr_list(length: u32, attrs: Vec<AttrValue>) -> Result<AttrValue> {
    let mut list = alloc_attr_list(length);
    attrs
        .into_iter()
        .enumerate()
        .try_for_each(|(i, a)| attr_list_set_item(addr_of_mut!(list), i as u32, a))?;
    Ok(list)
}

/// Allocate a new attribute list of a given length
pub fn alloc_attr_list(length: u32) -> AttrValue {
    unsafe { SIM_alloc_attr_list(length) }
}

pub fn alloc_attr_dict(length: u32) -> AttrValue {
    unsafe { SIM_alloc_attr_dict(length) }
}

#[simics_exception]
pub fn attr_list_set_item(attr: *mut AttrValue, index: u32, elem: AttrValue) {
    unsafe { SIM_attr_list_set_item(attr, index, elem) }
}

pub fn attr_list_resize(attr: *mut AttrValue, newsize: u32) {
    unsafe { SIM_attr_list_resize(attr, newsize) };
}

#[simics_exception]
pub fn attr_dict_set_item(attr: *mut AttrValue, index: u32, key: AttrValue, value: AttrValue) {
    unsafe { SIM_attr_dict_set_item(attr, index, key, value) };
}

pub fn attr_dict_resize(attr: *mut AttrValue, newsize: u32) {
    unsafe { SIM_attr_dict_resize(attr, newsize) };
}

/// Check whether an [`AttrValue`] is nil
pub fn attr_is_nil(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Nil
}

/// Check whether an [`AttrValue`] is int64
pub fn attr_is_int64(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Integer
        && (attr.private_size == 0 || unsafe { attr.private_u.integer } >= 0)
}

/// Check whether an [`AttrValue`] is uint64
pub fn attr_is_uint64(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Integer
        && (attr.private_size != 0 || unsafe { attr.private_u.integer } >= 0)
}

/// Check whether an [`AttrValue`] is an integer
pub fn attr_is_integer(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Integer
}

/// Get an [`AttrValue`] as an integer
pub fn attr_integer(attr: AttrValue) -> Result<i64> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Integer, unsafe {
        attr.private_u.integer
    })
}

/// Check whether an [`AttrValue`] is a boolean
pub fn attr_is_boolean(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Boolean
}

/// Get an [`AttrValue`] as a boolean
pub fn attr_boolean(attr: AttrValue) -> Result<bool> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Boolean, unsafe {
        attr.private_u.boolean
    })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_string(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_String
}

/// Get an [`AttrValue`] as a String
pub fn attr_string(attr: AttrValue) -> Result<String> {
    ok_if_kind!(
        attr,
        AttrKind::Sim_Val_String,
        unsafe { CStr::from_ptr(attr.private_u.string) }
            .to_str()?
            .to_string()
    )
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_floating(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Floating
}

/// Get an [`AttrValue`] as a f64
pub fn attr_floating(attr: AttrValue) -> Result<f64> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Floating, unsafe {
        attr.private_u.floating
    })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_object(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Object
}

/// Get an [`AttrValue`] as an object
pub fn attr_object(attr: AttrValue) -> Result<*mut ConfObject> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Object, unsafe {
        attr.private_u.object
    })
}

/// Obtain a [`ConfObject`] pointer from an [`AttrValue`] pointer
pub fn attr_object_from_ptr(attr: *mut AttrValue) -> Result<*mut ConfObject> {
    let ptr: *mut AttrValue = attr;
    attr_object(unsafe { *ptr })
}

/// Get an [`AttrValue`] as an object or nil if the object is a null pointer
pub fn attr_object_or_nil(attr: AttrValue) -> Result<*mut ConfObject> {
    if attr_is_nil(attr) {
        Ok(null_mut())
    } else {
        attr_object(attr)
    }
}

/// Get an [`AttrValue`] as an object or nil if the object is a null pointer
pub fn attr_object_or_nil_from_ptr(attr: *mut AttrValue) -> Result<*mut ConfObject> {
    let ptr: *mut AttrValue = attr;
    attr_object_or_nil(unsafe { *ptr })
}

/// Check whether an [`AttrValue`] is invalid
pub fn attr_is_invalid(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Invalid
}

/// Check whether an [`AttrValue`] is data
pub fn attr_is_data(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Data
}

/// Get the size of an [`AttrValue`]'s data
pub fn attr_data_size(attr: AttrValue) -> Result<u32> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Data, attr.private_size)
}

pub fn attr_data<T>(attr: AttrValue) -> Result<T> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Data, *unsafe {
        Box::from_raw(attr.private_u.data as *mut T)
    })
}

/// Check whether an [`AttrValue`] is a list
pub fn attr_is_list(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_List
}

/// Get the size of an [`AttrValue`]'s list
pub fn attr_list_size(attr: AttrValue) -> Result<u32> {
    ok_if_kind!(attr, AttrKind::Sim_Val_List, attr.private_size)
}

/// Retrieve a list item from an attr
///
/// # Safety
///
/// The bounds of the list are checked before obtaining an offset, so this function will never
/// crash unless the list size is incorrectly set by SIMICS
pub unsafe fn attr_list_item(attr: AttrValue, index: u32) -> Result<AttrValue> {
    let length = attr_list_size(attr)?;

    if index < length {
        ok_if_kind!(attr, AttrKind::Sim_Val_List, unsafe {
            *attr.private_u.list.offset(index.try_into()?)
        })
    } else {
        Err(Error::AttrValueListIndexOutOfBounds { index, length })
    }
}

/// Get the an [`AttrValue`] as a list
pub fn attr_list(attr: AttrValue) -> Result<*mut AttrValue> {
    ok_if_kind!(attr, AttrKind::Sim_Val_List, unsafe { attr.private_u.list })
}

/// Check whether an [`AttrValue`] is a dict
pub fn attr_is_dict(attr: AttrValue) -> bool {
    attr.private_kind == AttrKind::Sim_Val_Dict
}

/// Get the size of an an [`AttrValue`]'s dict
pub fn attr_dict_size(attr: AttrValue) -> Result<u32> {
    ok_if_kind!(attr, AttrKind::Sim_Val_Dict, attr.private_size)
}

/// Get a key for an [`AttrValue`]'s dict
pub fn attr_dict_key(attr: AttrValue, index: u32) -> Result<AttrValue> {
    let size = attr_dict_size(attr)?;

    if index < size {
        ok_if_kind!(attr, AttrKind::Sim_Val_Dict, unsafe {
            (*attr.private_u.dict.offset(index.try_into()?)).key
        })
    } else {
        Err(Error::AttrValueDictIndexOutOfBounds { index, size })
    }
}

/// Get a value for an [`AttrValue`]'s dict
pub fn attr_dict_value(attr: AttrValue, index: u32) -> Result<AttrValue> {
    let size = attr_dict_size(attr)?;

    if index < size {
        ok_if_kind!(attr, AttrKind::Sim_Val_Dict, unsafe {
            (*attr.private_u.dict.offset(index.try_into()?)).value
        })
    } else {
        Err(Error::AttrValueDictIndexOutOfBounds { index, size })
    }
}

#[simics_exception]
/// Free an attr value. [`attr_free`] should be used instead where possible.
pub fn free_attribute(attr: AttrValue) {
    unsafe { SIM_free_attribute(attr) }
}

#[simics_exception]
/// Free an attr value. This function is preferred over [`free_attribute`] because
/// it changes the argument type to invalid.
pub fn attr_free(attr: *mut AttrValue) {
    unsafe { SIM_attr_free(attr) }
}
