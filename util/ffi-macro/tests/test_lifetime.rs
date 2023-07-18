#![allow(clippy::new_without_default)]

use anyhow::Result;
use ffi_macro::{callback_wrappers, params};

struct TestStruct<'a> {
    _x: &'a [u8],
}

impl<'a> TestStruct<'a> {
    pub fn new() -> Self {
        Self { _x: &[1] }
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl<'a> TestStruct<'a> {
    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn do_return(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    #[params(!slf: *mut std::ffi::c_void, ...)]
    pub fn do_result(&self, a: i32, b: i32) -> Result<i32> {
        Ok(a + b)
    }

    #[params(!slf: *mut std::ffi::c_void)]
    pub fn do_noargs(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'b> From<*mut std::ffi::c_void> for &'a mut TestStruct<'b> {
    fn from(ptr: *mut std::ffi::c_void) -> &'a mut TestStruct<'b> {
        unsafe { &mut *(ptr as *mut TestStruct) }
    }
}

#[test]
fn test_cb() {
    let t = Box::new(TestStruct::new());
    // Pretend this object was created in C by a malloc or some such thing
    let t_ptr = Box::into_raw(t) as *mut std::ffi::c_void;
    println!("{}", teststruct_callbacks::do_return(t_ptr, 1, 2));
    teststruct_callbacks::do_result(t_ptr, 1, 2);
    teststruct_callbacks::do_noargs(t_ptr);
    // Free it on drop since it wasn't actually created in C by a malloc or some such thing
    let _: Box<TestStruct<'_>> = unsafe { Box::from_raw(t_ptr as *mut TestStruct<'_>) };
    println!("Done");
}
