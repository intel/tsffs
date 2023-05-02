//! The view SIMICS sees of the controller and the safe bindings to that view
use anyhow::{ensure, Result};
use confuse_simics_api::{conf_class_t, conf_object_t, mm_zalloc};
use confuse_simics_api::{SIM_get_class, SIM_object_class};
use raw_cstr::raw_cstr;
use std::ffi::{c_void, CString};
use std::mem::size_of;
use std::ptr::null_mut;

use crate::module::controller::Controller;

#[allow(non_camel_case_types)]
#[repr(C)]
/// This object is *only* used for snapshot restore
pub struct controller_conf_object_t {
    obj: conf_object_t,
}

impl controller_conf_object_t {
    /// Instantiate a pointer to this struct from a pointer to an object
    ///
    /// # Safety
    ///
    /// This function is safe as long as `obj` is actually a pointer to a `conf_object_t`, and
    /// will remain type safe as guaranteed by simics in this case
    pub unsafe fn try_from_obj(obj: *mut conf_object_t) -> Result<*mut controller_conf_object_t> {
        let cls = unsafe { SIM_get_class(raw_cstr!(Controller::CLASS_NAME)) };
        let obj_cls = unsafe { SIM_object_class(obj) };
        ensure!(
            cls == obj_cls,
            "Object is not an instance of {}",
            Controller::CLASS_NAME
        );
        Ok(obj as *mut controller_conf_object_t)
    }
}

#[no_mangle]
/// Allocate a controler configuration object
pub extern "C" fn alloc_controller_conf_object(_cls: *mut c_void) -> *mut conf_object_t {
    unsafe {
        mm_zalloc(
            size_of::<controller_conf_object_t>(),
            size_of::<controller_conf_object_t>(),
            raw_cstr!("controller_conf_object_t"),
            raw_cstr!(""),
            0x00,
        ) as *mut conf_object_t
    }
}

#[no_mangle]
/// Init a controller configuration object (return the object itself)
pub extern "C" fn init_controller_conf_object(
    obj: *mut conf_object_t,
    _data: *mut c_void,
) -> *mut c_void {
    obj as *mut c_void
}

#[no_mangle]
/// Init a controller configuration object (return the object itself) for SIM_create
pub extern "C" fn init_controller_conf_object_for_create(obj: *mut conf_object_t) -> *mut c_void {
    obj as *mut c_void
}

#[no_mangle]
/// Allocate a controler configuration object
pub extern "C" fn alloc_controller_conf_object_for_create(
    _cls: *mut conf_class_t,
) -> *mut conf_object_t {
    unsafe {
        mm_zalloc(
            size_of::<controller_conf_object_t>(),
            size_of::<controller_conf_object_t>(),
            raw_cstr!("controller_conf_object_t"),
            raw_cstr!(""),
            0x00,
        ) as *mut conf_object_t
    }
}

pub struct ControllerInstance {
    ptr: *mut controller_conf_object_t,
}

impl ControllerInstance {
    /// Instantiate this struct from a pointer to an object
    ///
    /// # Safety
    ///
    /// This function is safe as long as `obj` is actually a pointer to a `conf_object_t`, and
    /// will remain type safe as guaranteed by simics in this case
    pub unsafe fn try_from_obj(obj: *mut conf_object_t) -> Result<Self> {
        Ok(Self {
            ptr: unsafe { controller_conf_object_t::try_from_obj(obj)? },
        })
    }

    pub fn get(&self) -> *mut controller_conf_object_t {
        self.ptr
    }

    pub fn get_as_obj(&mut self) -> &mut conf_object_t {
        unsafe { &mut *(self.ptr as *mut conf_object_t) }
    }
}

impl Default for ControllerInstance {
    fn default() -> Self {
        Self { ptr: null_mut() }
    }
}

unsafe impl Send for ControllerInstance {}
unsafe impl Sync for ControllerInstance {}
