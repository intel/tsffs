// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]

//! Bindings for configuration objects

use crate::{
    api::{
        last_error,
        sys::{
            attr_attr_t, class_data_t, class_info_t, class_kind_t, conf_class_t, conf_object_t,
            get_attr_t, get_class_attr_t, object_iter_t, set_attr_t, set_class_attr_t, set_error_t,
            SIM_attribute_error, SIM_copy_class, SIM_create_class, SIM_extend_class,
            SIM_extension_data, SIM_get_class_data, SIM_get_class_interface, SIM_get_class_name,
            SIM_get_interface, SIM_marked_for_deletion, SIM_object_data, SIM_object_descendant,
            SIM_object_id, SIM_object_is_configured, SIM_object_iterator, SIM_object_iterator_next,
            SIM_object_name, SIM_object_parent, SIM_register_attribute_with_user_data,
            SIM_register_class_alias, SIM_register_class_attribute_with_user_data,
            SIM_register_interface, SIM_register_typed_attribute,
            SIM_register_typed_class_attribute, SIM_require_object, SIM_set_class_data,
            SIM_set_object_configured, SIM_shallow_object_iterator,
        },
        AttrValue, Interface,
    },
    Error, Result,
};
use raw_cstr::{raw_cstr, AsRawCstr};
use simics_macro::simics_exception;
use std::{
    ffi::{c_void, CStr},
    ops::Range,
    ptr::null_mut,
};

pub type ConfObject = conf_object_t;
pub type ConfClass = conf_class_t;
pub type ClassData = class_data_t;
pub type ClassInfo = class_info_t;
pub type ClassKind = class_kind_t;
pub type AttrAttr = attr_attr_t;
pub type ObjectIter = object_iter_t;
pub type GetAttr = get_attr_t;
pub type SetAttr = set_attr_t;
pub type GetClassAttr = get_class_attr_t;
pub type SetClassAttr = set_class_attr_t;
pub type SetErr = set_error_t;

/// A type in a [`TypeStringType::List`]
pub enum TypeStringListType {
    Type(Box<TypeStringType>),
    Range(Range<usize>, Box<TypeStringType>),
    Exact(usize, Box<TypeStringType>),
    ZeroOrMore(Box<TypeStringType>),
    OneOrMore(Box<TypeStringType>),
}

impl ToString for TypeStringListType {
    fn to_string(&self) -> String {
        match self {
            TypeStringListType::Type(t) => t.to_string(),
            TypeStringListType::Range(r, t) => {
                format!("{}{{{}:{}}}", t.to_string(), r.start, r.end)
            }
            TypeStringListType::Exact(c, t) => format!("{}{{{}}}", t.to_string(), c),
            TypeStringListType::ZeroOrMore(t) => format!("{}*", t.to_string()),
            TypeStringListType::OneOrMore(t) => format!("{}+", t.to_string()),
        }
    }
}

/// A type in a python-like type string
pub enum TypeStringType {
    Integer,
    Float,
    String,
    Boolean,
    Object,
    Data,
    Nil,
    Dictionary,
    Any,
    List(Vec<TypeStringListType>),
    Or(Box<TypeStringType>, Box<TypeStringType>),
}

impl ToString for TypeStringType {
    fn to_string(&self) -> String {
        match self {
            TypeStringType::Integer => "i".to_string(),
            TypeStringType::Float => "f".to_string(),
            TypeStringType::String => "s".to_string(),
            TypeStringType::Boolean => "b".to_string(),
            TypeStringType::Object => "o".to_string(),
            TypeStringType::Data => "d".to_string(),
            TypeStringType::Nil => "n".to_string(),
            TypeStringType::Dictionary => "D".to_string(),
            TypeStringType::Any => "a".to_string(),
            TypeStringType::List(l) => format!(
                "[{}]",
                l.iter().map(|li| li.to_string()).collect::<String>()
            ),
            TypeStringType::Or(l, r) => format!("{}|{}", l.to_string(), r.to_string()),
        }
    }
}

// NOTE: There is an old class creation method, but it is *actually* deprecated, so we do not
// include it with a #[deprecated] warning.

#[simics_exception]
pub fn register_class_alias<S>(alias: S, name: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_register_class_alias(raw_cstr(alias)?, raw_cstr(name)?) };
    Ok(())
}

#[simics_exception]
/// Create a class instance
pub fn create_class<S>(name: S, class_info: ClassInfo) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_create_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_create_class(name_raw, &class_info as *const ClassInfo) };

    if cls.is_null() {
        Err(Error::CreateClass {
            name: name.as_ref().to_string(),
            message: last_error(),
        })
    } else {
        Ok(cls)
    }
}

#[simics_exception]
/// Extend a class with another class
pub fn extend_class(cls: *mut ConfClass, ext: *mut ConfClass) {
    unsafe { SIM_extend_class(cls, ext) };
}

#[simics_exception]
/// Make a copy of a class with another name and description
pub fn copy_class<S>(name: S, src_cls: *mut ConfClass, desc: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_copy_class(raw_cstr(name)?, src_cls, raw_cstr(desc)?) })
}

#[simics_exception]
/// Get the name of a class
pub fn get_class_name(class_data: *mut ConfClass) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_get_class_name(class_data)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
pub fn set_class_data<T>(cls: *mut ConfClass, data: T) {
    unsafe { SIM_set_class_data(cls, Box::into_raw(Box::new(data)) as *mut c_void) }
}

#[simics_exception]
pub fn get_class_data<T>(cls: *mut ConfClass) -> Box<T> {
    unsafe { Box::from_raw(SIM_get_class_data(cls) as *mut T) }
}

#[simics_exception]
/// Require an object to be finalized. If it has not been finalized, finalized will be called.
pub fn require_object(obj: *mut ConfObject) {
    unsafe { SIM_require_object(obj) };
}

#[simics_exception]
/// Get the name of an object
pub fn object_name(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_name(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
/// Get the id of an object
pub fn object_id(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_id(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
/// Check whether an object is configured
pub fn object_is_configured(obj: *mut ConfObject) -> bool {
    unsafe { SIM_object_is_configured(obj) }
}

#[simics_exception]
/// Set an object as configured
pub fn set_object_configured(obj: *mut ConfObject) {
    unsafe { SIM_set_object_configured(obj) }
}

#[simics_exception]
pub fn object_data<T>(obj: *mut ConfObject) -> Box<T> {
    unsafe { Box::from_raw(SIM_object_data(obj) as *mut T) }
}

#[simics_exception]
pub fn extension_data<T>(obj: *mut ConfObject, cls: *mut ConfClass) -> Box<T> {
    unsafe { Box::from_raw(SIM_extension_data(obj, cls) as *mut T) }
}

#[simics_exception]
/// Get the parent of an object
pub fn object_parent(obj: *mut ConfObject) -> *mut ConfObject {
    unsafe { SIM_object_parent(obj) }
}

#[simics_exception]
/// Get an object's child object, if it has one with a given name
pub fn object_descendant<S>(obj: *mut ConfObject, relname: S) -> Result<*mut ConfObject>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_object_descendant(obj, raw_cstr(relname)?) })
}

#[simics_exception]
/// Obtain an iterator over the child objects at all depths of a given object
pub fn object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_object_iterator(obj) }
}

#[simics_exception]
/// Obtain an iterator over the child objects at depth 1 of a given object
pub fn shallow_object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_shallow_object_iterator(obj) }
}

#[simics_exception]
/// Consume and return the next item of an object iterator.
pub fn object_iterator_next(iter: *mut ObjectIter) -> Option<*mut ConfObject> {
    let obj = unsafe { SIM_object_iterator_next(iter) };

    if obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

extern "C" fn get_typed_attr_handler<F>(
    cb: *mut c_void,
    obj: *mut ConfObject,
    idx: *mut AttrValue,
) -> AttrValue
where
    F: FnOnce(*mut ConfObject, *mut AttrValue) -> AttrValue + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(obj, idx)
}

extern "C" fn set_typed_attr_handler<F>(
    cb: *mut c_void,
    obj: *mut ConfObject,
    val: *mut AttrValue,
    idx: *mut AttrValue,
) -> SetErr
where
    F: FnOnce(*mut ConfObject, *mut AttrValue, *mut AttrValue) -> SetErr + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(obj, val, idx)
}

extern "C" fn get_typed_class_attr_handler<F>(
    cb: *mut c_void,
    cls: *mut ConfClass,
    idx: *mut AttrValue,
) -> AttrValue
where
    F: FnOnce(*mut ConfClass, *mut AttrValue) -> AttrValue + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls, idx)
}

extern "C" fn set_typed_class_attr_handler<F>(
    cb: *mut c_void,
    cls: *mut ConfClass,
    val: *mut AttrValue,
    idx: *mut AttrValue,
) -> SetErr
where
    F: FnOnce(*mut ConfClass, *mut AttrValue, *mut AttrValue) -> SetErr + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls, val, idx)
}

extern "C" fn get_attr_handler<F>(obj: *mut ConfObject, cb: *mut c_void) -> AttrValue
where
    F: FnOnce(*mut ConfObject) -> AttrValue + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(obj)
}

extern "C" fn set_attr_handler<F>(
    obj: *mut ConfObject,
    val: *mut AttrValue,
    cb: *mut c_void,
) -> SetErr
where
    F: FnOnce(*mut ConfObject, *mut AttrValue) -> SetErr + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(obj, val)
}

extern "C" fn get_class_attr_handler<F>(cls: *mut ConfClass, cb: *mut c_void) -> AttrValue
where
    F: FnOnce(*mut ConfClass) -> AttrValue + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls)
}

extern "C" fn set_class_attr_handler<F>(
    cls: *mut ConfClass,
    val: *mut AttrValue,
    cb: *mut c_void,
) -> SetErr
where
    F: FnOnce(*mut ConfClass, *mut AttrValue) -> SetErr + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls, val)
}

#[simics_exception]
/// Register a typed attribute of a class. This attribute will appear on all instances of the
/// class.
pub fn register_typed_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: GF,
    setter: SF,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    idx_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnOnce(*mut ConfObject, *mut AttrValue) -> AttrValue + 'static,
    SF: FnOnce(*mut ConfObject, *mut AttrValue, *mut AttrValue) -> SetErr + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let idx_type = if let Some(idx_type) = idx_type {
        raw_cstr(idx_type.to_string())?
    } else {
        null_mut()
    };

    let getter_cb = Box::new(getter);
    let getter_cb_box = Box::new(getter_cb);
    let getter_cb_raw = Box::into_raw(getter_cb_box);
    let setter_cb = Box::new(setter);
    let setter_cb_box = Box::new(setter_cb);
    let setter_cb_raw = Box::into_raw(setter_cb_box);

    unsafe {
        SIM_register_typed_attribute(
            cls,
            raw_cstr(name)?,
            Some(get_typed_attr_handler::<GF>),
            getter_cb_raw as *mut c_void,
            Some(set_typed_attr_handler::<SF>),
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a typed attribute of a class. This attribute will appear on the class object itself.
pub fn register_typed_class_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: GF,
    setter: SF,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    idx_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnOnce(*mut ConfClass, *mut AttrValue) -> AttrValue + 'static,
    SF: FnOnce(*mut ConfClass, *mut AttrValue, *mut AttrValue) -> SetErr + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let idx_type = if let Some(idx_type) = idx_type {
        raw_cstr(idx_type.to_string())?
    } else {
        null_mut()
    };

    let getter_cb = Box::new(getter);
    let getter_cb_box = Box::new(getter_cb);
    let getter_cb_raw = Box::into_raw(getter_cb_box);
    let setter_cb = Box::new(setter);
    let setter_cb_box = Box::new(setter_cb);
    let setter_cb_raw = Box::into_raw(setter_cb_box);

    unsafe {
        SIM_register_typed_class_attribute(
            cls,
            raw_cstr(name)?,
            Some(get_typed_class_attr_handler::<GF>),
            getter_cb_raw as *mut c_void,
            Some(set_typed_class_attr_handler::<SF>),
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a pseudo-untyped attribute of the instances of a class.
pub fn register_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: GF,
    setter: SF,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnOnce(*mut ConfObject) -> AttrValue + 'static,
    SF: FnOnce(*mut ConfObject, *mut AttrValue) -> SetErr + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let getter_cb = Box::new(getter);
    let getter_cb_box = Box::new(getter_cb);
    let getter_cb_raw = Box::into_raw(getter_cb_box);
    let setter_cb = Box::new(setter);
    let setter_cb_box = Box::new(setter_cb);
    let setter_cb_raw = Box::into_raw(setter_cb_box);

    unsafe {
        SIM_register_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            Some(get_attr_handler::<GF>),
            getter_cb_raw as *mut c_void,
            Some(set_attr_handler::<SF>),
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a pseudo-untyped attribute on a class itself.
pub fn register_class_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: GF,
    setter: SF,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnOnce(*mut ConfClass) -> AttrValue + 'static,
    SF: FnOnce(*mut ConfClass, *mut AttrValue) -> SetErr + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let getter_cb = Box::new(getter);
    let getter_cb_box = Box::new(getter_cb);
    let getter_cb_raw = Box::into_raw(getter_cb_box);
    let setter_cb = Box::new(setter);
    let setter_cb_box = Box::new(setter_cb);
    let setter_cb_raw = Box::into_raw(setter_cb_box);

    unsafe {
        SIM_register_class_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            Some(get_class_attr_handler::<GF>),
            getter_cb_raw as *mut c_void,
            Some(set_class_attr_handler::<SF>),
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

// NOTE: We do not provide unuserdata untyped registration functions, we only want to register
// typed attributes, and we need userdata for our handlers

#[simics_exception]
/// When called inside a getter or setter callback (e.g. inside [`register_attribute`]'s
/// `getter` or `setter`), this function marks the get or set as an error and sets the
/// error message.
pub fn attribute_error<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_attribute_error(raw_cstr(msg)?) };
    Ok(())
}

// NOTE: add_configuration not implemented, it is only to be used from Python

#[simics_exception]
/// Register an interface for a class
pub fn register_interface<S, T>(cls: *mut ConfClass, name: S) -> Result<i32>
where
    S: AsRef<str>,
    T: Default,
{
    let name_raw = raw_cstr(name.as_ref())?;
    let iface_box = Box::<T>::default();
    // Note: This allocates and never frees. This is *required* by SIMICS and it is an error to
    // free this pointer
    let iface_raw = Box::into_raw(iface_box);

    debug_assert!(
        std::mem::size_of_val(&iface_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    let status = unsafe { SIM_register_interface(cls, name_raw, iface_raw as *mut _) };

    if status != 0 {
        Err(Error::RegisterInterface {
            name: name.as_ref().to_string(),
            message: last_error(),
        })
    } else {
        Ok(status)
    }
}

// TODO: Port & compatible interfaces

#[simics_exception]
/// Get an interface of an object
pub fn get_interface<I>(obj: *mut ConfObject) -> Result<*mut I::Interface>
where
    I: Interface,
{
    Ok(unsafe {
        SIM_get_interface(obj as *const ConfObject, I::NAME.as_raw_cstr()?) as *mut I::Interface
    })
}

#[simics_exception]
/// Get an interface of a class
pub fn get_class_interface<I>(cls: *mut ConfClass) -> Result<*mut I::Interface>
where
    I: Interface,
{
    Ok(unsafe {
        SIM_get_class_interface(cls as *const ConfClass, I::NAME.as_raw_cstr()?)
            as *mut I::Interface
    })
}

// TODO: Add Port Interfaces

#[simics_exception]
/// Check whether an object has been marked for deletion
pub fn marked_for_deletion(obj: *mut ConfObject) -> bool {
    unsafe { SIM_marked_for_deletion(obj) }
}
