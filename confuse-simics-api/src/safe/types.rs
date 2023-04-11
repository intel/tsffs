use crate::{class_data_t, class_info_t, conf_class_t, conf_object_t, event_class_t, hap_handle_t};
use std::ptr::null_mut;

pub type HapHandle = hap_handle_t;
// pub type ClassInfo = class_info_t;
// pub type ClassData = class_data_t;

pub struct SendSyncRawPointer<T>(pub *mut T);

impl<T> Default for SendSyncRawPointer<T> {
    fn default() -> Self {
        Self(null_mut())
    }
}

/// Wrapper type for event_class_t
pub struct EventClass(pub SendSyncRawPointer<event_class_t>);

/// EventClass is send because event_class_t objects, once created, are never moved or destroyed
unsafe impl Send for EventClass {}
/// EventClass is send because conf_class_t objects, once created, are never moved or destroyed
unsafe impl Sync for EventClass {}

/// Wrapper type for conf_class_t
pub struct ConfClass(pub SendSyncRawPointer<conf_class_t>);

/// EventClass is send because conf_class_t objects, once created, are never moved or destroyed
unsafe impl Send for ConfClass {}
/// EventClass is send because conf_class_t objects, once created, are never moved or destroyed
unsafe impl Sync for ConfClass {}

/// ConfObject is *not* Send + Sync because it can be mutated at any time by SIMICS, and needs to
/// be threadsafe
pub struct ConfObject(pub *mut conf_object_t);

impl Default for ConfObject {
    fn default() -> Self {
        Self(null_mut())
    }
}
