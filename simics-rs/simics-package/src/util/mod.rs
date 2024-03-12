// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Utilities helpful for packaging modules

use crate::{Error, Result};
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Recursively list all files in a directory
pub fn recursive_directory_listing<P>(directory: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    WalkDir::new(directory.as_ref())
        .into_iter()
        .filter_map(|p| p.ok())
        .map(|p| p.path().to_path_buf())
        .filter(|p| p.is_file())
        .collect::<Vec<_>>()
}

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
pub fn copy_dir_contents<P>(src_dir: P, dst_dir: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let src_dir = src_dir.as_ref().to_path_buf();

    if !src_dir.is_dir() {
        return Err(Error::NotADirectory {
            path: src_dir.clone(),
        });
    }

    let dst_dir = dst_dir.as_ref().to_path_buf();

    if !dst_dir.is_dir() {
        create_dir_all(&dst_dir)?;
    }

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
