use crate::{last_error, ConfObject};
use anyhow::{bail, Result};
use simics_api_sys::SIM_object_clock;

pub fn object_clock(obj: *mut ConfObject) -> Result<*mut ConfObject> {
    let clock = unsafe { SIM_object_clock(obj as *const ConfObject) };

    if clock.is_null() {
        bail!("Unable to get object clock: {}", last_error());
    } else {
        Ok(clock.into())
    }
}
