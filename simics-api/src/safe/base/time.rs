use crate::last_error;
use anyhow::{bail, Result};
use simics_api_sys::SIM_object_clock;

use crate::ConfObject;

pub fn object_clock(obj: ConfObject) -> Result<ConfObject> {
    let clock = unsafe { SIM_object_clock(obj.as_const()) };

    if clock.is_null() {
        bail!("Unable to get object clock: {}", last_error());
    } else {
        Ok(clock.into())
    }
}
