// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Error, Result};
use simics::{AttrValue, AttrValueType};
use std::str::FromStr;

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
}

impl Default for SnapshotRestorePolicy {
    fn default() -> Self {
        Self::Always
    }
}

impl FromStr for SnapshotRestorePolicy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let normalized = s.trim().to_ascii_lowercase();

        if let Ok(interval) = normalized.parse::<usize>() {
            return Ok(Self::from_interval(interval));
        }

        match normalized.as_str() {
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => {
                if let Some(raw_n) = normalized.strip_prefix("every:") {
                    let n = raw_n.parse::<usize>().map_err(|e| {
                        anyhow!(
                            "Invalid snapshot restore policy {s}. Failed to parse interval in \
                            'every:N': {e}"
                        )
                    })?;

                    if n < 2 {
                        return Err(anyhow!(
                            "Invalid snapshot restore policy {s}. 'every:N' requires N >= 2."
                        ));
                    }

                    Ok(Self::Every(n))
                } else {
                    Err(anyhow!(
                        "Invalid snapshot restore policy {s}. Expected one of: always, never, \
                        every:N, or an integer interval (0, 1, N)."
                    ))
                }
            }
        }
    }
}

impl std::fmt::Display for SnapshotRestorePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::Every(n) => write!(f, "every:{n}"),
            Self::Never => write!(f, "never"),
        }
    }
}

impl TryFrom<AttrValue> for SnapshotRestorePolicy {
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        if let Ok(interval) = i64::try_from(value) {
            if interval < 0 {
                return Err(anyhow!(
                    "Invalid snapshot restore interval {interval}. Interval must be >= 0."
                ));
            }
            return Ok(Self::from_interval(interval as usize));
        }

        String::try_from(value)?.parse()
    }
}

impl From<SnapshotRestorePolicy> for AttrValueType {
    fn from(value: SnapshotRestorePolicy) -> Self {
        value.to_string().into()
    }
}

impl From<SnapshotRestorePolicy> for AttrValue {
    fn from(value: SnapshotRestorePolicy) -> Self {
        AttrValueType::from(value).into()
    }
}
