use anyhow::Result;
use confuse_simics_manifest::PackageNumber;
use confuse_simics_project::SimicsProject;
use minimal_simics_module::HELLO_WORLD_EFI_MODULE;
use std::{env::var, path::PathBuf};
use test_cdylib::build_current_project;
use test_log::test;

#[test]
fn test_minimal_simics_module_exists() -> Result<()> {
    let dylib_path = build_current_project();

    assert!(dylib_path.is_file(), "No library found for module.");

    Ok(())
}

#[test]
fn test_load_minimal_simics_module() -> Result<()> {
    let minimal_module_path = build_current_project();
    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let resource_dir = manifest_dir.join("resource");

    let simics_project = SimicsProject::try_new()?
        .try_with_package(PackageNumber::QuickStartPlatform)?
        .try_with_contents(&resource_dir)?
        .try_with_module(&minimal_module_path)?;

    let status = simics_project
        .command()
        .arg("-batch-mode")
        .arg("-e")
        .arg("load-module minimal-simics-module")
        .status()?;

    assert!(status.success(), "Module load failed!");

    Ok(())
}
