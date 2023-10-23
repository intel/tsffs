//! Test that a project can be created

use anyhow::Result;
use tests::test_project_x86;

#[test]
fn test_create_project() -> Result<()> {
    let project_dir = test_project_x86(env!("CARGO_TARGET_TMPDIR"), "create_project")?;
    assert!(
        project_dir.is_dir() && project_dir.join("simics").is_file(),
        "Project not created"
    );
    Ok(())
}
