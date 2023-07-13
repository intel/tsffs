#[macro_export]
/// Ensure a pointer value is non-null. Log an error and return an Err value if it is null.
macro_rules! nonnull {
    ($const_ptr:expr) => {{
        tracing::info!("Checking if pointer is null: {}", $const_ptr.is_null());
        anyhow::ensure!(
            !$const_ptr.is_null(),
            format!("Pointer {} is null unexpectedly", stringify!($const_ptr))
        );
        $const_ptr
    }};
}
