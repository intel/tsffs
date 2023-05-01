use crate::{last_error, ConfClass, ConfObject};
use anyhow::{ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    event_class_t, SIM_event_cancel_time, SIM_event_find_next_time, SIM_event_post_time,
    SIM_register_event,
};
use std::{ffi::c_void, mem::transmute, ptr::null_mut};

pub type EventClass = event_class_t;

pub fn event_post_time<D>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    seconds: f64,
    user_data: Option<D>,
) where
    D: Into<*mut c_void>,
{
    let user_data = match user_data {
        Some(data) => data.into(),
        None => null_mut(),
    };

    unsafe { SIM_event_post_time(clock.into(), event.into(), obj.into(), seconds, user_data) };
}

pub fn event_find_next_time(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
) -> f64 {
    unsafe { SIM_event_find_next_time(clock.into(), event.into(), obj.into(), None, null_mut()) }
}

pub fn event_cancel_time(clock: *mut ConfObject, event: *mut EventClass, obj: *mut ConfObject) {
    unsafe { SIM_event_cancel_time(clock.into(), event.into(), obj.into(), None, null_mut()) }
}

pub fn register_event<S: AsRef<str>>(
    name: S,
    cls: *mut ConfClass,
    callback: extern "C" fn(trigger_obj: *mut ConfObject, user_data: *mut c_void),
) -> Result<*mut EventClass> {
    let event = unsafe {
        SIM_register_event(
            raw_cstr(name.as_ref())?,
            cls.into(),
            0,
            transmute(callback),
            None,
            None,
            None,
            None,
        )
    };

    ensure!(
        !event.is_null(),
        "Unable to register event {}: {}",
        name.as_ref(),
        last_error()
    );

    Ok(event)
}
