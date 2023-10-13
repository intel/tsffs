// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! conf_object - High level bindings for conf-object.h
//!
//! Defines:
//! - class_data_t
//! - class_info_t
//! - conf_class_t
//! - conf_object_t
//! - object_iter_t

use crate::{
    api::{
        last_error,
        sys::{
            attr_attr_t, class_data_t, class_info_t, class_kind_t, conf_class_t, conf_object_t,
            SIM_class_has_attribute, SIM_copy_class, SIM_create_class, SIM_create_object,
            SIM_current_checkpoint_dir, SIM_delete_object, SIM_delete_objects, SIM_extend_class,
            SIM_get_all_classes, SIM_get_all_objects, SIM_get_attribute,
            SIM_get_attribute_attributes, SIM_get_attribute_idx, SIM_get_class,
            SIM_get_class_interface, SIM_get_class_name, SIM_get_interface, SIM_get_object,
            SIM_object_id, SIM_object_is_configured, SIM_object_name, SIM_register_interface,
            SIM_require_object,
        },
        AttrValue, Interface,
    },
    Error, Result,
};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    get_attr_t, get_class_attr_t, object_iter_t, set_attr_t, set_class_attr_t, set_error_t,
    SIM_attribute_error, SIM_object_descendant, SIM_object_iterator, SIM_object_iterator_next,
    SIM_object_parent, SIM_register_attribute_with_user_data,
    SIM_register_class_attribute_with_user_data, SIM_register_typed_attribute,
    SIM_register_typed_class_attribute, SIM_set_object_configured, SIM_shallow_object_iterator,
};
use simics_macro::simics_exception;
use std::{
    ffi::{c_void, CStr},
    ops::Range,
    path::PathBuf,
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

trait GetTypedAttrClosure: FnOnce(*mut ConfObject, *mut AttrValue) -> AttrValue + 'static {}
trait SetTypedAttrClosure:
    FnOnce(*mut ConfObject, *mut AttrValue, *mut AttrValue) -> SetErr + 'static
{
}
trait GetTypedClassAttrClosure: FnOnce(*mut ConfClass, *mut AttrValue) -> AttrValue + 'static {}
trait SetTypedClassAttrClosure:
    FnOnce(*mut ConfClass, *mut AttrValue, *mut AttrValue) -> SetErr + 'static
{
}
trait GetAttrClosure: FnOnce(*mut ConfObject) -> AttrValue + 'static {}
trait SetAttrClosure: FnOnce(*mut ConfObject, *mut AttrValue) -> SetErr + 'static {}
trait GetClassAttrClosure: FnOnce(*mut ConfClass) -> AttrValue + 'static {}
trait SetClassAttrClosure: FnOnce(*mut ConfClass, *mut AttrValue) -> SetErr + 'static {}

extern "C" fn get_typed_attr_handler<F>(
    cb: *mut c_void,
    obj: *mut ConfObject,
    idx: *mut AttrValue,
) -> AttrValue
where
    F: GetTypedAttrClosure,
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
    F: SetTypedAttrClosure,
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
    F: GetTypedClassAttrClosure,
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
    F: SetTypedClassAttrClosure,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls, val, idx)
}

extern "C" fn get_attr_handler<F>(obj: *mut ConfObject, cb: *mut c_void) -> AttrValue
where
    F: GetAttrClosure,
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
    F: SetAttrClosure,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(obj, val)
}

extern "C" fn get_class_attr_handler<F>(cls: *mut ConfClass, cb: *mut c_void) -> AttrValue
where
    F: GetClassAttrClosure,
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
    F: SetClassAttrClosure,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };

    closure(cls, val)
}

// NOTE: There is an old class creation method, but it is *actually* deprecated, so we do not
// include it with a #[deprecated] warning.

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
            message: last_error()?,
        })
    } else {
        Ok(cls)
    }
}

#[simics_exception]
pub fn extend_class(cls: *mut ConfClass, ext: *mut ConfClass) {
    unsafe { SIM_extend_class(cls, ext) };
}

#[simics_exception]
pub fn copy_class<S>(name: S, src_cls: *mut ConfClass, desc: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_copy_class(raw_cstr(name)?, src_cls, raw_cstr(desc)?) })
}

#[simics_exception]
pub fn get_class_name(class_data: *mut ConfClass) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_get_class_name(class_data)) }
        .to_str()
        .map(|s| s.to_string())?)
}

// NOTE: Not implementing (set|get)_class_data, requires 'static

#[simics_exception]
pub fn require_object(obj: *mut ConfObject) {
    unsafe { SIM_require_object(obj) };
}

#[simics_exception]
pub fn object_name(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_name(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
pub fn object_id(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_id(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
pub fn object_is_configured(obj: *mut ConfObject) -> bool {
    unsafe { SIM_object_is_configured(obj) }
}

#[simics_exception]
pub fn set_object_configured(obj: *mut ConfObject) {
    unsafe { SIM_set_object_configured(obj) }
}

#[simics_exception]
pub fn object_parent(obj: *mut ConfObject) -> *mut ConfObject {
    unsafe { SIM_object_parent(obj) }
}

#[simics_exception]
pub fn object_descendant<S>(obj: *mut ConfObject, relname: S) -> Result<*mut ConfObject>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_object_descendant(obj, raw_cstr(relname)?) })
}

#[simics_exception]
pub fn object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_object_iterator(obj) }
}

#[simics_exception]
pub fn shallow_object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_shallow_object_iterator(obj) }
}

#[simics_exception]
pub fn object_iterator_next(iter: *mut ObjectIter) -> Option<*mut ConfObject> {
    let obj = unsafe { SIM_object_iterator_next(iter) };

    if obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

#[simics_exception]
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
    GF: GetTypedAttrClosure,
    SF: SetTypedAttrClosure,
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
            getter_cb_raw as *mut _ as *mut c_void,
            Some(set_typed_attr_handler::<SF>),
            setter_cb_raw as *mut _ as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
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
    GF: GetTypedClassAttrClosure,
    SF: SetTypedClassAttrClosure,
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
            getter_cb_raw as *mut _ as *mut c_void,
            Some(set_typed_class_attr_handler::<SF>),
            setter_cb_raw as *mut _ as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
pub fn register_attribute<S, GF, SF>(
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
    GF: GetAttrClosure,
    SF: SetAttrClosure,
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
        SIM_register_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            Some(get_attr_handler::<GF>),
            getter_cb_raw as *mut _ as *mut c_void,
            Some(set_attr_handler::<SF>),
            setter_cb_raw as *mut _ as *mut c_void,
            attr,
            attr_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
pub fn register_class_attribute<S, GF, SF>(
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
    GF: GetClassAttrClosure,
    SF: SetClassAttrClosure,
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
        SIM_register_class_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            Some(get_class_attr_handler::<GF>),
            getter_cb_raw as *mut _ as *mut c_void,
            Some(set_class_attr_handler::<SF>),
            setter_cb_raw as *mut _ as *mut c_void,
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
pub fn attribute_error<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_attribute_error(raw_cstr(msg)?) })
}

// NOTE: add_configuration not implemented, it is only to be used from Python

#[simics_exception]
/// Check if a class implements an attribute
pub fn class_has_attribute<S>(cls: *mut ConfClass, attr: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_class_has_attribute(cls, raw_cstr(attr.as_ref())?) })
}

#[simics_exception]
/// Create a new instance of a configuration class
pub fn create_object<S>(cls: *mut ConfClass, name: S, attrs: AttrValue) -> Result<*mut ConfObject>
where
    S: AsRef<str>,
{
    let obj = unsafe { SIM_create_object(cls.into(), raw_cstr(name)?, attrs) };

    if obj.is_null() {
        Err(Error::CreateObject {
            message: last_error()?,
        })
    } else {
        Ok(obj)
    }
}

/// Get the current checkpoint (bundle) directory if called during loading of a checkpoint.
/// May be absolute or relative.
pub fn current_checkpoint_dir() -> Result<PathBuf> {
    let res = unsafe { SIM_current_checkpoint_dir() };

    if res.is_null() {
        Err(Error::CurrentCheckpointDir {
            message: last_error()?,
        })
    } else {
        let mut dir = unsafe { CStr::from_ptr(res) }.to_str()?;

        if dir.is_empty() {
            dir = ".";
        }

        Ok(PathBuf::from(dir))
    }
}

#[simics_exception]
/// Delete the objects in the list or throw an exception if unsuccessful
pub fn delete_objects(val: AttrValue) {
    unsafe { SIM_delete_objects(val) };
}

#[simics_exception]
/// Delete the object or throw an exception if unsuccessful
pub fn delete_object(obj: *mut ConfObject) {
    unsafe { SIM_delete_object(obj) };
}

/// Get an unordered list of all conf classes in the simulator
pub fn get_all_classes() -> AttrValue {
    unsafe { SIM_get_all_classes() }
}

/// Get an unordered list of all conf objects in the simulator
pub fn get_all_objects() -> AttrValue {
    unsafe { SIM_get_all_objects() }
}

#[simics_exception]
/// Get the attribute of a given name from an object
pub fn get_attribute<S>(obj: *mut ConfObject, name: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute(obj, raw_cstr(name)?) })
}

#[simics_exception]
/// Get an index of an attribute of a given name from an object
pub fn get_attribute_idx<S>(
    obj: *mut ConfObject,
    name: S,
    index: *mut AttrValue,
) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute_idx(obj, raw_cstr(name)?, index) })
}

/// Get the flags of an attribute in a class
pub fn get_attribute_attributes<S>(cls: *mut ConfClass, attr: S) -> Result<AttrAttr>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute_attributes(cls, raw_cstr(attr)?) })
}

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
            message: last_error()?,
        })
    } else {
        Ok(status)
    }
}

#[simics_exception]
/// Get an interface of an object
pub fn get_interface<T>(obj: *mut ConfObject, iface: Interface) -> Result<*mut T> {
    Ok(unsafe {
        SIM_get_interface(
            obj as *const ConfObject,
            iface.try_as_slice()?.as_ptr() as *const i8,
        ) as *mut T
    })
}

#[simics_exception]
/// Get an interface of a class
pub fn get_class_interface<T>(cls: *mut ConfClass, iface: Interface) -> Result<*mut T> {
    Ok(unsafe {
        SIM_get_class_interface(
            cls as *const ConfClass,
            iface.try_as_slice()?.as_ptr() as *const i8,
        ) as *mut T
    })
}

// TODO: Add Port Interfaces

#[simics_exception]
/// Get a class instance by name
pub fn get_class<S>(name: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    let name_raw = raw_cstr(name.as_ref())?;

    let cls = unsafe { SIM_get_class(name_raw) };

    if cls.is_null() {
        Err(Error::ClassNotFound {
            name: name.as_ref().to_string(),
        })
    } else {
        Ok(cls)
    }
}

#[simics_exception]
/// Get an existing instance object by name
pub fn get_object<S>(name: S) -> Result<*mut ConfObject>
where
    S: AsRef<str>,
{
    let obj = unsafe { SIM_get_object(raw_cstr(name.as_ref())?) };

    if obj.is_null() {
        Err(Error::ObjectNotFound {
            name: name.as_ref().to_string(),
        })
    } else {
        Ok(obj)
    }
}
