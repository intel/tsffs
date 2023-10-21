// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS APIs for event management

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    api::{
        sys::{
            cycles_t, event_class_flag_t, event_class_t, SIM_event_cancel_step,
            SIM_event_cancel_time, SIM_event_find_next_cycle, SIM_event_find_next_step,
            SIM_event_find_next_time, SIM_event_post_cycle, SIM_event_post_step,
            SIM_event_post_time, SIM_register_event,
        },
        ConfClass, ConfObject, PcStep,
    },
    Error, Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::{ffi::c_void, ptr::null_mut};
use typed_builder::TypedBuilder;

/// Flags for an event
pub type EventFlags = event_class_flag_t;
pub type EventClass = event_class_t;
pub type Cycles = cycles_t;

#[derive(TypedBuilder)]
/// Callbacks to run on an event's expiry.
///
/// * `callback` is always run when the event expires and must be provided
/// * `destroy` is run when the event is removed from the queue without being called
/// (mainly intended for freeing event data) and is optional.
/// * `describe` returns an allocated string describing the event for the
/// `print-event-queue` command and is optional.
///
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
/// Registers events identified by name and to be posted for objects of class cl, and
/// returns the event class to be used in other calls. Callbacks are provided when
/// posting events individually, this function registers a FFI callback compatible with the
/// closure parameters for posting events.
///
/// The supplied methods are:
///
/// * `callback` - Called when the event expires.
/// * `destroy` - Called when the event is removed from the queue without being called.
/// The method is not allowed to use any event API calls; it is mainly intended for
/// freeing event data. May be null.
/// * `describe` - Called to generate a human-readable description of the event to be
/// used in the print-event-queue command.
///
/// Null function pointers correspond to the value None when invoked from Python.
///
/// The flags is typically either zero or [`EventFlags::Sim_EC_Notsaved`], where
/// [`EventFlags::Sim_EC_Notsaved`] indicates that the event should not be saved as part
/// of the configuration.
///
/// # Arguments
///
/// * `name` - The name of the event to register for
/// * `cls` - The class events will be posted for objects of
/// * `flags` - Flags describing the events
pub fn register_event<S, C, D, DE>(
    name: S,
    cls: *mut ConfClass,
    flags: EventFlags,
) -> Result<*mut EventClass>
where
    S: AsRef<str>,
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let event = unsafe {
        SIM_register_event(
            raw_cstr(name.as_ref())?,
            cls,
            flags,
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
/// An event of evclass for object obj is posted on clock to occur at a given point in
/// the future. The user_data will be associated with the event.  The clock is the
/// object that should be used for keeping track of time for the event.  It can be a
/// processor or an instance of the clock class.
///
/// If a configuration class was specified when evclass was registered, then obj must be
/// an instance of that class.
///
/// The expiration point can be specified in seconds, cycles or steps by using the
/// appropriate call, and these values are relative to the current state. Events that
/// need to run synchronized (Sim_EC_Machine_Sync) can only be posted in seconds or
/// cycles, not steps, since synchronization can only be perform in virtual time.
///
/// Note: Events posted with SIM_event_post_cycle are posted at a certain point in time
/// based on the clock's current frequency, not at a certain clock cycle. The difference
/// is significant if the frequency of the clock object can change dynamically.
///
/// # Arguments
///
/// * `clock` - The clock whose time this event is being posted for
/// * `event` - The event class registered with [`register_event`] being posted
/// * `obj` - The object the event is being posted on
/// * `seconds` - The number of seconds until this event expires
/// * `callbacks` - Callbacks to run for this event
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
/// An event of evclass for object obj is posted on clock to occur at a given point in
/// the future. The user_data will be associated with the event.  The clock is the
/// object that should be used for keeping track of time for the event.  It can be a
/// processor or an instance of the clock class.
///
/// If a configuration class was specified when evclass was registered, then obj must be
/// an instance of that class.
///
/// The expiration point can be specified in seconds, cycles or steps by using the
/// appropriate call, and these values are relative to the current state. Events that
/// need to run synchronized (Sim_EC_Machine_Sync) can only be posted in seconds or
/// cycles, not steps, since synchronization can only be perform in virtual time.
///
/// Note: Events posted with SIM_event_post_cycle are posted at a certain point in time
/// based on the clock's current frequency, not at a certain clock cycle. The difference
/// is significant if the frequency of the clock object can change dynamically.
///
/// # Arguments
///
/// * `clock` - The clock whose time this event is being posted for
/// * `event` - The event class registered with [`register_event`] being posted
/// * `obj` - The object the event is being posted on
/// * `cycles` - The number of seconds until this event expires
/// * `callbacks` - Callbacks to run for this event
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
/// An event of evclass for object obj is posted on clock to occur at a given point in
/// the future. The user_data will be associated with the event.  The clock is the
/// object that should be used for keeping track of time for the event.  It can be a
/// processor or an instance of the clock class.
///
/// If a configuration class was specified when evclass was registered, then obj must be
/// an instance of that class.
///
/// The expiration point can be specified in seconds, cycles or steps by using the
/// appropriate call, and these values are relative to the current state. Events that
/// need to run synchronized (Sim_EC_Machine_Sync) can only be posted in seconds or
/// cycles, not steps, since synchronization can only be perform in virtual time.
///
/// Note: Events posted with SIM_event_post_cycle are posted at a certain point in time
/// based on the clock's current frequency, not at a certain clock cycle. The difference
/// is significant if the frequency of the clock object can change dynamically.
///
/// # Arguments
///
/// * `clock` - The clock whose time this event is being posted for
/// * `event` - The event class registered with [`register_event`] being posted
/// * `obj` - The object the event is being posted on
/// * `cycles` - The number of seconds until this event expires
/// * `callbacks` - Callbacks to run for this event
pub fn event_post_step<C, D, DE>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    steps: PcStep,
    callbacks: EventCallbacks<C, D, DE>,
) where
    C: FnMut(*mut ConfObject) + 'static,
    D: FnMut(*mut ConfObject) + 'static,
    DE: FnMut(*mut ConfObject) -> *mut i8 + 'static,
{
    let callbacks = Box::new(callbacks);
    let callbacks_box = Box::new(callbacks);
    let callbacks_raw = Box::into_raw(callbacks_box);

    unsafe { SIM_event_post_step(clock, event, obj, steps, callbacks_raw as *mut c_void) };
}

extern "C" fn handle_cancel_filter<F>(data: *mut c_void, callback: *mut c_void) -> i32
where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(callback as *mut Box<F>) };
    closure(data)
}

#[simics_exception]
/// All unexpired evclass events posted for obj on clock for which pred returns nonzero
/// will be cancelled and their destructor methods (if any) called. pred will be called
/// with the data associated with the event and the supplied match_data. If pred is null
/// (None in Python), all evclass events for obj on clock will be cancelled.
///
/// There are separate calls for events posted at a point in time (cycle or seconds) and
/// on a specific step.
///
/// # Arguments
///
/// * `clock` - The clock the event to cancel was posted on
/// * `event` - The event to cancel
/// * `obj` - The object the event was posted on
pub fn event_cancel_time<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter = Box::new(filter);
        let filter_box = Box::new(filter);
        (
            Some(handle_cancel_filter::<F> as _),
            Box::into_raw(filter_box),
        )
    } else {
        (None, null_mut())
    };
    unsafe { SIM_event_cancel_time(clock, event, obj, callback, callback_data as *mut c_void) }
}

#[simics_exception]
/// All unexpired evclass events posted for obj on clock for which pred returns nonzero
/// will be cancelled and their destructor methods (if any) called. pred will be called
/// with the data associated with the event and the supplied match_data. If pred is null
/// (None in Python), all evclass events for obj on clock will be cancelled.
///
/// There are separate calls for events posted at a point in time (cycle or seconds) and
/// on a specific step.
///
/// # Arguments
///
/// * `clock` - The clock the event to cancel was posted on
/// * `event` - The event to cancel
/// * `obj` - The object the event was posted on
/// * `filter` - An optional closure to filter events that returns nonzero if an event
/// should be canceled or zero if it should not be canceled. If not provided, all events for class
/// `event` will be canceled.
pub fn event_cancel_step<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter = Box::new(filter);
        let filter_box = Box::new(filter);
        (
            Some(handle_cancel_filter::<F> as _),
            Box::into_raw(filter_box),
        )
    } else {
        (None, null_mut())
    };
    unsafe { SIM_event_cancel_step(clock, event, obj, callback, callback_data as *mut c_void) }
}

#[simics_exception]
/// Return the number of cycles/seconds/steps to the first event of evclass of obj posted
/// on clock for which pred is true, or −1 if no event matched. pred will be called with
/// the data associated with the event and the supplied match_data. If pred is null (None
/// in Python), the first evclass event for obj on clock will be used.
///
/// There are separate calls of events posted at a point in time (cycle or seconds) and
/// on a specific step. Note that the return value of SIM_event_find_next_cycle is only a
/// preliminary estimate; the number of remaining cycles will change if the clock's
/// frequency changes dynamically. To handle dynamically changing clock frequencies
/// correctly, subscribe to the frequency changes via the clock's simple_dispatcher
/// interface.
///
/// # Arguments
///
/// * `clock` - The clock for the posted event
/// * `event` - The posted event class
/// * `obj` - The object posted on
///
/// # Return Value
///
/// If found, the cycle number the event will next trigger on
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
/// Return the number of cycles/seconds/steps to the first event of evclass of obj posted
/// on clock for which pred is true, or −1 if no event matched. pred will be called with
/// the data associated with the event and the supplied match_data. If pred is null (None
/// in Python), the first evclass event for obj on clock will be used.
///
/// There are separate calls of events posted at a point in time (cycle or seconds) and
/// on a specific step. Note that the return value of SIM_event_find_next_cycle is only a
/// preliminary estimate; the number of remaining cycles will change if the clock's
/// frequency changes dynamically. To handle dynamically changing clock frequencies
/// correctly, subscribe to the frequency changes via the clock's simple_dispatcher
/// interface.
///
/// # Arguments
///
/// * `clock` - The clock for the posted event
/// * `event` - The posted event class
/// * `obj` - The object posted on
///
/// # Return Value
///
/// If found, the cycle number the event will next trigger on
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

#[simics_exception]
/// Return the number of cycles/seconds/steps to the first event of evclass of obj posted
/// on clock for which pred is true, or −1 if no event matched. pred will be called with
/// the data associated with the event and the supplied match_data. If pred is null (None
/// in Python), the first evclass event for obj on clock will be used.
///
/// There are separate calls of events posted at a point in time (cycle or seconds) and
/// on a specific step. Note that the return value of SIM_event_find_next_cycle is only a
/// preliminary estimate; the number of remaining cycles will change if the clock's
/// frequency changes dynamically. To handle dynamically changing clock frequencies
/// correctly, subscribe to the frequency changes via the clock's simple_dispatcher
/// interface.
///
/// # Arguments
///
/// * `clock` - The clock for the posted event
/// * `event` - The posted event class
/// * `obj` - The object posted on
///
/// # Return Value
///
/// If found, the cycle number the event will next trigger on
pub fn event_find_next_step(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
) -> Result<PcStep> {
    let time = unsafe { SIM_event_find_next_step(clock, event, obj, None, null_mut()) };

    if time == -1 {
        Err(Error::NoEventFound)
    } else {
        Ok(time)
    }
}
