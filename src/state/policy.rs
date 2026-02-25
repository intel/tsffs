// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Error, Result};
use simics::{AttrValue, AttrValueType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub(crate) enum SnapshotRestorePolicy {
    Always,
    Every(usize),
    Never,
}

impl SnapshotRestorePolicy {
    fn from_interval(interval: usize) -> Self {
        match interval {
            0 => Self::Never,
            1 => Self::Always,
            n => Self::Every(n),
        }
    }

    fn as_interval(self) -> i64 {
        match self {
            Self::Never => 0,
            Self::Always => 1,
            Self::Every(n) => n as i64,
        }
    }
}

impl Default for SnapshotRestorePolicy {
    fn default() -> Self {
        Self::Always
    }
}

impl TryFrom<AttrValue> for SnapshotRestorePolicy {
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        let interval = i64::try_from(value).map_err(|_| {
            anyhow!("Invalid snapshot restore interval type. Expected a non-negative integer.")
        })?;

        if interval < 0 {
            return Err(anyhow!(
                "Invalid snapshot restore interval {interval}. Interval must be >= 0."
            ));
        }

        Ok(Self::from_interval(interval as usize))
    }
}

impl From<SnapshotRestorePolicy> for AttrValueType {
    fn from(value: SnapshotRestorePolicy) -> Self {
        value.as_interval().into()
    }
}

impl From<SnapshotRestorePolicy> for AttrValue {
    fn from(value: SnapshotRestorePolicy) -> Self {
        AttrValueType::from(value).into()
    }
}
