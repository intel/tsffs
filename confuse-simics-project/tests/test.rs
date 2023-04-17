use anyhow::Result;
use confuse_simics_project::SimicsProject;

#[test]
fn test_add_module() -> Result<()> {
    let mut project = SimicsProject::try_new_latest()?.try_with_module("confuse_module")?;
    project.persist();

    let _ = project.build()?;

    Ok(())
}
