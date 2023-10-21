// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Logging macros wrapping the underlying SIMICS logging functionality

#[macro_export]
/// Log an error message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object. Note that errors logged may trigger simulator exit if
/// the simulator is run with the `-werror` flag.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::error;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// error!(module_instance, "Error message with parameter {}", parameter);
/// error!("Error message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_error`] API instead.
///
/// This macro will cause simulator exit, triggering a cascading panic, if it is called while
/// the simulator is run with `-werror`.
macro_rules! error {
    ($obj:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(simics::api::logging::LogLevel::Error, $obj, $fmt $($args)*)
    };
    ($fmt:literal $($args:tt)*) => {
        simics::log!(
            simics::api::logging::LogLevel::Warn,
            $fmt
            $($args)*
        )
    };
}

#[macro_export]
/// Log a warning message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::warn;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// warn!(module_instance, "Warning message with parameter {}", parameter);
/// warn!("Warning message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_info`] API instead.
macro_rules! warn {
    ($obj:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(simics::api::logging::LogLevel::Warn, $obj, $fmt $($args)*)
    };
    ($fmt:literal $($args:tt)*) => {
        simics::log!(
            simics::api::logging::LogLevel::Warn,
            $fmt
            $($args)*
        )
    };
}

#[macro_export]
/// Log an informational message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::info;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// info!(module_instance, "Info message with parameter {}", parameter);
/// info!("Info message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_info`] API instead.
macro_rules! info {
    ($obj:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(simics::api::logging::LogLevel::Info, $obj, $fmt $($args)*)
    };
    ($fmt:literal $($args:tt)*) => {
        simics::log!(
            simics::api::logging::LogLevel::Info,
            $fmt
            $($args)*
        )
    };
}

#[macro_export]
/// Log a debug message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::debug;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// debug!(module_instance, "Debug message with parameter {}", parameter);
/// debug!("Debug message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_info`] API instead.
macro_rules! debug {
    ($obj:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(simics::api::logging::LogLevel::Debug, $obj, $fmt $($args)*)
    };
    ($fmt:literal $($args:tt)*) => {
        simics::log!(
            simics::api::logging::LogLevel::Debug,
            $fmt
            $($args)*
        )
    };
}

#[macro_export]
/// Log a trace message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::trace;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// trace!(module_instance, "Trace message with parameter {}", parameter);
/// trace!("Trace message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_info`] API instead.
macro_rules! trace {
    ($obj:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(simics::api::logging::LogLevel::Trace, $obj, $fmt $($args)*)
    };
    ($fmt:literal $($args:tt)*) => {
        simics::log!(
            simics::api::logging::LogLevel::Trace,
            $fmt
            $($args)*
        )
    };
}

#[macro_export]
/// Log a message via the SIMICS logging API. If an object is provided, the
/// message will be logged through that object. If not, the message will be logged
/// through the base `sim` object. [`trace`], [`debug`], [`info`], [`warn`] , and [`error`] messages
/// use this macro internally. This macro takes the log level as its first parameter.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::log::log;
///
/// let module_instance = get_object("object_name")?;
/// let parameter = 0;
///
/// log!(LogLevel::Debug, module_instance, "Debug message with parameter {}", parameter);
/// log!(LogLevel::Debug, "Debug message without object with parameter {}", parameter);
/// ```
///
/// # Panics
///
/// This macro will panic of there is an error in the logging call. This is unlikely if the
/// object is valid, but if your use case requires handling errors or is dynamically generating
/// objects without static lifetimes, you should use the internal [`log_info`] API instead.
macro_rules! log {
    ($level:expr, $obj:expr, $fmt:literal $($args:tt)*) => {
        match $level {
            simics::api::logging::LogLevel::Error => {
                #[allow(unnecessary_cast)]
                simics::api::logging::log_error(
                    $obj as *mut simics::api::ConfObject,
                    format!($fmt $($args)*),
                ).unwrap_or_else(|e| {
                    panic!(
                        "Fatal error attempting to log message {}: {}",
                        format!($fmt $($args)*),
                        e
                    )
                })
            }
            _ => {
                #[allow(unnecessary_cast)]
                simics::api::logging::log_info(
                    $level,
                    $obj as *mut simics::api::ConfObject,
                    format!($fmt $($args)*),
                ).unwrap_or_else(|e| {
                    panic!(
                        "Fatal error attempting to log message {}: {}",
                        format!($fmt $($args)*),
                        e
                    )
                })
            }
        }
    };
    ($level:expr, $fmt:literal $($args:tt)*) => {
        simics::log!(
            $level,
            simics::api::simulator::sim_conf_object::get_object("sim")
                .unwrap_or_else(|e| panic!("Unable to get base sim object: {e}")),
            $fmt
            $($args)*
        )
    };
}
