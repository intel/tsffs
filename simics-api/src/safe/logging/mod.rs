use anyhow::{ensure, Result};
use log::{LevelFilter, Log};
use simics_api_sys::{SIM_log_error, SIM_log_info};
use std::{ffi::CString, ptr::null_mut};

use crate::ConfObject;

const DEFAULT_LEVEL: LevelFilter = LevelFilter::Trace;

fn filter_to_i32(filter: LevelFilter) -> i32 {
    match filter {
        LevelFilter::Off => -1,
        LevelFilter::Error => 0,
        LevelFilter::Warn => 1,
        LevelFilter::Info => 2,
        LevelFilter::Debug => 3,
        LevelFilter::Trace => 4,
    }
}

pub struct SimicsLogger {
    level: LevelFilter,
    dev: *mut ConfObject,
}

unsafe impl Send for SimicsLogger {}
unsafe impl Sync for SimicsLogger {}

impl SimicsLogger {
    /// Create a new logger. The [`init`] function must be called to use the logger.
    pub fn new() -> Self {
        Self {
            level: DEFAULT_LEVEL,
            dev: null_mut(),
        }
    }

    /// Set the level for the logger. The [`level`] defaults to TRACE (no filtering) and can be
    /// restricted.
    pub fn with_level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Add a device to send the log messages to. This [`dev`] *should* be the pointer to the
    /// [`ConfObject`] the SIMICS module the logging is being done by, and the pointer can be
    /// obtained during the [`Module::init`] method callback when the module is loaded like so:
    ///
    /// ```text
    /// use simics_api::{Module, SimicsLogger};
    /// use log::info;
    ///
    /// impl Module for MyMod {
    ///     fn init(obj: *mut ConfObject) -> Result<*mut ConfObject> {
    ///         SimicsLogger::new()
    ///             .with_level(Level::Info.to_level_filter())
    ///             .with_dev(obj)
    ///             .init()?;
    ///         
    ///         info!("Initializing MyModule");
    ///     }
    /// }
    /// ```
    pub fn with_dev(mut self, dev: *mut ConfObject) -> Self {
        self.dev = dev;
        self
    }

    /// Initialize the [`SimicsLogger`]. This function must be called to actually initialize the
    /// logger
    pub fn init(self) -> Result<()> {
        ensure!(
            !(self.dev as *const ConfObject).is_null(),
            "Device must be provided"
        );

        log::set_max_level(self.level);
        let self_box = Box::new(self);
        log::set_logger(unsafe { &mut *Box::into_raw(self_box) }).expect("Failed to set logger");

        Ok(())
    }
}

impl Default for SimicsLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Log for SimicsLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level().to_level_filter() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = filter_to_i32(self.level);
            let group = 0;
            let msg = record.args().to_string();

            if level <= 0 {
                log_error(self.dev, group, msg).ok();
            } else {
                log_info(level, self.dev, group, msg).ok();
            }
        }
    }

    fn flush(&self) {}
}

pub fn log_info(level: i32, device: *mut ConfObject, group: i32, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_info(level, device.into(), group, msg_cstring.as_ptr());
    };

    Ok(())
}

pub fn log_error(device: *mut ConfObject, group: i32, msg: String) -> Result<()> {
    let msg_cstring = CString::new(msg)?;

    unsafe {
        SIM_log_error(device.into(), group, msg_cstring.as_ptr());
    };

    Ok(())
}
