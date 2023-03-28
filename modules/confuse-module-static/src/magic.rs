use anyhow::{anyhow, Error, Result};

#[repr(i64)]
pub enum Magic {
    Stop = 0x4242,
    Start = 0x4343,
}

impl TryFrom<i64> for Magic {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        match value {
            x if x == Magic::Stop as i64 => Ok(Magic::Stop),
            x if x == Magic::Start as i64 => Ok(Magic::Start),
            _ => Err(anyhow!("Invalid magic value {}", value)),
        }
    }
}
