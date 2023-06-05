use anyhow::Result;
use simics::simics::Simics;
use simics_api::{GuiMode, InitArg, InitArgs};

#[test]
pub fn test_hello_world() -> Result<()> {
    let mut init_args = InitArgs::default()
        .arg(InitArg::gui_mode(GuiMode::None)?)
        .arg(InitArg::batch_mode()?)
        .arg(InitArg::no_windows()?);

    let simics = Simics::try_new(&mut init_args)?;

    Ok(())
}
