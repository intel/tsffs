use std::{ffi::c_void, ptr::addr_of_mut};

use anyhow::Result;
use ffi_macro::ffi;

#[derive(Debug, Default)]
pub struct Accumulator {
    total: u64,
}

impl From<*mut std::ffi::c_void> for &mut Accumulator {
    fn from(value: *mut std::ffi::c_void) -> Self {
        unsafe { *(value as *mut Self) }
    }
}

#[ffi(mod_name = "ffi", expect, self_ty = "*mut std::ffi::c_void")]
impl Accumulator {
    #[ffi(arg(rest), arg(self))]
    pub fn add(&mut self, a: u64, b: u64) -> Result<u64> {
        self.total += a;
        self.total += b;
        Ok(a + b)
    }
}

fn main() {
    let mut a = Accumulator::default();
    let res = ffi::add(1, 2, addr_of_mut!(a) as *mut c_void);
    assert_eq!(res, 3);
    assert_eq!(a.total, 3);
}
