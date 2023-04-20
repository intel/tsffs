use crate::last_error;
use anyhow::{bail, Result};
use simics_api_sys::SIM_object_clock;

use crate::OwnedMutConfObjectPtr;

pub fn object_clock(obj: OwnedMutConfObjectPtr) -> Result<OwnedMutConfObjectPtr> {
    let clock = unsafe { SIM_object_clock(obj.as_const()) };

    if clock.is_null() {
        bail!("Unable to get object clock: {}", last_error());
    } else {
        Ok(clock.into())
    }
}
