//! Utility functionality to assist managing SIMICS projects

use anyhow::{bail, ensure, Context, Error, Result};
use cargo_metadata::{MetadataCommand, Package};
use regex::Regex;
use std::{
    fs::{copy, create_dir_all},
    path::{Component, Path, PathBuf},
    str::FromStr,
};
use tracing::{debug, info};
use walkdir::WalkDir;

/// Copy the contents of one directory to another, recursively, overwriting files if they exist but
/// without replacing directories or their contents if they already exist
pub fn copy_dir_contents<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<()> {
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

/// Find a dynamic or static library that is a build result of a particular crate
pub fn find_library<S: AsRef<str>>(crate_name: S, library_type: LibraryType) -> Result<PathBuf> {
    let suffix = library_type.suffix();
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let ws_root = metadata.workspace_root;
    let workspace_metadata = MetadataCommand::new()
        .no_deps()
        .manifest_path(ws_root.join("Cargo.toml"))
        .exec()?;

    let package = workspace_metadata
        .packages
        .iter()
        .find(|p| p.name == crate_name.as_ref())
        .context(format!(
            "No package with crate name {} found",
            crate_name.as_ref()
        ))?;

    #[cfg(debug_assertions)]
    let target_subdir = "debug";
    #[cfg(not(debug_assertions))]
    let target_subdir = "release";

    let lib_paths = [
        workspace_metadata
            .target_directory
            .join(target_subdir)
            .join("deps")
            .join(format!("lib{}{}", package.name.replace('-', "_"), suffix)),
        workspace_metadata
            .target_directory
            .join(target_subdir)
            .join(format!("lib{}{}", package.name.replace('-', "_"), suffix)),
    ];

    let lib_path = lib_paths.iter().find(|p| p.is_file()).context(format!(
        "No file exists for requested module {}",
        crate_name.as_ref()
    ))?;

    info!("Found module for {} at {:?}", crate_name.as_ref(), lib_path);

    Ok(lib_path.into())
}

/// Find the [`Package`] outputs of a crate in the same workspace as the crate calling this
/// function.
pub fn find_crate<S: AsRef<str>>(crate_name: S) -> Result<Package> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let ws_root = metadata.workspace_root;
    let workspace_metadata = MetadataCommand::new()
        .no_deps()
        .manifest_path(ws_root.join("Cargo.toml"))
        .exec()?;

    Ok(workspace_metadata
        .packages
        .iter()
        .find(|p| p.name == crate_name.as_ref())
        .context(format!("No crate matching {} found", crate_name.as_ref()))?
        .clone())
}

/// Construct a relative path from a provided base directory path to the provided path.
///
/// ```rust,ignore
/// use confuse_simics_project::util::diff_paths;
/// use std::path::*;
///
/// let baz = "/foo/bar/baz";
/// let bar = "/foo/bar";
/// let quux = "/foo/bar/quux";
/// assert_eq!(diff_paths(bar, baz), Some("../".into()));
/// assert_eq!(diff_paths(baz, bar), Some("baz".into()));
/// assert_eq!(diff_paths(quux, baz), Some("../quux".into()));
/// assert_eq!(diff_paths(baz, quux), Some("../baz".into()));
/// assert_eq!(diff_paths(bar, quux), Some("../".into()));
///
/// assert_eq!(diff_paths(&baz, &bar.to_string()), Some("baz".into()));
/// assert_eq!(diff_paths(Path::new(baz), Path::new(bar).to_path_buf()), Some("baz".into()));
/// ```
pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let path = path.as_ref();
    let base = base.as_ref();

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

/// Get a relative path from a base from a relative or absolute path `file`
pub fn abs_or_rel_base_relpath<P: AsRef<Path>, S: AsRef<str>>(base: P, file: S) -> Result<PathBuf> {
    if let Ok(abspath) = PathBuf::from(file.as_ref()).canonicalize() {
        diff_paths(abspath, base)
            .context("Cannot construct relative path from absolute and base path")
    } else if let Ok(relpath) = base.as_ref().join(file.as_ref()).canonicalize() {
        diff_paths(relpath, base)
            .context("Cannot construct relative path from absolute and base path")
    } else {
        bail!(
            "File {} is not an absolute or relative path from base {}",
            file.as_ref(),
            base.as_ref().display()
        );
    }
}

/// Locate a file recursively using a regex pattern in the simics base directory. If there are
/// multiple occurrences of a filename, it is undefined which will be returned.
pub fn find_file_in_dir<P: AsRef<Path>, S: AsRef<str>>(
    simics_base_dir: P,
    file_name_pattern: S,
) -> Result<PathBuf> {
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
        .context(format!(
            "Could not find {} in {}",
            file_name_pattern.as_ref(),
            simics_base_dir.as_ref().display()
        ))?
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
