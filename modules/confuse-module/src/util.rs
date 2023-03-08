


#[macro_export]
macro_rules! nonnull {
    ($const_ptr:expr) => {{
        if $const_ptr.is_null() {
            error!("Pointer is NULL: $const_ptr");
            Err(anyhow!("Pointer is NULL: $const_ptr"))
        } else {
            Ok($const_ptr)
        }
    }};
}
