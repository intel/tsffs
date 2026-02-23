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

#[cfg(test)]
mod tests {
    use super::*;

    fn attr_from_i64(value: i64) -> AttrValue {
        AttrValueType::from(value).into()
    }

    #[test]
    fn parses_non_negative_integer_intervals() {
        assert_eq!(
            SnapshotRestorePolicy::try_from(attr_from_i64(0)).expect("0 should parse"),
            SnapshotRestorePolicy::Never
        );
        assert_eq!(
            SnapshotRestorePolicy::try_from(attr_from_i64(1)).expect("1 should parse"),
            SnapshotRestorePolicy::Always
        );
        assert_eq!(
            SnapshotRestorePolicy::try_from(attr_from_i64(10)).expect("10 should parse"),
            SnapshotRestorePolicy::Every(10)
        );
    }

    #[test]
    fn rejects_negative_intervals() {
        let err = SnapshotRestorePolicy::try_from(attr_from_i64(-1))
            .expect_err("negative values must be rejected");
        assert!(
            err.to_string().contains("Interval must be >= 0"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_non_integer_attr_values() {
        let value: AttrValue = AttrValueType::from("never").into();
        let err = SnapshotRestorePolicy::try_from(value)
            .expect_err("string values must be rejected for int-only policy");
        assert!(
            err.to_string().contains("Expected a non-negative integer"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn roundtrip_through_attr_value_preserves_policy() {
        let cases = [
            SnapshotRestorePolicy::Never,
            SnapshotRestorePolicy::Always,
            SnapshotRestorePolicy::Every(64),
        ];

        for policy in cases {
            let value: AttrValue = policy.into();
            let parsed =
                SnapshotRestorePolicy::try_from(value).expect("roundtrip conversion should work");
            assert_eq!(parsed, policy);
        }
    }
}
