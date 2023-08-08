// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{last_error, ConfClass, ConfObject};
use anyhow::{bail, ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    event_class_flag_t_Sim_EC_Machine_Sync, event_class_flag_t_Sim_EC_No_Flags,
    event_class_flag_t_Sim_EC_No_Serialize, event_class_flag_t_Sim_EC_Notsaved,
    event_class_flag_t_Sim_EC_Slot_Early, event_class_flag_t_Sim_EC_Slot_Late, event_class_t,
    SIM_event_cancel_time, SIM_event_find_next_time, SIM_event_post_time, SIM_register_event,
};
use std::{ffi::c_void, mem::transmute, ptr::null_mut};

pub type EventClass = event_class_t;

/// Post an event to be triggered after a particular amount of time
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

/// Find the next time an event is scheduled to be triggered
pub fn event_find_next_time(
    clock: *mut ConfObject,
    event: *mut EventClass,
    obj: *mut ConfObject,
) -> Result<f64> {
    let time = unsafe {
        SIM_event_find_next_time(clock.into(), event.into(), obj.into(), None, null_mut())
    };

    if time == -1.0 {
        bail!("No matching event was found");
    } else {
        Ok(time)
    }
}

/// Cancel an event that was posted with [`event_post_time`]
pub fn event_cancel_time(clock: *mut ConfObject, event: *mut EventClass, obj: *mut ConfObject) {
    unsafe { SIM_event_cancel_time(clock.into(), event.into(), obj.into(), None, null_mut()) }
}

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
/// Flags for an event
pub enum EventFlags {
    None = event_class_flag_t_Sim_EC_No_Flags,
    NotSaved = event_class_flag_t_Sim_EC_Notsaved,
    MachineSync = event_class_flag_t_Sim_EC_Machine_Sync,
    NoSerialize = event_class_flag_t_Sim_EC_No_Serialize,
    SlotEarly = event_class_flag_t_Sim_EC_Slot_Early,
    SlotLate = event_class_flag_t_Sim_EC_Slot_Late,
}

/// Register an event with a callback. If `flags` is `&[EventFlags::NotSaved]`, `cls` may be
/// null.
pub fn register_event<S>(
    name: S,
    cls: *mut ConfClass,
    callback: extern "C" fn(trigger_obj: *mut ConfObject, user_data: *mut c_void),
    flags: &[EventFlags],
) -> Result<*mut EventClass>
where
    S: AsRef<str>,
{
    let mut event_flags = EventFlags::None as u32;
    for flag in flags {
        event_flags |= *flag as u32;
    }

    let event = unsafe {
        SIM_register_event(
            raw_cstr(name.as_ref())?,
            cls.into(),
            event_flags,
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
