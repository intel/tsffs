// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Callbacks from the simulator to user code

#[cfg(not(windows))]
use crate::sys::SIM_notify_on_descriptor;
use crate::{
    simics_exception,
    sys::{
        notify_mode_t, socket_t, SIM_cancel_realtime_event, SIM_notify_on_socket,
        SIM_process_pending_work, SIM_process_work, SIM_realtime_event, SIM_register_work,
        SIM_run_alone, SIM_run_in_thread, SIM_thread_safe_callback,
    },
    Result,
};
use raw_cstr::raw_cstr;
use std::{ffi::c_void, ptr::null_mut};

/// Alias for `notify_mode_t`
pub type NotifyMode = notify_mode_t;
/// Alias for `socket_t`
pub type Socket = socket_t;

extern "C" fn handle_notify_on_descriptor_callback<F>(cb: *mut c_void)
where
    F: Fn() + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

#[cfg(not(windows))]
#[simics_exception]
/// Set a callback whenever a specific IO event occurs on the host on a given file descriptor.
/// If `callback` is `None`, the callback is removed.
///
/// # Context
///
/// Cell Context
/// Callback: Threaded Context if `run_in_thread`, Global Context otherwise
pub fn notify_on_descriptor<C>(fd: i32, mode: NotifyMode, run_in_thread: bool, callback: Option<C>)
where
    C: Fn() + 'static,
{
    let callback = if callback.is_none() {
        None
    } else {
        Some(handle_notify_on_descriptor_callback::<C>)
    };
    unsafe {
        SIM_notify_on_descriptor(
            fd,
            mode,
            run_in_thread as i32,
            callback
                .is_some()
                .then_some(handle_notify_on_descriptor_callback::<C>),
            callback
                .map(|c| {
                    let callback = Box::new(c);
                    let callback_box = Box::new(callback);
                    Box::into_raw(callback_box) as *mut c_void
                })
                .unwrap_or(null_mut()),
        )
    }
}

#[simics_exception]
/// Set a callback whenever a specific IO event occurs on the host on a given file descriptor.
/// If `callback` is `None`, the callback is removed.
///
/// # Context
///
/// Cell Context
/// Callback: Threaded Context if `run_in_thread`, Global Context otherwise
pub fn notify_on_socket<C>(sock: Socket, mode: NotifyMode, run_in_thread: bool, callback: Option<C>)
where
    C: Fn() + 'static,
{
    let callback = if callback.is_none() {
        None
    } else {
        Some(handle_notify_on_descriptor_callback::<C>)
    };
    unsafe {
        SIM_notify_on_socket(
            sock,
            mode,
            run_in_thread as i32,
            callback
                .is_some()
                .then_some(handle_notify_on_descriptor_callback::<C>),
            callback
                .map(|c| {
                    let callback = Box::new(c);
                    let callback_box = Box::new(callback);
                    Box::into_raw(callback_box) as *mut c_void
                })
                .unwrap_or(null_mut()),
        )
    }
}

extern "C" fn handle_work_callback<F>(cb: *mut c_void)
where
    F: FnOnce() + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

#[simics_exception]
/// Register a callback to be run in the simics thread
///
/// # Context
///
/// Cell Context
/// Callback: Global Context
pub fn register_work<F>(work: F)
where
    F: FnOnce() + 'static,
{
    let work = Box::new(work);
    let work_box = Box::new(work);
    unsafe {
        SIM_register_work(
            Some(handle_work_callback::<F>),
            Box::into_raw(work_box) as *mut c_void,
        )
    };
}

extern "C" fn handle_process_work_callback<F>(cb: *mut c_void) -> i32
where
    F: FnOnce() -> i32 + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

#[simics_exception]
/// process_work and process_pending_work processes work posted by
/// thread_safe_callback and realtime_event. These process work functions are
/// typically called when embedding Simics in another application to allow periodic and
/// asynchronous Simics work to run while the simulation is not advancing.
/// process_pending_work runs all work that has been queued up since the last call
/// and returns immediately after.
///
/// process_work is similar but waits for new work to arrive. Each time some work
/// has been processed, the supplied done callback is called with done_data as its only
/// argument. A return value of 1 tells process_work to stop processing work and
/// return control to the caller again while 0 tells it to continue.
///
/// The done predicate is only evaluated between callbacks that are run in Global
/// Context, that is, not registered with the run_in_thread parameter set).
///
/// The process work functions return -1 if the user has pressed the interrupt key
/// before or while they were running, provided that the simulator core was initialized
/// to catch signals. Otherwise the return value is 0.
///
/// # Context
///
/// Global Context
pub fn process_work<F>(work: F)
where
    F: FnOnce() -> i32 + 'static,
{
    let work = Box::new(work);
    let work_box = Box::new(work);
    unsafe {
        SIM_process_work(
            Some(handle_process_work_callback::<F>),
            Box::into_raw(work_box) as *mut c_void,
        )
    };
}

#[simics_exception]
/// process_work and process_pending_work processes work posted by
/// thread_safe_callback and realtime_event. These process work functions are
/// typically called when embedding Simics in another application to allow periodic and
/// asynchronous Simics work to run while the simulation is not advancing.
/// process_pending_work runs all work that has been queued up since the last call
/// and returns immediately after.
///
/// process_work is similar but waits for new work to arrive. Each time some work
/// has been processed, the supplied done callback is called with done_data as its only
/// argument. A return value of 1 tells process_work to stop processing work and
/// return control to the caller again while 0 tells it to continue.
///
/// The done predicate is only evaluated between callbacks that are run in Global
/// Context, that is, not registered with the run_in_thread parameter set).
///
/// The process work functions return -1 if the user has pressed the interrupt key
/// before or while they were running, provided that the simulator core was initialized
/// to catch signals. Otherwise the return value is 0.
///
/// # Context
///
/// Global Context
pub fn process_pending_work() {
    unsafe { SIM_process_pending_work() };
}

extern "C" fn handle_realtime_event_callback<F>(cb: *mut c_void)
where
    F: FnOnce() + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

#[simics_exception]
/// # Context
///
/// Cell Context
pub fn realtime_event<F, S>(
    delay_ms: u32,
    callback: F,
    run_in_thread: bool,
    description: S,
) -> Result<i64>
where
    S: AsRef<str>,
    F: FnOnce() + 'static,
{
    let callback = Box::new(callback);
    let callback_box = Box::new(callback);
    Ok(unsafe {
        SIM_realtime_event(
            delay_ms,
            Some(handle_realtime_event_callback::<F>),
            Box::into_raw(callback_box) as *mut c_void,
            run_in_thread as i32,
            raw_cstr(description)?,
        )
    })
}

#[simics_exception]
/// # Context
///
/// Cell Context
pub fn cancel_realtime_event(id: i64) {
    unsafe { SIM_cancel_realtime_event(id) };
}

// NOTE: No binding for SIM_register_work, it is not consistent and basically deprecated

extern "C" fn handle_run_alone_callback<F>(cb: *mut c_void)
where
    F: FnOnce() -> Result<()> + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure().expect("Failed while running run_alone callback");
}

#[simics_exception]
/// Schedule a callback to be run with all execution stopped (global context)
///
/// If posted while an instruction is being executed, the callback will be invoked after the
/// current instruction has completed.
///
/// will make sure that the callback f, passed as argument, will be run in a context
/// where all execution threads are stopped and the full Simics API is available (Global
/// Context). This is useful for temporarily stopping the simulation to run API
/// functions not allowed in Cell Context.
///
/// If the callback is posted while an instruction is being emulated then the callback
/// be invoked when the current instruction has completed and before the next
/// instruction is dispatched.
///
/// Although no other execution threads are running when the callback is invoked, their
/// exact position in simulated time may vary between runs. If the callback accesses
/// objects in cells other than the one that run_alone was called from, then care
/// must be taken to preserve determinism.
///
/// # Context
///
/// All Contexts
/// Callback: Global Context
pub fn run_alone<F>(cb: F)
where
    F: FnOnce() -> Result<()> + 'static,
{
    let cb = Box::new(cb);
    let cb_box = Box::new(cb);
    let cb_raw = Box::into_raw(cb_box);

    debug_assert!(
        std::mem::size_of_val(&cb_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    unsafe {
        SIM_run_alone(
            Some(handle_run_alone_callback::<F>),
            cb_raw as *mut _ as *mut c_void,
        )
    }
}

extern "C" fn handle_thread_safe_callback<F>(cb: *mut c_void)
where
    F: FnOnce() + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure()
}

#[simics_exception]
/// Schedule a callback to be run with all execution stopped (global context)
///
/// If posted while an instruction is being executed, the callback will be invoked after the
/// current instruction has completed.
///
/// This is the function in the Simics API that can be called from threads that are not
/// created by Simics (i.e., from Threaded context).
///
/// When the callback is run, it is executed in Global Context, which means that it is
/// safe to call any API functions from it. Another thread in the module may at this
/// time also call API functions, if it synchronizes correctly with the callback
/// function. For example, the callback function might just signal to the foreign thread
/// to do its Simics API calls, wait for the thread to signal that it has finished, and
/// then return.
///
/// # Context
///
/// Threaded Context
/// Callback: Global Context
pub fn thread_safe_callback<F>(cb: F)
where
    F: FnOnce() + 'static,
{
    let cb = Box::new(cb);
    let cb_box = Box::new(cb);
    let cb_raw = Box::into_raw(cb_box);

    debug_assert!(
        std::mem::size_of_val(&cb_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    unsafe {
        SIM_thread_safe_callback(
            Some(handle_thread_safe_callback::<F>),
            cb_raw as *mut _ as *mut c_void,
        )
    }
}

extern "C" fn handle_in_thread_callback<F>(cb: *mut c_void)
where
    F: FnOnce() -> anyhow::Result<()> + 'static,
{
    let closure: Box<Box<F>> = unsafe { Box::from_raw(cb as *mut Box<F>) };
    closure().expect("Error running in thread callback")
}

#[simics_exception]
/// Run a closure in a new thread.
///
/// run_in_thread schedules the callback f to run on a separate thread. The callback
/// will run in Threaded Context and must observe the associated restrictions.  Simics
/// maintains a pool of worker threads used by this function, and hence the callback can
/// typically be started quickly.
///
/// The callback is allowed to block or otherwise run for a long time.
///
/// The user supplied arg parameter is passed unmodified to the callback.
///
/// # Context
///
/// Any Context
/// Callback: Threaded Context
pub fn run_in_thread<F>(cb: F)
where
    F: FnOnce() -> anyhow::Result<()> + 'static,
{
    let cb = Box::new(cb);
    let cb_box = Box::new(cb);
    let cb_raw = Box::into_raw(cb_box);

    debug_assert!(
        std::mem::size_of_val(&cb_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    unsafe {
        SIM_run_in_thread(
            Some(handle_in_thread_callback::<F>),
            cb_raw as *mut _ as *mut c_void,
        )
    }
}
