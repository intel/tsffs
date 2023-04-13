use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::{SIM_break_simulation, SIM_quit};

pub fn quit(exit_code: i32) {
    unsafe {
        SIM_quit(exit_code);
    }
}

pub fn break_simulation<S: AsRef<str>>(msg: S) -> Result<()> {
    unsafe { SIM_break_simulation(raw_cstr(msg.as_ref())?) };
    Ok(())
}
