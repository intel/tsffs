//! conf_object - High level bindings for conf-object.h
//!
//! Defines:
//! - class_data_t
//! - class_info_t
//! - conf_class_t
//! - conf_object_t
//! - object_iter_t

use crate::{last_error, EventClass, Interface};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    class_data_t, class_info_t, class_kind_t_Sim_Class_Kind_Extension,
    class_kind_t_Sim_Class_Kind_Pseudo, class_kind_t_Sim_Class_Kind_Session,
    class_kind_t_Sim_Class_Kind_Vanilla, conf_class_t, conf_object_t, SIM_c_get_interface,
    SIM_create_class, SIM_get_class, SIM_register_class, SIM_register_event,
    SIM_register_interface,
};
use std::{ffi::c_void, mem::transmute};

pub type ConfObject = conf_object_t;
pub type ConfClass = conf_class_t;
pub type ClassData = class_data_t;
pub type ClassInfo = class_info_t;

#[derive(Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ClassKind {
    Vanilla = class_kind_t_Sim_Class_Kind_Vanilla,
    Session = class_kind_t_Sim_Class_Kind_Session,
    Pseudo = class_kind_t_Sim_Class_Kind_Pseudo,
    Extension = class_kind_t_Sim_Class_Kind_Extension,
}

pub fn register_class<S: AsRef<str>>(name: S, class_data: ClassData) -> Result<*mut ConfClass> {
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_register_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_register_class(name_raw, &class_data as *const ClassData) };

    if cls.is_null() {
        bail!("Failed to register class: {}", last_error());
    } else {
        Ok(cls)
    }
}

pub fn create_class<S: AsRef<str>>(name: S, class_info: ClassInfo) -> Result<*mut ConfClass> {
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_create_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_create_class(name_raw, &class_info as *const ClassInfo) };

    if cls.is_null() {
        bail!(
            "Failed to register class {}: {}",
            name.as_ref(),
            last_error()
        );
    } else {
        Ok(cls)
    }
}

pub fn register_interface<S: AsRef<str>, T>(cls: &ConfClass, name: S) -> Result<i32>
where
    T: Default,
{
    let name_raw = raw_cstr(name.as_ref())?;
    let iface_box = Box::<T>::default();
    // Note: This allocates and never frees. This is *required* by SIMICS and it is an error to
    // free this pointer
    let iface_raw = Box::into_raw(iface_box);
    let mut cls = *cls;
    let status = unsafe {
        SIM_register_interface(&mut cls as *mut ConfClass, name_raw, iface_raw as *mut _)
    };

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

pub fn get_interface<T>(obj: *mut ConfObject, iface: Interface) -> *mut T {
    unsafe {
        SIM_c_get_interface(
            obj as *const ConfObject,
            iface.as_slice().as_ptr() as *const i8,
        ) as *mut T
    }
}

pub fn get_class<S: AsRef<str>>(name: S) -> Result<*mut ConfClass> {
    let name_raw = raw_cstr(name.as_ref())?;

    let cls = unsafe { SIM_get_class(name_raw) };

    if cls.is_null() {
        bail!("Failed to get class {}: {}", name.as_ref(), last_error());
    } else {
        Ok(cls)
    }
}
pub fn register_event<S: AsRef<str>>(
    name: S,
    cls: &ConfClass,
    callback: unsafe extern "C" fn(*mut ConfObject, *mut c_void),
) -> Result<*mut EventClass> {
    let name_raw = raw_cstr(name.as_ref())?;
    let mut cls = *cls;
    let event = unsafe {
        SIM_register_event(
            name_raw,
            &mut cls as *mut ConfClass,
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
        Ok(event)
    }
}
