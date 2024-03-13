// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! High level bindings to the SIMICS API in idiomatic Rust

#![deny(clippy::unwrap_used)]
#![deny(missing_docs)]

// NOTE: Unsafe code is allowed in the API, as it is wrapping a C API
pub mod api;
#[forbid(unsafe_code)]
pub mod error;
#[forbid(unsafe_code)]
pub mod log;
#[forbid(unsafe_code)]
pub mod util;

use std::{any::Any, panic::PanicInfo};

// Glob re-export the api and utilities
pub use api::*;
pub use error::*;
// NOTE: the log module only contains macros, so we don't need to re-export it
pub use util::*;

// Re-export simics_macro
pub use simics_macro::*;

#[cfg(feature = "global-allocator")]
#[global_allocator]
/// All crates using the SIMICS API must also use the SIMICS allocator as their
/// global allocator, hence we set it here
static GLOBAL: SimicsAlloc = SimicsAlloc;

/// Attempt to produce a `&str` message (with a default)
/// from a [`std::panic::catch_unwind`] payload.
/// See [module docs][crate] for usage.
pub fn panic_message(payload: &Box<dyn Any + Send>) -> &str {
    imp::get_panic_message(payload.as_ref()).unwrap_or({
        // Copy what rustc does in the default panic handler
        "Box<dyn Any>"
    })
}

/// Attempt to produce a `&str` message
/// from a [`std::panic::catch_unwind`] payload.
/// See [module docs][crate] for usage.
pub fn get_panic_message(payload: &Box<dyn Any + Send>) -> Option<&str> {
    imp::get_panic_message(payload.as_ref())
}

/// Attempt to produce a `&str` message (with a default)
/// from a [`std::panic::PanicInfo`].
/// See [module docs][crate] for usage.
pub fn panic_info_message<'pi>(panic_info: &'pi PanicInfo<'_>) -> &'pi str {
    imp::get_panic_message(panic_info.payload()).unwrap_or({
        // Copy what rustc does in the default panic handler
        "Box<dyn Any>"
    })
}

/// Attempt to produce a `&str` message (with a default)
/// from a [`std::panic::PanicInfo`].
/// See [module docs][crate] for usage.
pub fn get_panic_info_message<'pi>(panic_info: &'pi PanicInfo<'_>) -> Option<&'pi str> {
    imp::get_panic_message(panic_info.payload())
}

mod imp {
    use std::any::Any;

    /// Attempt to produce a message from a borrowed `dyn Any`. Note that care must be taken
    /// when calling this to avoid a `Box<dyn Any>` being coerced to a `dyn Any` itself.
    pub(super) fn get_panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
        // taken from: https://github.com/rust-lang/rust/blob/4b9f4b221b92193c7e95b1beb502c6eb32c3b613/library/std/src/panicking.rs#L194-L200
        match payload.downcast_ref::<&'static str>() {
            Some(msg) => Some(*msg),
            None => match payload.downcast_ref::<String>() {
                Some(msg) => Some(msg.as_str()),
                // Copy what rustc does in the default panic handler
                None => None,
            },
        }
    }
}

/// Panic handler for Simics modules. This will log the panic message and then
/// call `SIM_quit` to exit the simulator (backtraces are not available in the
/// simulator, so we don't bother trying to print one). It is usually automatically
/// installed by any #[simics_init] attribute macro, but can be manually installed
/// using `std::panic::set_hook`.
pub fn panic_handler(info: &PanicInfo<'_>) -> ! {
    let message = panic_info_message(info);

    if let Some(location) = info.location() {
        eprintln!("{message}: {location}");
    } else {
        eprintln!("{message}");
    }

    unsafe { crate::sys::SIM_quit(1) }
}
