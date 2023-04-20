use std::ptr::null_mut;

use simics_api_sys::{
    event_class_t, SIM_event_cancel_time, SIM_event_find_next_time, SIM_event_post_time,
};

use crate::OwnedMutConfObjectPtr;

#[derive(Debug)]
pub struct EventClass {
    class: *mut event_class_t,
}

impl EventClass {
    pub fn new(class: *mut event_class_t) -> Self {
        Self { class }
    }
}

impl From<*mut event_class_t> for EventClass {
    fn from(value: *mut event_class_t) -> Self {
        Self::new(value)
    }
}

impl From<EventClass> for *mut event_class_t {
    fn from(value: EventClass) -> Self {
        value.class
    }
}

pub fn event_post_time(
    clock: OwnedMutConfObjectPtr,
    event: EventClass,
    obj: OwnedMutConfObjectPtr,
    seconds: f64,
) {
    unsafe { SIM_event_post_time(clock.into(), event.into(), obj.into(), seconds, null_mut()) };
}

pub fn event_find_next_time(
    clock: OwnedMutConfObjectPtr,
    event: EventClass,
    obj: OwnedMutConfObjectPtr,
) -> f64 {
    unsafe { SIM_event_find_next_time(clock.into(), event.into(), obj.into(), None, null_mut()) }
}

pub fn event_cancel_time(
    clock: OwnedMutConfObjectPtr,
    event: EventClass,
    obj: OwnedMutConfObjectPtr,
) {
    unsafe { SIM_event_cancel_time(clock.into(), event.into(), obj.into(), None, null_mut()) }
}
