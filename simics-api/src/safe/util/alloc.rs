use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::{mm_free, mm_zalloc};
use std::{ffi::c_void, mem::transmute};

#[macro_export]
macro_rules! simics_alloc {
    ($typ:ty, $sz:expr) => {
        $crate::alloc($sz, stringify!($typ), file!(), line!() as i32)
    };
}

pub fn alloc<T, S: AsRef<str>>(
    size: usize,
    typename: S,
    filename: S,
    line_number: i32,
) -> Result<*mut T> {
    unsafe {
        let res = mm_zalloc(
            size,
            size,
            raw_cstr(typename)?,
            raw_cstr(filename)?,
            line_number,
        );
        Ok(transmute(res))
    }
}

pub fn free<T>(ptr: *mut T) {
    unsafe { mm_free(ptr as *mut c_void) };
}
