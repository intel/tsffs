// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Extensions to [`Versioning`] to make sorting, comparing, and constraining versions easier
//!
//! # Examples
//!
//! ```
//! use anyhow::anyhow;
//! use version_tools::VersionConstraint;
//! use versions::Versioning;
//! use std::str::FromStr;
//!
//! let constraint = VersionConstraint::from_str(">=1.0.0")?;
//! assert!(constraint.matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
//! assert!(constraint.matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
//! assert!(!constraint.matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
//! # Ok::<(), anyhow::Error>(())
//! ```

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use anyhow::{anyhow, bail, Error, Result};
use serde::{de::Error as _, Deserialize, Deserializer};
use std::{fmt::Display, str::FromStr};
pub use versions::*;

#[non_exhaustive]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// Operation component of a version constraint. For example, in `==1.0.0`, the `==` is the
/// operation that constrains the version to *exactly* 1.0.0. You can find more detailed
/// documentation in [`semver::Op`](https://docs.rs/semver/latest/semver/enum.Op.html)
pub enum Op {
    /// `==` operation, exactly equal to this version
    Exact,
    /// `>` operation, must be semantically greater than this version
    Greater,
    /// `>=` operation, must be semantically greater than or equal to this version
    GreaterEq,
    /// `<` operation, must be semantically less than this version
    Less,
    /// `<=` operation, must be semantically less than or equal to this version
    LessEq,
    /// *Not implemented yet*, must be at least this version, but not more than one minor version
    /// greater (the patch version may increase)
    Tilde,
    /// *Not implemented yet*, any part to the right of the first non-zero part of the version
    /// may increase
    Caret,
    /// Any version matches
    Wildcard,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Op::Exact => "==",
                Op::Greater => ">",
                Op::GreaterEq => ">=",
                Op::Less => "<",
                Op::LessEq => "<=",
                Op::Tilde => "~",
                Op::Caret => "^",
                Op::Wildcard => "*",
            }
        )
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// A version constraint with an operation and a version, unless the operation is a wildcard
/// in which case the version is omitted.
pub struct VersionConstraint {
    op: Op,
    version: Option<Versioning>,
}

impl VersionConstraint {
    pub fn op(&self) -> &Op {
        &self.op
    }

    pub fn version(&self) -> Option<&Versioning> {
        self.version.as_ref()
    }
}

impl From<&str> for VersionConstraint {
    fn from(value: &str) -> Self {
        value
            .parse()
            .unwrap_or_else(|_| panic!("Invalid version constraint {}", value))
    }
}

impl FromStr for VersionConstraint {
    type Err = Error;

    /// Convert from a string like `==1.0.0` or `*` to a [`VersionConstraint`]
    ///
    /// # Examples
    fn from_str(s: &str) -> Result<Self> {
        let comparator: String = s
            .chars()
            .take_while(|c| matches!(c, '=' | '>' | '<' | '~' | '^' | '*'))
            .collect();
        let rest = &s[comparator.len()..];
        let comp = match comparator.as_ref() {
            "==" => Op::Exact,
            ">" => Op::Greater,
            ">=" => Op::GreaterEq,
            "<" => Op::Less,
            "<=" => Op::LessEq,
            "~" => Op::Tilde,
            "^" => Op::Caret,
            "*" => Op::Wildcard,
            "" => Op::Exact,
            _ => bail!("Invalid constraint {}", comparator),
        };

        Ok(Self {
            op: comp,
            version: if rest.is_empty() {
                None
            } else {
                Some(
                    Versioning::new(rest)
                        .ok_or_else(|| anyhow!("Invalid constraint string: {}", rest))?,
                )
            },
        })
    }
}

impl Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.op,
            self.version
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default()
        )
    }
}

/// Checks whether two versioning triples are "tilde-compatible", that is v2's patch version
/// may be greater than v1's, but its major and minor versions may not be.
/// For tilde matches, the v2 patch can be greater than the v1 patch
fn version_triples_match_tilde(v1: &Versioning, v2: &Versioning) -> bool {
    match v1 {
        Versioning::Ideal(v1) => {
            if let Versioning::Ideal(v2) = v2 {
                v1.major == v2.major && v1.minor == v2.minor && v2.patch >= v1.patch
            } else {
                false
            }
        }
        Versioning::General(v1) => {
            if let Versioning::General(v2) = v2 {
                if v1.chunks.0.len() != v2.chunks.0.len() {
                    false
                } else {
                    // Check all but the last
                    for (v1_chunk, v2_chunk) in v1
                        .chunks
                        .0
                        .iter()
                        .rev()
                        .skip(1)
                        .rev()
                        .zip(v2.chunks.0.iter().rev().skip(1).rev())
                    {
                        if v1_chunk != v2_chunk {
                            return false;
                        }
                    }
                    match (v1.chunks.0.last(), v2.chunks.0.last()) {
                        // TODO: Do our best with strings. Right now, the alpha patch version can be "less" than the
                        // first one and this will still be true
                        (Some(Chunk::Alphanum(_a1)), Some(Chunk::Alphanum(_a2))) => true,
                        (Some(Chunk::Numeric(n1)), Some(Chunk::Numeric(n2))) => n2 >= n1,
                        _ => false,
                    }
                }
            } else {
                false
            }
        }
        // Complex can't be tilde-equal because they're not semantic
        Versioning::Complex(_svc) => false,
    }
}

impl VersionConstraint {
    fn matches_tilde(&self, v: &Versioning) -> bool {
        if let Some(version) = self.version.as_ref() {
            version_triples_match_tilde(version, v)
        } else {
            false
        }
    }

    fn matches_caret(&self, _v: &Versioning) -> bool {
        panic!("Caret constraint not implemented.");
    }

    /// Check if a version matches a version constraint.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use anyhow::anyhow;
    /// use version_tools::VersionConstraint;
    /// use versions::Versioning;
    /// use std::str::FromStr;
    ///
    /// let constraint_gt = VersionConstraint::from_str(">=1.0.0")?;
    /// assert!(constraint_gt.matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(constraint_gt.matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(!constraint_gt.matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
    ///
    /// let constraint_wild = VersionConstraint::from_str("*")?;
    /// assert!(constraint_wild.matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(constraint_wild.matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(constraint_wild.matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
    ///
    /// let constraint_eq = VersionConstraint::from_str("==1.0.0")?;
    /// assert!(constraint_eq.matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(!constraint_eq.matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
    /// assert!(!constraint_eq.matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
    ///
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn matches(&self, v: &Versioning) -> bool {
        if let Some(sv) = &self.version {
            match self.op {
                Op::Exact => v == sv,
                Op::Greater => v > sv,
                Op::GreaterEq => v >= sv,
                Op::Less => v < sv,
                Op::LessEq => v <= sv,
                Op::Tilde => self.matches_tilde(v),
                Op::Caret => self.matches_caret(v),
                Op::Wildcard => true,
            }
        } else {
            matches!(self.op, Op::Wildcard)
        }
    }
}

impl Default for VersionConstraint {
    fn default() -> Self {
        Self {
            op: Op::Wildcard,
            version: None,
        }
    }
}

pub fn versioning_from_string<'de, D>(deserializer: D) -> Result<Versioning, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    Versioning::new(&s)
        .ok_or_else(|| anyhow!("Unable to deserialize {} as versioning", s))
        .map_err(D::Error::custom)
}

pub fn version_constraint_from_string<'de, D>(
    deserializer: D,
) -> Result<VersionConstraint, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    s.parse().map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use crate::VersionConstraint;
    use anyhow::{anyhow, Result};
    use std::str::FromStr;
    use versions::Versioning;

    #[test]
    fn test_eq() -> Result<()> {
        assert!(VersionConstraint::from_str("==1.0.0")?
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("==1.1.0")?
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("==0.9.0")?
            .matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("==6.0.pre134")?
            .matches(&Versioning::new("6.0.pre134").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("==6.0.166")?
            .matches(&Versioning::new("6.0.166").ok_or_else(|| anyhow!("Invalid version"))?));

        // The == can be left off as a shorthand
        assert!(VersionConstraint::from_str("1.0.0")?
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("1.1.0")?
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("0.9.0")?
            .matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("6.0.pre134")?
            .matches(&Versioning::new("6.0.pre134").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(VersionConstraint::from_str("6.0.166")?
            .matches(&Versioning::new("6.0.166").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }

    #[test]
    fn test_wild() -> Result<()> {
        let constraint_wild = VersionConstraint::from_str("*")?;
        assert!(constraint_wild
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_wild
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_wild
            .matches(&Versioning::new("0.9.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_wild
            .matches(&Versioning::new("6.0.pre134").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_wild
            .matches(&Versioning::new("6.0.166").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }

    #[test]
    fn test_gt() -> Result<()> {
        let constraint_gt = VersionConstraint::from_str(">1.1.1")?;

        assert!(!constraint_gt
            .matches(&Versioning::new("1.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gt
            .matches(&Versioning::new("2.2.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gt
            .matches(&Versioning::new("2.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gt
            .matches(&Versioning::new("1.2.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gt
            .matches(&Versioning::new("1.1.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gt
            .matches(&Versioning::new("0.9.9").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gt
            .matches(&Versioning::new("0.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gt
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gt
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }

    #[test]
    fn test_lt() -> Result<()> {
        let constraint_lt = VersionConstraint::from_str("<1.1.1")?;
        assert!(!constraint_lt
            .matches(&Versioning::new("1.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lt
            .matches(&Versioning::new("2.2.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lt
            .matches(&Versioning::new("2.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lt
            .matches(&Versioning::new("1.2.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lt
            .matches(&Versioning::new("1.1.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lt
            .matches(&Versioning::new("0.9.9").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lt
            .matches(&Versioning::new("0.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lt
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lt
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }

    #[test]
    fn test_gte() -> Result<()> {
        let constraint_gte = VersionConstraint::from_str(">=1.1.1")?;
        assert!(constraint_gte
            .matches(&Versioning::new("1.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gte
            .matches(&Versioning::new("2.2.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gte
            .matches(&Versioning::new("2.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gte
            .matches(&Versioning::new("1.2.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_gte
            .matches(&Versioning::new("1.1.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gte
            .matches(&Versioning::new("0.9.9").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gte
            .matches(&Versioning::new("0.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gte
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_gte
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }

    #[test]
    fn test_lte() -> Result<()> {
        let constraint_lte = VersionConstraint::from_str("<=1.1.1")?;
        assert!(constraint_lte
            .matches(&Versioning::new("1.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lte
            .matches(&Versioning::new("2.2.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lte
            .matches(&Versioning::new("2.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lte
            .matches(&Versioning::new("1.2.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(!constraint_lte
            .matches(&Versioning::new("1.1.2").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lte
            .matches(&Versioning::new("0.9.9").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lte
            .matches(&Versioning::new("0.1.1").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lte
            .matches(&Versioning::new("1.0.0").ok_or_else(|| anyhow!("Invalid version"))?));
        assert!(constraint_lte
            .matches(&Versioning::new("1.1.0").ok_or_else(|| anyhow!("Invalid version"))?));

        Ok(())
    }
}
