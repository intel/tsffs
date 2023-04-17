#[macro_export]
/// NOTE: This macro leaks memory, do not use it hot!
macro_rules! raw_cstr {
    ($s:expr) => {
        CString::new($s)
            .expect(concat!(
                "Failed to initialize C string from ",
                stringify!($s)
            ))
            .into_raw()
    };
}
