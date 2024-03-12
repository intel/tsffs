// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! SIMICS APIs for event management

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    simics_exception,
    sys::{
        cycles_t, event_class_t, SIM_event_cancel_step, SIM_event_cancel_time,
        SIM_event_find_next_cycle, SIM_event_find_next_step, SIM_event_find_next_time,
        SIM_event_post_cycle, SIM_event_post_step, SIM_event_post_time, SIM_register_event,
    },
    ConfClass, ConfObject, Error, PcStep, Result,
};
use raw_cstr::raw_cstr;
use std::{ffi::c_void, ptr::null_mut};
use typed_builder::TypedBuilder;

/// Flags for an event
pub use crate::api::sys::event_class_flag_t as EventClassFlag;
/// Alias for `event_class_t`
pub type EventClass = event_class_t;
/// Alias for `cycles_t`
pub type Cycles = cycles_t;

/// A callable closure which receives a pointer to the triggering object when an event
/// occurs
pub type EventCallbackClosure = Box<dyn FnMut(*mut ConfObject)>;
/// A callback which is called to determine whether action should be taken on an event
pub type EventFilterClosure = Box<dyn Fn(*mut c_void) -> i32>;

extern "C" fn event_callback_handler(obj: *mut ConfObject, cb: *mut c_void) {
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut EventCallbackClosure) });
    closure(obj)
}

extern "C" fn event_destroy_handler(_: *mut ConfObject, cb: *mut c_void) {
    let _ = unsafe { Box::from_raw(cb as *mut EventCallbackClosure) };
    // NOTE: _ dropped
}

extern "C" fn event_filter_handler(data: *mut c_void, callback: *mut c_void) -> i32 {
    let closure = Box::leak(unsafe { Box::from_raw(callback as *mut EventFilterClosure) });
    closure(data)
}
#[derive(TypedBuilder, Debug, Clone)]
/// Simplified event management mechanism using dynamic dispatch to circumvent complex trait
/// requirements due to difference in callback specification and post time when using the
/// canonical SIMICS APIs
pub struct Event {
    #[builder(setter(into))]
    #[allow(unused)]
    /// The name of the event. This should identify the event uniquely.
    name: String,
    #[allow(unused)]
    /// The class the event will be posted for. This should be the class that is *posting* the
    /// events, not the class the events are posting on. For example, if you are implementing
    /// a module, `cls` should be the main class in your module.
    cls: *mut ConfClass,
    #[builder(default = EventClassFlag(0), setter(into))]
    #[allow(unused)]
    /// Flags of the event. Should either be 0 (the default), which indicates the event should not
    /// be saved, or [`simics::api::sys::Sim_EC_Notsaved`] which indicates this may not be saved
    /// in the configuration.
    flags: EventClassFlag,
    #[builder(default = register_event(&name, cls, flags).expect("Failed to register event"))]
    event_class: *mut EventClass,
}

impl Event {
    /// Return the class an event is posted for
    pub fn cls(&self) -> *mut ConfClass {
        self.cls
    }

    /// Return the class of this event
    pub fn event_class(&self) -> *mut EventClass {
        self.event_class
    }

    /// Register a new event to be posted for objects of class cl, and
    /// returns the event class to be used in other calls.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the event to register for
    /// * `cls` - The class events will be posted for objects of
    /// * `flags` - Flags describing the events
    ///
    /// # Context
    ///
    /// Global Context
    pub fn register<S>(name: S, cls: *mut ConfClass, flags: EventClassFlag) -> Result<Self>
    where
        S: AsRef<str>,
    {
        Ok(Self {
            name: name.as_ref().to_string(),
            cls,
            flags,
            event_class: register_event(name, cls, flags)?,
        })
    }

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
    /// * `obj` - The object the event is being posted on
    /// * `clock` - The clock whose time this event is being posted for
    /// * `seconds` - The number of seconds until this event expires
    /// * `callback` - Callback to run for this event
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn post_time<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        seconds: f64,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(*mut ConfObject) + 'static,
    {
        event_post_time(clock, self.event_class, obj, seconds, callback)
    }

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
    /// * `obj` - The object the event was posted on
    /// * `clock` - The clock the event to cancel was posted on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn cancel_time(&self, obj: *mut ConfObject, clock: *mut ConfObject) -> Result<()> {
        event_cancel_time::<Box<dyn Fn(*mut c_void) -> i32>>(clock, self.event_class, obj, None)
    }

    #[deprecated = "Filter function will not be freed. This will lead to memory leaks."]
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
    /// * `obj` - The object the event was posted on
    /// * `clock` - The clock the event to cancel was posted on
    /// * `filter` - The filter function. Note that there is a limitation currently
    ///   which does not allow this filter function to be freed once it is boxed, which can
    ///   lead to memory leaks, so this method is deprecated as a warning.
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn cancel_time_filter<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        filter: Option<F>,
    ) -> Result<()>
    where
        F: Fn(*mut c_void) -> i32 + 'static,
    {
        event_cancel_time(clock, self.event_class, obj, filter)
    }

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
    /// * `obj` - The object the event is being posted on
    /// * `clock` - The clock whose time this event is being posted for
    /// * `steps` - The number of seconds until this event expires
    /// * `callback` - Callback to run for this event
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn post_step<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        steps: PcStep,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(*mut ConfObject) + 'static,
    {
        event_post_step(clock, self.event_class, obj, steps, callback)
    }

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
    /// * `obj` - The object the event was posted on
    /// * `clock` - The clock the event to cancel was posted on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn cancel_step(&self, obj: *mut ConfObject, clock: *mut ConfObject) -> Result<()> {
        event_cancel_step::<Box<dyn Fn(*mut c_void) -> i32>>(clock, self.event_class, obj, None)
    }

    #[deprecated = "Filter function will not be freed. This will lead to memory leaks."]
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
    /// * `obj` - The object the event was posted on
    /// * `clock` - The clock the event to cancel was posted on
    /// * `filter` - The filter function. Note that there is a limitation currently
    ///   which does not allow this filter function to be freed once it is boxed, which can
    ///   lead to memory leaks, so this method is deprecated as a warning.
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn cancel_step_filter<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        filter: Option<F>,
    ) -> Result<()>
    where
        F: Fn(*mut c_void) -> i32 + 'static,
    {
        event_cancel_step(clock, self.event_class, obj, filter)
    }

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
    /// * `obj` - The object the event is being posted on
    /// * `clock` - The clock whose time this event is being posted for
    /// * `cycles` - The number of seconds until this event expires
    /// * `callback` - Callback to run for this event
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn post_cycle<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        cycles: Cycles,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(*mut ConfObject) + 'static,
    {
        event_post_cycle(clock, self.event_class, obj, cycles, callback)
    }

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
    /// * `obj` - The object posted on
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_time(&self, obj: *mut ConfObject, clock: *mut ConfObject) -> Result<f64> {
        event_find_next_time::<Box<dyn Fn(*mut c_void) -> i32>>(clock, self.event_class, obj, None)
    }

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
    /// * `obj` - The object posted on
    /// * `filter` - A function to filter objects by returning true or false
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_time_filter<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        filter: F,
    ) -> Result<f64>
    where
        F: Fn(*mut c_void) -> i32 + 'static,
    {
        event_find_next_time(clock, self.event_class, obj, Some(filter))
    }

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
    /// * `obj` - The object posted on
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_cycle(&self, obj: *mut ConfObject, clock: *mut ConfObject) -> Result<Cycles> {
        event_find_next_cycle::<Box<dyn Fn(*mut c_void) -> i32>>(clock, self.event_class, obj, None)
    }

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
    /// * `obj` - The object posted on
    /// * `filter` - A function to filter objects by returning true or false
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_cycle_filter<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        filter: F,
    ) -> Result<Cycles>
    where
        F: Fn(*mut c_void) -> i32 + 'static,
    {
        event_find_next_cycle(clock, self.event_class, obj, Some(filter))
    }

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
    /// * `obj` - The object posted on
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_step(&self, obj: *mut ConfObject, clock: *mut ConfObject) -> Result<PcStep> {
        event_find_next_step::<Box<dyn Fn(*mut c_void) -> i32>>(clock, self.event_class, obj, None)
    }

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
    /// * `obj` - The object posted on
    /// * `filter` - A function to filter objects by returning true or false
    ///
    /// # Return Value
    ///
    /// If found, the cycle number the event will next trigger on
    ///
    /// # Context
    ///
    /// Cell Context
    pub fn find_next_step_filter<F>(
        &self,
        obj: *mut ConfObject,
        clock: *mut ConfObject,
        filter: F,
    ) -> Result<PcStep>
    where
        F: Fn(*mut c_void) -> i32 + 'static,
    {
        event_find_next_step(clock, self.event_class, obj, Some(filter))
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
///
/// # Context
///
/// Global Context
fn register_event<S>(name: S, cls: *mut ConfClass, flags: EventClassFlag) -> Result<*mut EventClass>
where
    S: AsRef<str>,
{
    let event = unsafe {
        SIM_register_event(
            raw_cstr(name.as_ref())?,
            cls,
            flags,
            Some(event_callback_handler),
            // NOTE: Destroy callback frees the callback itself
            Some(event_destroy_handler),
            None,
            None,
            None,
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
///
/// # Context
///
/// Cell Context
pub fn event_post_time<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    seconds: f64,
    callback: F,
) where
    F: FnMut(*mut ConfObject) + 'static,
{
    let callbacks: EventCallbackClosure = Box::new(callback);
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
///
/// # Context
///
/// Cell Context
pub fn event_post_cycle<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    cycles: Cycles,
    callback: F,
) where
    F: FnMut(*mut ConfObject) + 'static,
{
    let callbacks: EventCallbackClosure = Box::new(callback);
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
/// * `steps` - The number of seconds until this event expires
/// * `callback` - Callback to run for this event
///
/// # Context
///
/// Cell Context
pub fn event_post_step<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    steps: PcStep,
    callback: F,
) where
    F: FnMut(*mut ConfObject) + 'static,
{
    let callbacks: EventCallbackClosure = Box::new(callback);
    let callbacks_box = Box::new(callbacks);
    let callbacks_raw = Box::into_raw(callbacks_box);

    unsafe { SIM_event_post_step(clock, event, obj, steps, callbacks_raw as *mut c_void) };
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
///
/// # Context
///
/// Cell Context
pub fn event_cancel_time<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter: EventFilterClosure = Box::new(filter);
        let filter_box = Box::new(filter);
        (Some(event_filter_handler as _), Box::into_raw(filter_box))
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
///
/// # Context
///
/// Cell Context
pub fn event_cancel_step<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter: EventFilterClosure = Box::new(filter);
        let filter_box = Box::new(filter);
        (Some(event_filter_handler as _), Box::into_raw(filter_box))
    } else {
        (None, null_mut())
    };
    unsafe { SIM_event_cancel_step(clock, event, obj, callback, callback_data as *mut c_void) }
}

// NOTE: There is no such function event_cancel_cycle

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
///
/// # Context
///
/// Cell Context
pub fn event_find_next_cycle<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) -> Result<Cycles>
where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter: EventFilterClosure = Box::new(filter);
        let filter_box = Box::new(filter);
        (Some(event_filter_handler as _), Box::into_raw(filter_box))
    } else {
        (None, null_mut())
    };

    let time =
        unsafe { SIM_event_find_next_cycle(clock, event, obj, callback, callback_data as _) };

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
///
/// # Context
///
/// Cell Context
pub fn event_find_next_time<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) -> Result<f64>
where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter: EventFilterClosure = Box::new(filter);
        let filter_box = Box::new(filter);
        (Some(event_filter_handler as _), Box::into_raw(filter_box))
    } else {
        (None, null_mut())
    };

    let time = unsafe { SIM_event_find_next_time(clock, event, obj, callback, callback_data as _) };

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
///
/// # Context
///
/// Cell Context
pub fn event_find_next_step<F>(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
    filter: Option<F>,
) -> Result<PcStep>
where
    F: Fn(*mut c_void) -> i32 + 'static,
{
    let (callback, callback_data) = if let Some(filter) = filter {
        let filter: EventFilterClosure = Box::new(filter);
        let filter_box = Box::new(filter);
        (Some(event_filter_handler as _), Box::into_raw(filter_box))
    } else {
        (None, null_mut())
    };

    let time = unsafe { SIM_event_find_next_step(clock, event, obj, callback, callback_data as _) };

    if time == -1 {
        Err(Error::NoEventFound)
    } else {
        Ok(time)
    }
}
