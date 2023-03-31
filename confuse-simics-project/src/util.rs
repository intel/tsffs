//! Utility functionality to assist managing SIMICS projects

use anyhow::{ensure, Context, Result};
use cargo_metadata::MetadataCommand;
use log::info;
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
pub fn copy_dir_contents<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<()> {
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

enum LibraryType {
    Static,
    Dynamic,
}

fn find_library<S: AsRef<str>>(crate_name: S, library_type: LibraryType) -> Result<PathBuf> {
    let suffix = match library_type {
        LibraryType::Dynamic => "so",
        LibraryType::Static => "a",
    };
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let ws_root = metadata.workspace_root;
    let workspace_metadata = MetadataCommand::new()
        .no_deps()
        .manifest_path(ws_root.join("Cargo.toml"))
        .exec()?;

    let target = workspace_metadata
        .packages
        .iter()
        .filter(|p| p.name == crate_name.as_ref() && p.targets.iter().any(|t| t.is_lib()))
        .filter_map(|p| p.targets.iter().find(|t| t.is_lib()))
        .take(1)
        .next()
        .context("No package with given crate name.")?;

    #[cfg(debug_assertions)]
    let target_subdir = "debug";
    #[cfg(not(debug_assertions))]
    let target_subdir = "release";

    let lib_paths = [
        workspace_metadata
            .target_directory
            .join(target_subdir)
            .join("deps")
            .join(format!("lib{}.{}", target.name.replace('-', "_"), suffix)),
        workspace_metadata
            .target_directory
            .join(target_subdir)
            .join(format!("lib{}.{}", target.name.replace('-', "_"), suffix)),
    ];

    let lib_path = lib_paths.iter().find(|p| p.is_file()).context(format!(
        "No file exists for requested module {}",
        crate_name.as_ref()
    ))?;

    info!("Found module for {} at {:?}", crate_name.as_ref(), lib_path);

    Ok(lib_path.into())
}

/// Find a dynamic cdylib build artifact by crate name. This only works for crates in the
/// current workspace.
/// This isn't 100% foolproof but should work consistently for targets/fuzzers set up like the
/// examples given in the Confuse workspace
pub fn find_dynamic_library<S: AsRef<str>>(crate_name: S) -> Result<PathBuf> {
    find_library(crate_name, LibraryType::Dynamic)
}

/// Find a staticlib build artifact by crate name. This only works for crates in the
/// current workspace.
/// This isn't 100% foolproof but should work consistently for targets/fuzzers set up like the
/// examples given in the Confuse workspace
pub fn find_static_library<S: AsRef<str>>(crate_name: S) -> Result<PathBuf> {
    find_library(crate_name, LibraryType::Static)
}

pub fn find_crate_dir<S: AsRef<str>>(crate_name: S) -> Result<PathBuf> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let ws_root = metadata.workspace_root;
    let workspace_metadata = MetadataCommand::new()
        .no_deps()
        .manifest_path(ws_root.join("Cargo.toml"))
        .exec()?;

    Ok(workspace_metadata
        .packages
        .iter()
        .find(|p| p.name == crate_name.as_ref() && p.targets.iter().any(|t| t.is_lib()))
        .context(format!("No crate matching {} found", crate_name.as_ref()))?
        .manifest_path
        .parent()
        .context(format!(
            "Manifest for crate {} has no parent directory",
            crate_name.as_ref()
        ))?
        .to_path_buf()
        .into())
}
