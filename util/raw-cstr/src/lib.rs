use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, ffi::CString};

// thread_local! {
//     static RAW_CSTRS: RefCell<HashMap<String, *mut i8>> = RefCell::new(HashMap::new());
// }

pub fn raw_cstr<S: AsRef<str>>(str: S) -> Result<*mut i8> {
    let raw = CString::new(str.as_ref())?.into_raw();
    Ok(raw)
    // RAW_CSTRS.with(|rc| {
    //     let mut raw_cstrs_map = rc.borrow_mut();
    //     let saved = raw_cstrs_map.get(str.as_ref());

    //     if let Some(saved) = saved {
    //         Ok(*saved)
    //     } else {
    //         let raw = CString::new(str.as_ref())?.into_raw();
    //         raw_cstrs_map.insert(str.as_ref().to_string(), raw);
    //         Ok(raw)
    //     }
    // })
}

pub use byte_strings::c_str;
