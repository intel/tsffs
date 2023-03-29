#[macro_export]
/// Ensure a pointer value is non-null. Log an error and return an Err value if it is null.
macro_rules! nonnull {
    ($const_ptr:expr) => {{
        if $const_ptr.is_null() {
            log::error!("Pointer is NULL: $const_ptr");
            Err(anyhow::anyhow!("Pointer is NULL: $const_ptr"))
        } else {
            Ok($const_ptr)
        }
    }};
}
