// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Utility functionality to assist managing SIMICS projects

use anyhow::{anyhow, bail, ensure, Error, Result};
use regex::Regex;
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::debug;
use walkdir::WalkDir;

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
pub fn copy_dir_contents<P>(src_dir: P, dst_dir: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let src_dir = src_dir.as_ref().to_path_buf();
    ensure!(src_dir.is_dir(), "Source must be a directory");
    let dst_dir = dst_dir.as_ref().to_path_buf();
    if !dst_dir.is_dir() {
        create_dir_all(&dst_dir)?;
    }

    debug!(
        "Copying directory contents from {} to {}",
        src_dir.display(),
        dst_dir.display()
    );

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

pub enum LibraryType {
    Static,
    Dynamic,
}

impl FromStr for LibraryType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.ends_with(".a") {
            Ok(Self::Static)
        } else if s.ends_with(".so") {
            Ok(Self::Dynamic)
        } else {
            bail!("Unrecognized extension for library type from {}", s);
        }
    }
}

impl LibraryType {
    pub fn suffix(&self) -> String {
        match self {
            Self::Static => ".a".to_string(),
            Self::Dynamic => ".so".to_string(),
        }
    }
}

/// Locate a file recursively using a regex pattern in the simics base directory. If there are
/// multiple occurrences of a filename, it is undefined which will be returned.
pub fn find_file_in_dir<P, S>(simics_base_dir: P, file_name_pattern: S) -> Result<PathBuf>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let file_name_regex = Regex::new(file_name_pattern.as_ref())?;
    let found_file = WalkDir::new(&simics_base_dir)
        .into_iter()
        .filter_map(|de| de.ok())
        // is_ok_and is unstable ;_;
        .filter(|de| {
            if let Ok(m) = de.metadata() {
                m.is_file()
            } else {
                false
            }
        })
        .find(|de| {
            if let Some(name) = de.path().file_name() {
                file_name_regex.is_match(&name.to_string_lossy())
            } else {
                false
            }
        })
        .ok_or_else(|| {
            anyhow!(
                "Could not find {} in {}",
                file_name_pattern.as_ref(),
                simics_base_dir.as_ref().display()
            )
        })?
        .path()
        .to_path_buf();

    ensure!(
        found_file.is_file(),
        "No file {} found in {}",
        file_name_pattern.as_ref(),
        simics_base_dir.as_ref().display()
    );

    Ok(found_file)
}
