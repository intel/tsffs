use anyhow::Result;
use confuse_simics::setup_simics_project;
use confuse_simics_manifest::PackageNumber;
use tempdir::TempDir;
use test_cdylib::build_current_project;
use test_log::test;

#[test]
fn test_minimal_simics_module_exists() -> Result<()> {
    let dylib_path = build_current_project();

    assert!(dylib_path.is_file(), "No library found for module.");

    Ok(())
}

#[test]
fn test_minimal_simics_module_loads() -> Result<()> {
    let _dylib_path = build_current_project();
    let test_dir = TempDir::new("test_minimal_simics_module_loads")?;

    setup_simics_project(&test_dir, vec![PackageNumber::QuickStartPlatform])?;

    Ok(())
}
