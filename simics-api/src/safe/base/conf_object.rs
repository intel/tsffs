//! conf_object - High level bindings for conf-object.h
//!
//! Defines:
//! - class_data_t
//! - class_info_t
//! - conf_class_t
//! - conf_object_t
//! - object_iter_t

use crate::{last_error, Interface};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    class_data_t, class_info_t, class_kind_t_Sim_Class_Kind_Extension,
    class_kind_t_Sim_Class_Kind_Pseudo, class_kind_t_Sim_Class_Kind_Session,
    class_kind_t_Sim_Class_Kind_Vanilla, conf_class_t, conf_object_t, SIM_c_get_interface,
    SIM_create_class, SIM_get_class, SIM_register_class, SIM_register_interface,
};

pub type ConfObject = conf_object_t;
pub type ConfClass = conf_class_t;
pub type ClassData = class_data_t;
pub type ClassInfo = class_info_t;

#[derive(Debug, Eq, PartialEq)]
#[repr(u32)]
/// Kinds of classes
pub enum ClassKind {
    Vanilla = class_kind_t_Sim_Class_Kind_Vanilla,
    Session = class_kind_t_Sim_Class_Kind_Session,
    Pseudo = class_kind_t_Sim_Class_Kind_Pseudo,
    Extension = class_kind_t_Sim_Class_Kind_Extension,
}

/// Register a class with a given name
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

/// Create a class instance
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

/// Register an interface for a class
pub fn register_interface<S: AsRef<str>, T>(cls: *mut ConfClass, name: S) -> Result<i32>
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

/// Get an interface of an object
pub fn get_interface<T>(obj: *mut ConfObject, iface: Interface) -> *mut T {
    unsafe {
        SIM_c_get_interface(
            obj as *const ConfObject,
            iface.as_slice().as_ptr() as *const i8,
        ) as *mut T
    }
}

/// Get a class instance by name
pub fn get_class<S: AsRef<str>>(name: S) -> Result<*mut ConfClass> {
    let name_raw = raw_cstr(name.as_ref())?;

    let cls = unsafe { SIM_get_class(name_raw) };

    if cls.is_null() {
        bail!("Failed to get class {}: {}", name.as_ref(), last_error());
    } else {
        Ok(cls)
    }
}
