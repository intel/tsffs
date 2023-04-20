//! conf_object - High level bindings for conf-object.h
//!
//! Defines:
//! - class_data_t
//! - class_info_t
//! - conf_class_t
//! - conf_object_t
//! - object_iter_t

use crate::{last_error, EventClass};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    class_data_t, class_info_t, conf_class_t, conf_object_t, SIM_create_class, SIM_get_class,
    SIM_register_class, SIM_register_event, SIM_register_interface,
};
use std::{ffi::c_void, mem::transmute};

pub type ConfObject = conf_object_t;
pub type ConfClass = conf_class_t;
pub type ClassData = class_data_t;
pub type ClassInfo = class_info_t;

#[derive(Debug)]
#[repr(C)]
pub struct OwnedMutConfObjectPtr {
    object: *mut ConfObject,
}

impl OwnedMutConfObjectPtr {
    pub fn new(object: *mut ConfObject) -> Self {
        Self { object }
    }

    pub fn as_const(&self) -> *const ConfObject {
        self.object as *const ConfObject
    }
}

impl From<*mut ConfObject> for OwnedMutConfObjectPtr {
    fn from(value: *mut ConfObject) -> Self {
        Self::new(value)
    }
}

impl From<OwnedMutConfObjectPtr> for *mut ConfObject {
    fn from(value: OwnedMutConfObjectPtr) -> Self {
        value.object
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct OwnedMutConfClassPtr {
    cls: *mut ConfClass,
}

impl OwnedMutConfClassPtr {
    pub fn new(cls: *mut ConfClass) -> Self {
        Self { cls }
    }
}

impl From<*mut ConfClass> for OwnedMutConfClassPtr {
    fn from(value: *mut ConfClass) -> Self {
        Self::new(value)
    }
}

impl From<OwnedMutConfClassPtr> for *mut ConfClass {
    fn from(val: OwnedMutConfClassPtr) -> Self {
        val.cls
    }
}

pub fn register_class<S: AsRef<str>>(
    name: S,
    class_data: ClassData,
) -> Result<OwnedMutConfClassPtr> {
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_register_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_register_class(name_raw, &class_data as *const class_data_t) };

    if cls.is_null() {
        bail!("Failed to register class: {}", last_error());
    } else {
        Ok(cls.into())
    }
}

pub fn create_class<S: AsRef<str>>(name: S, class_info: ClassInfo) -> Result<OwnedMutConfClassPtr> {
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_create_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_create_class(name_raw, &class_info as *const class_info_t) };

    if cls.is_null() {
        bail!(
            "Failed to register class {}: {}",
            name.as_ref(),
            last_error()
        );
    } else {
        Ok(cls.into())
    }
}

pub fn register_interface<S: AsRef<str>, T>(cls: OwnedMutConfClassPtr, name: S) -> Result<i32>
where
    T: Default,
{
    let name_raw = raw_cstr(name.as_ref())?;
    let iface_box = Box::<T>::default();
    // Note: This allocates and never frees. This is *required* by SIMICS and it is an error to
    // free this pointer
    let iface_raw = Box::into_raw(iface_box);
    let status = unsafe { SIM_register_interface(cls.into(), name_raw, iface_raw as *mut _) };

    if status != 0 {
        bail!(
            "Failed to register interface {}: {}",
            name.as_ref(),
            last_error()
        );
    } else {
        Ok(status)
    }
}

pub fn get_class<S: AsRef<str>>(name: S) -> Result<OwnedMutConfClassPtr> {
    let name_raw = raw_cstr(name.as_ref())?;

    let cls = unsafe { SIM_get_class(name_raw) };

    if cls.is_null() {
        bail!("Failed to get class {}: {}", name.as_ref(), last_error());
    } else {
        Ok(cls.into())
    }
}
pub fn register_event<S: AsRef<str>>(
    name: S,
    cls: OwnedMutConfClassPtr,
    callback: unsafe extern "C" fn(*mut ConfObject, *mut c_void),
) -> Result<EventClass> {
    let name_raw = raw_cstr(name.as_ref())?;
    let event = unsafe {
        SIM_register_event(
            name_raw,
            cls.into(),
            0,
            transmute(callback),
            None,
            None,
            None,
            None,
        )
    };

    if event.is_null() {
        bail!(
            "Unable to register event {}: {}",
            name.as_ref(),
            last_error()
        );
    } else {
        Ok(event.into())
    }
}
