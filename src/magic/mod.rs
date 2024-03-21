//! Magic number definitions

use std::fmt::Display;

use num_derive::{FromPrimitive, ToPrimitive};
#[allow(unused_imports)]
use num_traits::{FromPrimitive as _, ToPrimitive as _};
use serde::{Deserialize, Serialize};

#[repr(i64)]
#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize, FromPrimitive, ToPrimitive)]
pub enum MagicNumber {
    StartBufferPtrSizePtr = 1,
    StartBufferPtrSizeVal = 2,
    StartBufferPtrSizePtrVal = 3,
    StopNormal = 4,
    StopAssert = 5,
}

impl Display for MagicNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as i64)
    }
}
