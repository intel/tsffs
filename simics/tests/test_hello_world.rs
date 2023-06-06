use anyhow::Result;
use simics::simics::Simics;
use simics_api::{GuiMode, InitArg, InitArgs};

#[test]
pub fn test_hello_world() -> Result<()> {
    let init_args = InitArgs::default()
        .arg(InitArg::gui_mode(GuiMode::None)?)
        .arg(InitArg::log_enable()?)
        .arg(InitArg::batch_mode()?)
        .arg(InitArg::verbose()?)
        .arg(InitArg::no_windows()?);

    let simics = Simics::try_new(init_args)?;

    simics.command("print 0x1000")?;

    Ok(())
}
