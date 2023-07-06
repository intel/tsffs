#![deny(clippy::unwrap_used)]

use anyhow::Result;
use simics::{
    api::{GuiMode, InitArg, InitArgs},
    simics::Simics,
};

pub fn test_init() -> Result<()> {
    let args = InitArgs::default()
        .arg(InitArg::batch_mode()?)
        .arg(InitArg::gui_mode(GuiMode::None.to_string())?)
        .arg(InitArg::quiet()?)
        .arg(InitArg::no_windows()?)
        .arg(InitArg::no_settings()?);

    Simics::try_init(args)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::test_init;
    use anyhow::Result;

    #[test]
    pub fn init() -> Result<()> {
        test_init()
    }
}
