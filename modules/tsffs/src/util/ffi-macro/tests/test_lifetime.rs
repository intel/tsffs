use ffi_macro::ffi;
use std::ffi::c_void;

pub struct Test<'a>
where
    'a: 'static,
{
    x: &'a mut u64,
}

#[ffi(from_ptr, expect, self_ty = "*mut c_void")]
impl<'a> Test<'a>
where
    'a: 'static,
{
    #[ffi(arg(rest), arg(self))]
    pub fn get(&mut self, y: u64) -> u64 {
        *self.x += y;
        *self.x
    }
}
