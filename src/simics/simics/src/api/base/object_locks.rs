//! Thread-related locking functionality
//!
//! See the SIMICS API reference manual, chapter 2: Threading Model for more information

use crate::api::sys::VT_acquire_object;
use anyhow::Result;

pub fn acquire_object() -> Result<()> {
    VT_acquire_object()
}
