//! Test that we can load TSFFS in a new project

use anyhow::Result;
use tests::test_project_x86;

#[test]
fn test_load_tsffs() -> Result<()> {
    let project_dir = test_project_x86(env!("CARGO_TARGET_TMPDIR"), "create_project")?;

    Ok(())
}
