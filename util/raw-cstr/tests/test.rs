use std::ffi::CStr;

use anyhow::Result;
use raw_cstr::raw_cstr;

#[test]
fn test_string_eq() -> Result<()> {
    const ORIG_STR: &str = "Hello, world!";
    let c_str = raw_cstr(ORIG_STR)?;

    let rust_str = unsafe { String::from_raw_parts(c_str as *mut u8, 13, 13) };

    assert_eq!(rust_str, ORIG_STR, "Raw C String doesn't match");

    Ok(())
}

#[test]
fn test_strings_reused() -> Result<()> {
    const ORIG_STR: &str = "Hello, world!";
    let c_str = raw_cstr(ORIG_STR)?;
    let o_c_str = raw_cstr(ORIG_STR)?;

    assert_eq!(
        c_str as usize, o_c_str as usize,
        "String pointers are different"
    );

    let rust_str = unsafe { CStr::from_ptr(c_str) }
        .to_str()
        .expect("Couldn't get CStr")
        .to_owned();
    let o_rust_str = unsafe { CStr::from_ptr(o_c_str) }
        .to_str()
        .expect("Couldn't get CStr")
        .to_owned();

    assert_eq!(rust_str, ORIG_STR, "Raw C String doesn't match rust string");
    assert_eq!(
        o_rust_str, ORIG_STR,
        "Other Raw C String doesn't match rust string"
    );
    assert_eq!(rust_str, o_rust_str, "Raw C strings don't match each other");

    Ok(())
}
