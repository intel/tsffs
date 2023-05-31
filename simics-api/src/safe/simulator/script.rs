use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::SIM_run_command;

use crate::AttrValue;

pub fn run_command<S: AsRef<str>>(line: S) -> Result<AttrValue> {
    Ok(unsafe { SIM_run_command(raw_cstr(line)?) })
}
