#![deny(clippy::unwrap_used)]

use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, ffi::CString};

struct RawCStrs(RefCell<HashMap<String, *mut i8>>);

impl Drop for RawCStrs {
    fn drop(&mut self) {
        self.0.borrow_mut().iter_mut().for_each(|(_, c)| unsafe {
            drop(CString::from_raw(*c));
        });
        self.0.borrow_mut().clear();
    }
}

thread_local! {
    static RAW_CSTRS: RawCStrs = RawCStrs(RefCell::new(HashMap::new()));
}

/// Create a constant raw C string as a `*mut i8` from a Rust string reference. C Strings are cached,
/// and creating the same string twice will cost zero additional memory. This is useful when calling
/// C APIs that take a string as an argument, particularly when the string can't be known at compile
/// time, although this function is also efficient in space (but not time) when a constant string
/// is known. For compile-time constants, you can use `c_str!`.
///
/// # Safety
///
/// - Do *not* use [`String::from_raw_parts`] to convert the pointer back to a [`String`]. This
///   may cause a double free because the [`String`] will take ownership of the pointer. Use
///   [`CStr::from_ptr`] instead, and convert to a string with
///   `.to_str().expect("...").to_owned()` instead.
///
pub fn raw_cstr<S>(str: S) -> Result<*mut i8>
where
    S: AsRef<str>,
{
    // This is the old, inefficient way to implement this. Instead, we use a thread local cache
    // of raw strings, because we only use this function to talk to SIMICS
    // let raw = CString::new(str.as_ref())?.into_raw();
    // Ok(raw)
    RAW_CSTRS.with(|rc| {
        let mut raw_cstrs_map = rc.0.borrow_mut();
        let saved = raw_cstrs_map.get(str.as_ref());

        if let Some(saved) = saved {
            Ok(*saved)
        } else {
            let raw = CString::new(str.as_ref())?.into_raw();
            raw_cstrs_map.insert(str.as_ref().to_string(), raw);
            Ok(raw)
        }
    })
}

pub use byte_strings::c_str;
