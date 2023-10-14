// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    api::{
        sys::{
            cycles_t, event_class_flag_t, event_class_t, SIM_event_cancel_time,
            SIM_event_find_next_cycle, SIM_event_find_next_time, SIM_event_post_cycle,
            SIM_event_post_time, SIM_register_event,
        },
        ConfClass, ConfObject,
    },
    Error, Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::{ffi::c_void, ptr::null_mut};
use typed_builder::TypedBuilder;

pub type EventClass = event_class_t;
pub type Cycles = cycles_t;

#[derive(TypedBuilder)]
pub struct EventCallbacks<C, D, DE>
where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    callback: C,
    #[builder(default)]
    destroy: Option<D>,
    #[builder(default)]
    describe: Option<DE>,
}

extern "C" fn event_callback_handler<C, D, DE>(obj: *mut ConfObject, cb: *mut c_void)
where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let mut closures: Box<Box<EventCallbacks<C, D, DE>>> =
        unsafe { Box::from_raw(cb as *mut Box<EventCallbacks<C, D, DE>>) };
    (closures.callback)(obj)
}

extern "C" fn event_destroy_handler<C, D, DE>(obj: *mut ConfObject, cb: *mut c_void)
where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let mut closures: Box<Box<EventCallbacks<C, D, DE>>> =
        unsafe { Box::from_raw(cb as *mut Box<EventCallbacks<C, D, DE>>) };

    if let Some(ref mut destroy) = closures.destroy {
        destroy(obj)
    }
}

extern "C" fn event_describe_handler<C, D, DE>(obj: *mut ConfObject, cb: *mut c_void) -> *mut i8
where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let mut closures: Box<Box<EventCallbacks<C, D, DE>>> =
        unsafe { Box::from_raw(cb as *mut Box<EventCallbacks<C, D, DE>>) };
    if let Some(ref mut describe) = closures.describe {
        describe(obj)
    } else {
        null_mut()
    }
}

#[simics_exception]
/// Register an event with a callback. If `flags` is `&[EventFlags::NotSaved]`, `cls` may be
/// null.
pub fn register_event<S, C, D, DE>(
    name: S,
    cls: *mut ConfClass,
    flags: &[EventFlags],
) -> Result<*mut EventClass>
where
    S: AsRef<str>,
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let mut event_flags = EventFlags::Sim_EC_No_Flags;

    for flag in flags {
        event_flags |= *flag;
    }

    let event = unsafe {
        SIM_register_event(
            raw_cstr(name.as_ref())?,
            cls,
            event_flags,
            Some(event_callback_handler::<C, D, DE>),
            Some(event_destroy_handler::<C, D, DE>),
            // TODO: Serialize the callback userdata?
            None,
            None,
            Some(event_describe_handler::<C, D, DE>),
        )
    };

    Ok(event)
}

#[simics_exception]
/// Post an event to be triggered after a particular amount of time
pub fn event_post_time<C, D, DE>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    seconds: f64,
    callbacks: EventCallbacks<C, D, DE>,
) where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let callbacks = Box::new(callbacks);
    let callbacks_box = Box::new(callbacks);
    let callbacks_raw = Box::into_raw(callbacks_box);

    unsafe { SIM_event_post_time(clock, event, obj, seconds, callbacks_raw as *mut c_void) };
}

#[simics_exception]
/// Post an event to be triggered after a particular amount of time
pub fn event_post_cycle<C, D, DE>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    cycles: Cycles,
    callbacks: EventCallbacks<C, D, DE>,
) where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let callbacks = Box::new(callbacks);
    let callbacks_box = Box::new(callbacks);
    let callbacks_raw = Box::into_raw(callbacks_box);

    unsafe { SIM_event_post_cycle(clock, event, obj, cycles, callbacks_raw as *mut c_void) };
}

#[simics_exception]
/// Cancel an event that was posted with [`event_post_time`]
pub fn event_cancel_time(clock: *mut ConfObject, event: *mut EventClass, obj: *mut ConfObject) {
    unsafe { SIM_event_cancel_time(clock, event, obj, None, null_mut()) }
}

#[simics_exception]
/// Find the next time an event is scheduled to be triggered
pub fn event_find_next_cycle(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
) -> Result<Cycles> {
    let time = unsafe { SIM_event_find_next_cycle(clock, event, obj, None, null_mut()) };

    if time == -1 {
        Err(Error::NoEventFound)
    } else {
        Ok(time)
    }
}

#[simics_exception]
/// Find the next time an event is scheduled to be triggered
pub fn event_find_next_time(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
) -> Result<f64> {
    let time = unsafe { SIM_event_find_next_time(clock, event, obj, None, null_mut()) };

    if time == -1.0 {
        Err(Error::NoEventFound)
    } else {
        Ok(time)
    }
}

// TODO: Add step API

/// Flags for an event
pub type EventFlags = event_class_flag_t;
