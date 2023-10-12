use ffi_macro::ffi;
use std::ffi::c_void;

pub struct Test {
    x: u64,
}

impl From<*mut c_void> for &Test {
    fn from(value: *mut c_void) -> Self {
        unsafe { *(value as *mut Self) }
    }
}

#[ffi(mod_name = "test_ffi_forward", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(self), arg(), arg(), arg())]
    pub fn test_forward(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(mod_name = "test_ffi_reverse", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(), arg(), arg(), arg(self))]
    pub fn test_reverse(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(mod_name = "test_ffi_middle", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(), arg(self), arg(), arg())]
    pub fn test_middle(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(mod_name = "test_ffi_forward_rest", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(self), arg(rest))]
    pub fn test_forward_rest(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(mod_name = "test_ffi_reverse_rest", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(rest), arg(self))]
    pub fn test_reverse_rest(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(mod_name = "test_ffi_middle_rest", self_ty = "*mut std::ffi::c_void")]
impl Test {
    #[ffi(arg(), arg(self), arg(rest))]
    pub fn test_middle_rest(&self, a: u64, b: u64, c: u64) -> u64 {
        a + b + c + self.x
    }
}

#[ffi(
    mod_name = "test_ffi_forward_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(self), arg(), arg(), arg())]
    pub fn test_forward_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[ffi(
    mod_name = "test_ffi_reverse_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(), arg(), arg(), arg(self))]
    pub fn test_reverse_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[ffi(
    mod_name = "test_ffi_middle_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(), arg(self), arg(), arg())]
    pub fn test_middle_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[ffi(
    mod_name = "test_ffi_forward_rest_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(self), arg(rest))]
    pub fn test_forward_rest_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[ffi(
    mod_name = "test_ffi_reverse_rest_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(rest), arg(self))]
    pub fn test_reverse_rest_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[ffi(
    mod_name = "test_ffi_middle_rest_expect",
    expect,
    self_ty = "*mut std::ffi::c_void"
)]
impl Test {
    #[ffi(arg(), arg(self), arg(rest))]
    pub fn test_middle_rest_expect(&self, a: u64, b: u64, c: u64) -> anyhow::Result<u64> {
        Ok(a + b + c + self.x)
    }
}

#[cfg(test)]
mod test {}
