// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simulator configuration object handling

use crate::{
    last_error, simics_exception,
    sys::{
        SIM_class_has_attribute, SIM_class_port, SIM_create_object, SIM_delete_object,
        SIM_delete_objects, SIM_get_all_classes, SIM_get_all_objects, SIM_get_attribute,
        SIM_get_attribute_attributes, SIM_get_attribute_idx, SIM_get_class_attribute,
        SIM_get_class_attribute_idx, SIM_get_object, SIM_set_attribute, SIM_set_attribute_default,
        SIM_set_attribute_idx, SIM_set_class_attribute, SIM_set_class_attribute_idx,
    },
    AttrAttr, AttrValue, ConfClass, ConfObject, Error, Result, SetErr,
};
use raw_cstr::raw_cstr;

#[simics_exception]
/// Get the flags of an attribute in a class
pub fn get_attribute_attributes<S>(cls: *mut ConfClass, attr: S) -> Result<AttrAttr>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute_attributes(cls, raw_cstr(attr)?) })
}

#[simics_exception]
/// Check if a class implements an attribute
pub fn class_has_attribute<S>(cls: *mut ConfClass, attr: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_class_has_attribute(cls, raw_cstr(attr.as_ref())?) })
}

#[simics_exception]
/// Get a port for a class
pub fn class_port<S>(cls: *mut ConfClass, portname: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_class_port(cls, raw_cstr(portname)?) })
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

#[simics_exception]
/// Delete the object or throw an exception if unsuccessful
pub fn delete_object(obj: *mut ConfObject) {
    unsafe { SIM_delete_object(obj) };
}

#[simics_exception]
/// Delete the objects in the list or throw an exception if unsuccessful
pub fn delete_objects(val: AttrValue) {
    unsafe { SIM_delete_objects(val.into()) };
}

#[simics_exception]
/// Get the attribute of a given name from an object
pub fn get_attribute<S>(obj: *mut ConfObject, name: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute(obj, raw_cstr(name)?) }.into())
}

#[simics_exception]
/// Get an index of an attribute of a given name from an object
pub fn get_attribute_idx<S>(
    obj: *mut ConfObject,
    name: S,
    index: &mut AttrValue,
) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_attribute_idx(obj, raw_cstr(name)?, index.as_mut_ptr()) }.into())
}

#[simics_exception]
/// Get the attribute of a given name from a class
pub fn get_class_attribute<S>(obj: *mut ConfClass, name: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_class_attribute(obj, raw_cstr(name)?) }.into())
}

#[simics_exception]
/// Get an index of an attribute of a given name from a class
pub fn get_class_attribute_idx<S>(
    obj: *mut ConfClass,
    name: S,
    index: &mut AttrValue,
) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_get_class_attribute_idx(obj, raw_cstr(name)?, index.as_mut_ptr()) }.into())
}

#[simics_exception]
/// Set an attribute value on an object
pub fn set_attribute<S>(obj: *mut ConfObject, name: S, value: &mut AttrValue) -> Result<SetErr>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_set_attribute(obj, raw_cstr(name)?, value.as_mut_ptr()) })
}

#[simics_exception]
/// Set an attribute value at an index on an object
pub fn set_attribute_idx<S>(
    obj: *mut ConfObject,
    name: S,
    index: &mut AttrValue,
    value: &mut AttrValue,
) -> Result<SetErr>
where
    S: AsRef<str>,
{
    Ok(unsafe {
        SIM_set_attribute_idx(obj, raw_cstr(name)?, index.as_mut_ptr(), value.as_mut_ptr())
    })
}

#[simics_exception]
/// Set the default value of an attribute on an object
pub fn set_attribute_default<S>(obj: *mut ConfObject, name: S, value: AttrValue) -> Result<SetErr>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_set_attribute_default(obj, raw_cstr(name)?, value.into()) })
}

#[simics_exception]
/// Set the default value of an attribute on a class
pub fn set_class_attribute<S>(cls: *mut ConfClass, name: S, value: &mut AttrValue) -> Result<SetErr>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_set_class_attribute(cls, raw_cstr(name)?, value.as_mut_ptr()) })
}

#[simics_exception]
/// Set the default value of an attribute on a class with an index
pub fn set_class_attribute_idx<S>(
    cls: *mut ConfClass,
    name: S,
    index: &mut AttrValue,
    value: &mut AttrValue,
) -> Result<SetErr>
where
    S: AsRef<str>,
{
    Ok(unsafe {
        SIM_set_class_attribute_idx(cls, raw_cstr(name)?, index.as_mut_ptr(), value.as_mut_ptr())
    })
}

#[simics_exception]
/// Get an unordered list of all conf classes in the simulator
pub fn get_all_classes() -> AttrValue {
    unsafe { SIM_get_all_classes() }.into()
}

#[simics_exception]
/// Get an unordered list of all conf objects in the simulator
pub fn get_all_objects() -> AttrValue {
    unsafe { SIM_get_all_objects() }.into()
}

#[simics_exception]
/// Create a new instance of a configuration class
pub fn create_object<S>(cls: *mut ConfClass, name: S, attrs: AttrValue) -> Result<*mut ConfObject>
where
    S: AsRef<str>,
{
    let obj = unsafe { SIM_create_object(cls, raw_cstr(name)?, attrs.into()) };

    if obj.is_null() {
        Err(Error::CreateObject {
            message: last_error(),
        })
    } else {
        Ok(obj)
    }
}
