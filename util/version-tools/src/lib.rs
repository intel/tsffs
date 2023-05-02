use anyhow::{bail, Context, Error, Result};
use std::str::FromStr;

use versions::Versioning;

#[non_exhaustive]
enum Op {
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
    Wildcard,
}

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
