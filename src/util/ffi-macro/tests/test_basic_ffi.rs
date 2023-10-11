use anyhow::Result;
use ffi_macro::ffi;

pub struct Test {}

#[ffi(visibility = "pub(crate)", name = "test_ffi_test")]
impl Test {
    #[args(expect = true)]
    pub fn test(&self, a: bool) -> Result<()> {
        println!("{}", a);
        Ok(())
    }
}
