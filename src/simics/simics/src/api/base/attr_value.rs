// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Safe wrappers for attr_value_t operations
//!
//! `attr_value_t` instances are basically Python objects as tagged unions (like an `enum`), these
//! functions convert the objects back and forth between anonymous `attr_value_t` and actual data
//! types like `bool`, `String`, etc.

use crate::api::sys::{
    attr_value__bindgen_ty_1, attr_value_t, SIM_alloc_attr_list, SIM_attr_free, SIM_free_attribute,
    SIM_get_attribute,
};
use crate::api::ConfObject;
use anyhow::{ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::attr_kind_t;
use std::{ffi::CStr, mem::size_of, ptr::null_mut};

pub type AttrValue = attr_value_t;

pub type AttrKind = attr_kind_t;

/// Create a new invalid [`AttrValue`]
pub fn make_attr_invalid() -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Invalid.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    })
}

/// Create a new nil [`AttrValue`]
pub fn make_attr_nil() -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Nil.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    })
}

/// Create a new uint64 [`AttrValue`] with a value of `i`
pub fn make_attr_uint64(i: u64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Integer.try_into()?,
        private_size: 0, /* unsigned */
        private_u: attr_value__bindgen_ty_1 {
            integer: i64::try_from(i)?,
        },
    })
}

/// Create a new int64 [`AttrValue`] with a value of `i`
pub fn make_attr_int64(i: i64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Integer.try_into()?,
        private_size: 1, /* signed */
        private_u: attr_value__bindgen_ty_1 { integer: i },
    })
}

/// Create a new boolean [`AttrValue`] with a value of `b`
pub fn make_attr_boolean(b: bool) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Boolean.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { boolean: b },
    })
}

/// Create a newly allocated string [`AttrValue`] with a value of `s`
pub fn make_attr_string_adopt<S>(s: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    let string = raw_cstr(s)?;

    Ok(AttrValue {
        private_kind: if string.is_null() {
            AttrKind::Sim_Val_Nil.try_into()?
        } else {
            AttrKind::Sim_Val_String.try_into()?
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { string },
    })
}

/// Create a new floating point [`AttrValue`] with a value of `d`
pub fn make_attr_floating(d: f64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Sim_Val_Floating.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { floating: d },
    })
}

/// Create a new object [`AttrValue`] with a value of `obj`
pub fn make_attr_object(obj: *mut ConfObject) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: if (obj as *const ConfObject).is_null() {
            AttrKind::Sim_Val_Nil.try_into()?
        } else {
            AttrKind::Sim_Val_Object.try_into()?
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { object: obj.into() },
    })
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

    ensure!(
        !(data_raw.is_null() && data_size == 0),
        "NULL data requires zero size"
    );

    Ok(attr_value_t {
        private_kind: AttrKind::Sim_Val_Data.try_into()?,
        private_size: u32::try_from(data_size)?,
        private_u: attr_value__bindgen_ty_1 {
            data: data_raw as *mut u8,
        },
    })
}

/// Check whether an [`AttrValue`] is nil
pub fn attr_is_nil(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Nil.try_into()?)
}

/// Check whether an [`AttrValue`] is int64
pub fn attr_is_int64(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Integer.try_into()?
        && (attr.private_size == 0 || unsafe { attr.private_u.integer } >= 0))
}

/// Check whether an [`AttrValue`] is uint64
pub fn attr_is_uint64(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Integer.try_into()?
        && (attr.private_size != 0 || unsafe { attr.private_u.integer } >= 0))
}

/// Check whether an [`AttrValue`] is an integer
pub fn attr_is_integer(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Integer.try_into()?)
}

/// Get an [`AttrValue`] as an integer
pub fn attr_integer(attr: AttrValue) -> Result<i64> {
    ensure!(attr_is_integer(attr)?, "Attribute must be integer!");
    Ok(unsafe { attr.private_u.integer })
}

/// Check whether an [`AttrValue`] is a boolean
pub fn attr_is_boolean(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Boolean.try_into()?)
}

/// Get an [`AttrValue`] as a boolean
pub fn attr_boolean(attr: AttrValue) -> Result<bool> {
    ensure!(attr_is_boolean(attr)?, "Attribute must be bool!");
    Ok(unsafe { attr.private_u.boolean })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_string(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_String.try_into()?)
}

/// Get an [`AttrValue`] as a String
pub fn attr_string(attr: AttrValue) -> Result<String> {
    ensure!(attr_is_string(attr)?, "Attribute must be string!");
    Ok(unsafe { CStr::from_ptr(attr.private_u.string) }
        .to_string_lossy()
        .to_string())
}

/* <append-fun id="SIM_attr_integer"/> */
// TODO: Impl
// pub fn attr_string_detach(attr: *mut attr_value_t) -> char * {
//
//         char *ret;
//         VALIDATE_ATTR_KIND(SIM_attr_string_detach, *attr, String,
//                            (SIM_attr_free(attr),
//                             *attr = SIM_make_attr_nil(),
//                             MM_STRDUP("")));
//         ret = (char *)attr-.private_u.string;
//         *attr = SIM_make_attr_nil();
//         return ret;
// }

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_floating(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Floating.try_into()?)
}

/// Get an [`AttrValue`] as a f64
pub fn attr_floating(attr: AttrValue) -> Result<f64> {
    ensure!(attr_is_floating(attr)?, "Attribute must be floating point!");
    Ok(unsafe { attr.private_u.floating })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_object(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Object.try_into()?)
}

/// Get an [`AttrValue`] as an object
pub fn attr_object(attr: AttrValue) -> Result<*mut ConfObject> {
    ensure!(attr_is_object(attr)?, "Attribute must be object!");
    Ok(unsafe { attr.private_u.object })
}

/// Obtain a [`ConfObject`] pointer from an [`AttrValue`] pointer
pub fn attr_object_from_ptr(attr: *mut AttrValue) -> Result<*mut ConfObject> {
    let ptr: *mut AttrValue = attr.into();
    attr_object(unsafe { *ptr })
}

/// Get an [`AttrValue`] as an object or nil if the object is a null pointer
pub fn attr_object_or_nil(attr: AttrValue) -> Result<*mut ConfObject> {
    if attr_is_nil(attr)? {
        Ok(null_mut())
    } else {
        attr_object(attr)
    }
}

/// Get an [`AttrValue`] as an object or nil if the object is a null pointer
pub fn attr_object_or_nil_from_ptr(attr: *mut AttrValue) -> Result<*mut ConfObject> {
    let ptr: *mut AttrValue = attr.into();
    attr_object_or_nil(unsafe { *ptr })
}

/// Check whether an [`AttrValue`] is invalid
pub fn attr_is_invalid(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Invalid.try_into()?)
}

/// Check whether an [`AttrValue`] is data
pub fn attr_is_data(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_Data.try_into()?)
}

/// Get the size of an [`AttrValue`]'s data
pub fn attr_data_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_data(attr)?, "Attribute must be data!");
    Ok(attr.private_size)
}

pub fn attr_data<T>(attr: AttrValue) -> Result<T> {
    ensure!(attr_is_data(attr)?, "Attribute must be data!");
    let data: Box<T> = unsafe { Box::from_raw(attr.private_u.data as *mut T) };
    Ok(*data)
}

/// Check whether an [`AttrValue`] is a list
pub fn attr_is_list(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Sim_Val_List.try_into()?)
}

/// Get the size of an [`AttrValue`]'s list
pub fn attr_list_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    Ok(attr.private_size)
}

/// Retrieve a list item from an attr
///
/// # Safety
///
/// The bounds of the list are checked before obtaining an offset, so this function will never
/// crash unless the list size is incorrectly set by SIMICS
pub unsafe fn attr_list_item(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    ensure!(index < attr_list_size(attr)?, "Index out of bounds of list");
    Ok(unsafe { *attr.private_u.list.offset(index.try_into()?) })
}

/// Get the an [`AttrValue`] as a list
pub fn attr_list(attr: AttrValue) -> Result<*mut AttrValue> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    Ok(unsafe { attr.private_u.list }.into())
}

/// Check whether an [`AttrValue`] is a dict
pub fn attr_is_dict(attr: AttrValue) -> bool {
    matches!(attr.private_kind, AttrKind::Sim_Val_Dict)
}

/// Get the size of an an [`AttrValue`]'s dict
pub fn attr_dict_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    Ok(attr.private_size)
}

/// Get a key for an [`AttrValue`]'s dict
pub fn attr_dict_key(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    ensure!(
        index < attr_dict_size(attr)?,
        "Index out of range of dictionary!"
    );
    let pair = unsafe { attr.private_u.dict.offset(index.try_into()?) };
    Ok(unsafe { *pair }.key)
}

/// Get a value for an [`AttrValue`]'s dict
pub fn attr_dict_value(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    ensure!(
        index < attr_dict_size(attr)?,
        "Index out of range of dictionary!"
    );
    let pair = unsafe { attr.private_u.dict.offset(index.try_into()?) };
    Ok(unsafe { *pair }.value)
}

/// Get an attribute of an object
pub fn get_attribute<S>(obj: *mut ConfObject, attribute: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute(obj.into(), raw_cstr(attribute)?) })
}

pub fn free_attribute(attr: AttrValue) {
    unsafe { SIM_free_attribute(attr) }
}

pub fn attr_free(attr: *mut AttrValue) {
    unsafe { SIM_attr_free(attr.into()) }
}

pub fn alloc_attr_list(length: u32) -> AttrValue {
    unsafe { SIM_alloc_attr_list(length) }
}
