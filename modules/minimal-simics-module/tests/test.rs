use anyhow::{ensure, Result};
use confuse_simics::{setup_simics_project, SimicsApp, SimicsAppParam};
use confuse_simics_manifest::PackageNumber;
use confuse_simics_modsign::sign_simics_module;
use minimal_simics_module::HELLO_WORLD_EFI_MODULE;
use std::{
    env::var,
    fs::{copy, create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempdir::TempDir;
use test_cdylib::build_current_project;
use test_log::test;
use walkdir::WalkDir;
use whoami::username;

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

fn copy_dir_contents<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<()> {
    let src_dir = src_dir.as_ref().to_path_buf();
    ensure!(src_dir.is_dir(), "Source must be a directory");
    let dst_dir = dst_dir.as_ref().to_path_buf();

    for (src, dst) in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let src = p.path().to_path_buf();
            match src.strip_prefix(&src_dir) {
                Ok(suffix) => Some((src.clone(), dst_dir.join(suffix))),
                Err(_) => None,
            }
        })
    {
        if src.is_dir() {
            create_dir_all(&dst)?;
        } else if src.is_file() {
            copy(&src, &dst)?;
        }
    }
    Ok(())
}

#[test]
fn test_run_minimal_simics_module() -> Result<()> {
    let dylib_path = build_current_project();
    let test_dir = TempDir::new("test_minimal_simics_module_loads")?;
    let test_dir = test_dir.into_path();

    setup_simics_project(&test_dir, vec![PackageNumber::QuickStartPlatform])?;

    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let resource_dir = manifest_dir.join("resource");

    copy_dir_contents(&resource_dir, &test_dir)?;

    let hello_world_efi_module_path = test_dir.join("HelloWorld.efi");
    let mut hello_world_efi_module_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&hello_world_efi_module_path)?;

    hello_world_efi_module_file.write_all(HELLO_WORLD_EFI_MODULE)?;

    let simics_modules_dir = test_dir.join("linux64").join("lib");
    let minimal_module_path = simics_modules_dir.join("libminimal_simics_module.so");

    create_dir_all(&simics_modules_dir)?;

    copy(&dylib_path, &minimal_module_path)?;

    let simics_bin_path = test_dir.join("simics");

    let username = username();

    println!("Signing '{}'", minimal_module_path.display());
    sign_simics_module(username, &minimal_module_path)?;

    Ok(())
}
