use anyhow::Error;
use std::convert::AsRef;
use std::{path::PathBuf, str::FromStr};
use strum_macros::{AsRefStr, Display};

#[derive(Debug, Clone)]
pub struct SimicsPath {
    from: SimicsPathMarker,
    to: PathBuf,
}

impl SimicsPath {
    fn new(s: &str, from: SimicsPathMarker) -> Self {
        let to = PathBuf::from(s).components().skip(1).collect();

        Self { from, to }
    }
    fn simics(s: &str) -> Self {
        Self::new(s, SimicsPathMarker::Simics)
    }

    fn script(s: &str) -> Self {
        Self::new(s, SimicsPathMarker::Script)
    }
}

#[derive(Debug, Clone, AsRefStr, Display)]
enum SimicsPathMarker {
    /// `%simics%`
    #[strum(serialize = "%simics%")]
    Simics,
    /// `%script%`
    #[strum(serialize = "%script%")]
    Script,
}

// impl ToString for SimicsPathMarker {
//     fn to_string(&self) -> String {
//         match self {
//             SimicsPathMarker::Simics => "%simics%",
//             SimicsPathMarker::Script => "%script%",
//         }
//         .to_string()
//     }
// }

#[derive(Debug, Clone)]
pub enum ArgPath {
    Path(PathBuf),
    SimicsPath(SimicsPath),
}

impl FromStr for ArgPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = PathBuf::from(s);
        Ok(match p.components().next() {
            Some(c) if c.as_os_str() == SimicsPathMarker::Script.as_ref() => {
                Self::SimicsPath(SimicsPath::script(s))
            }
            Some(c) if c.as_os_str() == SimicsPathMarker::Simics.as_ref() => {
                Self::SimicsPath(SimicsPath::simics(s))
            }
            _ => Self::Path(PathBuf::from(s).canonicalize()?),
        })
    }
}
