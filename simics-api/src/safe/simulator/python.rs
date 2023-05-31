use std::path::Path;

use crate::{clear_exception, last_error, AttrValue, SimException};
use anyhow::{anyhow, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{SIM_source_python, VT_call_python_module_function};

pub fn call_python_module_function<S: AsRef<str>>(
    module: S,
    function: S,
    args: *mut AttrValue,
) -> Result<AttrValue> {
    Ok(unsafe {
        VT_call_python_module_function(
            raw_cstr(module.as_ref())?,
            raw_cstr(function.as_ref())?,
            args.into(),
        )
    })
}

pub fn source_python<P: AsRef<Path>>(file: P) -> Result<()> {
    unsafe { SIM_source_python(raw_cstr(file.as_ref().to_string_lossy())?) }

    match clear_exception()? {
        SimException::NoException => Ok(()),
        _ => Err(anyhow!("Error running python script: {}", last_error())),
    }
}
