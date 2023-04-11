use crate::{
    class_data_t, class_info_t, conf_class_t, conf_object_t, event_class_t,
    micro_checkpoint_flags_t_Sim_MC_ID_User, micro_checkpoint_flags_t_Sim_MC_Persistent,
    safe::types::{ClassData, ClassInfo, ConfClass, ConfObject, EventClass},
    CORE_discard_future, SIM_break_simulation, SIM_create_class, SIM_event_cancel_time,
    SIM_event_find_next_time, SIM_event_post_time, SIM_get_class, SIM_last_error, SIM_object_clock,
    SIM_quit, SIM_register_class, SIM_register_event, SIM_register_interface,
    VT_restore_micro_checkpoint, VT_save_micro_checkpoint,
};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use std::{
    ffi::{c_int, c_void, CStr, CString},
    mem::transmute,
    ptr::null_mut,
};

use super::types::SendSyncRawPointer;

pub fn quit() {
    unsafe {
        SIM_quit(0);
    }
}

pub fn restore_micro_checkpoint(index: i32) {
    unsafe {
        VT_restore_micro_checkpoint(index as c_int);
    }
}

pub fn save_micro_checkpoint<S: AsRef<str>>(name: S) {
    unsafe {
        VT_save_micro_checkpoint(
            raw_cstr!(name.as_ref()),
            micro_checkpoint_flags_t_Sim_MC_ID_User | micro_checkpoint_flags_t_Sim_MC_Persistent,
        )
    }
}

pub fn discard_future() {
    unsafe {
        CORE_discard_future();
    }
}

pub fn break_simulation<S: AsRef<str>>(msg: S) {
    unsafe { SIM_break_simulation(raw_cstr!(msg.as_ref())) }
}

pub fn last_error() -> String {
    let error_str = unsafe { CStr::from_ptr(SIM_last_error()) };
    error_str.to_string_lossy().to_string()
}

pub fn register_class<S: AsRef<str>>(name: S, class_data: ClassData) -> Result<ConfClass> {
    let name_raw = raw_cstr!(name.as_ref());
    let cls = unsafe { SIM_register_class(name_raw, &class_data as *const class_data_t) };
    if cls.is_null() {
        bail!("Failed to register class: {}", last_error());
    } else {
        Ok(ConfClass(SendSyncRawPointer(cls)))
    }
}

pub fn create_class<S: AsRef<str>>(name: S, class_info: ClassInfo) -> Result<ConfClass> {
    let name_raw = raw_cstr!(name.as_ref());
    let cls = unsafe { SIM_create_class(name_raw, &class_info as *const class_info_t) };

    if cls.is_null() {
        bail!(
            "Failed to register class {}: {}",
            name.as_ref(),
            last_error()
        );
    } else {
        Ok(ConfClass(SendSyncRawPointer(cls)))
    }
}

pub fn register_interface<S: AsRef<str>, T>(cls: ConfClass, name: S) -> Result<i32>
where
    T: Default,
{
    let name_raw = raw_cstr!(name.as_ref());
    let iface_box = Box::<T>::default();
    // Note: This allocates and never frees. This is *required* by SIMICS and it is an error to
    // free this pointer
    let iface_raw = Box::into_raw(iface_box);
    let status = unsafe { SIM_register_interface(cls.0 .0, name_raw, iface_raw as *mut _) };

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

pub fn get_class<S: AsRef<str>>(name: S) -> Result<ConfClass> {
    let name_raw = raw_cstr!(name.as_ref());

    let cls = unsafe { SIM_get_class(name_raw) };

    if cls.is_null() {
        bail!("Failed to get class {}: {}", name.as_ref(), last_error());
    } else {
        Ok(ConfClass(SendSyncRawPointer(cls)))
    }
}

pub fn register_event<S: AsRef<str>>(
    name: S,
    cls: ConfClass,
    callback: unsafe extern "C" fn(*mut conf_object_t, *mut c_void),
) -> Result<EventClass> {
    let name_raw = raw_cstr!(name.as_ref());
    let event = unsafe {
        SIM_register_event(
            name_raw,
            cls.0 .0,
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
        Ok(EventClass(SendSyncRawPointer(event)))
    }
}

pub fn object_clock(obj: &ConfObject) -> Result<ConfObject> {
    let clock = unsafe { SIM_object_clock(obj.0) };

    if clock.is_null() {
        bail!("Unable to get object clock: {}", last_error());
    } else {
        Ok(ConfObject(clock))
    }
}

pub fn event_post_time(clock: ConfObject, event: EventClass, obj: ConfObject, seconds: f64) {
    unsafe { SIM_event_post_time(clock.0, event.0 .0, obj.0, seconds, null_mut()) };
}

pub fn event_find_next_time(clock: ConfObject, event: EventClass, obj: ConfObject) -> f64 {
    unsafe { SIM_event_find_next_time(clock.0, event.0 .0, obj.0, None, null_mut()) }
}

pub fn event_cancel_time(clock: ConfObject, event: EventClass, obj: ConfObject) {
    unsafe { SIM_event_cancel_time(clock.0, event.0 .0, obj.0, None, null_mut()) }
}
