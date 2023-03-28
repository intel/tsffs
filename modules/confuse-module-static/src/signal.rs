use anyhow::{bail, Error, Result};

#[derive(Debug)]
#[repr(i64)]
pub enum Signal {
    Start = 1,
}

impl TryFrom<i64> for Signal {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        match value {
            1 => Ok(Signal::Start),
            _ => bail!("No such signal!"),
        }
    }
}
