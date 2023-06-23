use anyhow::{bail, Context, Error, Result};
use std::str::FromStr;

use versions::{Chunk, Versioning};

#[non_exhaustive]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Op {
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
    Wildcard,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VersionConstraint {
    op: Op,
    version: Option<Versioning>,
}

impl FromStr for VersionConstraint {
    type Err = Error;

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
                        .context(format!("Invalid constraint string: {}", rest))?,
                )
            },
        })
    }
}

/// For tilde matches, the v2 patch can be greater than the v1 patch
pub fn version_triples_match_tilde(v1: &Versioning, v2: &Versioning) -> bool {
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
    fn matches_tilde(&self, _v: &Versioning) -> bool {
        panic!("Tilde constraint not implemented.");
    }

    fn matches_caret(&self, _v: &Versioning) -> bool {
        panic!("Caret constraint not implemented.");
    }

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
