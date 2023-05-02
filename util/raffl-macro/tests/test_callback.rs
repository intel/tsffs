use anyhow::Result;
use raffl_macro::{callback_wrappers, params};

struct TestStruct {}

impl TestStruct {
    pub fn new() -> Self {
        Self {}
    }
}

#[callback_wrappers(pub, unwrap_result)]
impl TestStruct {
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

impl<'a> From<*mut std::ffi::c_void> for &'a mut TestStruct {
    fn from(ptr: *mut std::ffi::c_void) -> &'a mut TestStruct {
        unsafe { *(ptr as *mut Self) }
    }
}

#[test]
fn test_cb() {
    let t = TestStruct::new();
    let t_ptr = &t as *const TestStruct as *mut std::ffi::c_void;
    println!("{}", teststruct_callbacks::do_return(t_ptr, 1, 2));
    teststruct_callbacks::do_result(t_ptr, 1, 2);
    teststruct_callbacks::do_noargs(t_ptr);
    println!("Done");
}
