// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simics Project
//!
//! Tools for managing simics projects, including linking to simics, loading
//! modules, and creating and destroying temporary project directories, and actually running
//! the SIMICS process after configuration

use anyhow::{anyhow, bail, Error, Result};
use derive_builder::Builder;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs::read_to_string,
    path::{Component, Path, PathBuf},
    str::FromStr,
};
use strum::{AsRefStr, Display};
use tmp_dir::{TmpDir, TmpDirBuilder};
use tracing::{debug, error};

/// CAUTION: This does not resolve symlinks (unlike
/// [`std::fs::canonicalize`]). This may cause incorrect or surprising
/// behavior at times. This should be used carefully. Unfortunately,
/// [`std::fs::canonicalize`] can be hard to use correctly, since it can often
/// fail, or on Windows returns annoying device paths. This is a problem Cargo
/// needs to improve on.
///
/// # Notes
///
/// - Taken from the `cargo` project which is Apache/MIT dual licensed
///   https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

#[derive(Debug, Clone)]
pub struct SimicsPath {
    from: Option<SimicsPathMarker>,
    to: PathBuf,
}

impl SimicsPath {
    fn new<P>(p: P, from: Option<SimicsPathMarker>) -> Self
    where
        P: AsRef<Path>,
    {
        if from.is_some() {
            let to = p.as_ref().to_path_buf().components().skip(1).collect();
            Self { from, to }
        } else {
            Self {
                from: None,
                to: p.as_ref().to_path_buf(),
            }
        }
    }

    pub fn simics<P>(p: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::new(p, Some(SimicsPathMarker::Simics))
    }

    pub fn script<P>(p: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::new(p, Some(SimicsPathMarker::Script))
    }

    pub fn path<P>(p: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::new(p, None)
    }

    pub fn canonicalize<P>(&self, base: P) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        debug!(
            "Canonicalizing {:?} on base {}",
            self,
            base.as_ref().display()
        );
        let canonicalized = match self.from {
            Some(SimicsPathMarker::Script) => bail!("Script relative paths are not supported"),
            Some(SimicsPathMarker::Simics) => {
                let p = normalize_path(
                    base.as_ref()
                        .to_path_buf()
                        .canonicalize()
                        .map_err(|e| {
                            anyhow!(
                                "Could not canonicalize base path for simics path {}: {}",
                                base.as_ref().display(),
                                e
                            )
                        })?
                        .join(&self.to),
                );
                p.starts_with(base.as_ref())
                    .then_some(p.clone())
                    .ok_or_else(|| {
                        anyhow!(
                            "Canonicalized non-simics path {} is not relative to the base path {}",
                            p.display(),
                            base.as_ref().display()
                        )
                    })?
            }
            None => {
                let p = normalize_path(&self.to);
                if p.is_absolute() {
                    p.starts_with(base.as_ref())
                        .then_some(p.clone())
                        .ok_or_else(|| {
                            anyhow!(
                            "Canonicalized non-simics path {} is not relative to the base path {}",
                            p.display(),
                            base.as_ref().display()
                        )
                        })?
                } else {
                    let p = normalize_path(
                        base.as_ref()
                            .to_path_buf()
                            .canonicalize()
                            .map_err(|e| {
                                anyhow!(
                                    "Could not canonicalize base path for simics path {}: {}",
                                    base.as_ref().display(),
                                    e
                                )
                            })?
                            .join(&self.to),
                    );
                    p.starts_with(base.as_ref())
                        .then_some(p.clone())
                        .ok_or_else(|| {
                            anyhow!(
                            "Canonicalized non-simics path {} is not relative to the base path {}",
                            p.display(),
                            base.as_ref().display()
                        )
                        })?
                }
            }
        };
        debug!(
            "Canonicalized simics path {:?} to {}",
            self,
            canonicalized.display()
        );
        Ok(canonicalized)
    }
}

impl From<PathBuf> for SimicsPath {
    fn from(value: PathBuf) -> Self {
        Self::path(value)
    }
}

impl FromStr for SimicsPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = PathBuf::from(s);
        Ok(match p.components().next() {
            Some(c) if c.as_os_str() == SimicsPathMarker::Script.as_ref() => Self::script(s),
            Some(c) if c.as_os_str() == SimicsPathMarker::Simics.as_ref() => Self::simics(s),
            _ => Self::path(PathBuf::from(s)),
        })
    }
}

impl TryFrom<&str> for SimicsPath {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        value.parse()
    }
}

#[derive(Debug, Clone, AsRefStr, Display)]
enum SimicsPathMarker {
    /// `%simics%`
    #[strum(serialize = "%simics%")]
    Simics,
    /// `%script%`
    #[strum(serialize = "%script%")]
    Script,
}

#[derive(Debug)]
pub struct ProjectPath {
    pub path: PathBuf,
    temporary: Option<TmpDir>,
}

impl ProjectPath {
    const PREFIX: &'static str = "project";

    fn default() -> Result<Self> {
        // By default, remove_on_drop is false, because if it is set to true before the launcher
        // is spawned, we will remove it twice (not good)
        let tmp = TmpDirBuilder::default()
            .prefix(Self::PREFIX)
            .remove_on_drop(false)
            .build()?;
        Ok(Self {
            path: tmp.path().to_path_buf(),
            temporary: Some(tmp),
        })
    }

    pub fn remove_on_drop(&mut self, remove_on_drop: bool) {
        if let Some(temporary) = self.temporary.as_mut() {
            temporary.remove_on_drop(remove_on_drop);
        }
    }
}

impl<P> From<P> for ProjectPath
where
    P: AsRef<Path>,
{
    fn from(value: P) -> Self {
        Self {
            path: value.as_ref().to_path_buf(),
            temporary: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PropertiesMd5Entry {
    path: String,
    // Always 'MD5'
    hash_type: String,
    hash: String,
}

impl PropertiesMd5Entry {
    pub const SEPARATOR: &'static str = "MD5";
}

impl FromStr for PropertiesMd5Entry {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let cols = s
            .split(Self::SEPARATOR)
            .map(|c| c.trim())
            .collect::<Vec<_>>();
        Ok(Self {
            path: cols
                .first()
                .ok_or_else(|| anyhow!("No path column in {}", s))?
                .to_string(),
            hash_type: Self::SEPARATOR.to_string(),
            hash: cols
                .get(1)
                .ok_or_else(|| anyhow!("No hash column in {}", s))?
                .to_string(),
        })
    }
}

pub struct PropertiesMd5 {
    _md5: HashSet<PropertiesMd5Entry>,
}

impl FromStr for PropertiesMd5 {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(Self {
            _md5: s
                .lines()
                .filter_map(|l| {
                    l.parse()
                        .map_err(|e| {
                            error!("Error parsing line {} into md5 entry", e);
                            e
                        })
                        .ok()
                })
                .collect(),
        })
    }
}

pub struct PropertiesPaths {
    _project: String,
    simics_root: String,
    _simics_model_builder: String,
    _mingw: String,
}

impl FromStr for PropertiesPaths {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let paths = s
            .lines()
            .map(|l| l.split(':').map(|l| l.trim()).collect::<Vec<_>>())
            .map(|l| {
                (
                    l.first().map(|k| k.to_string()).unwrap_or("".to_owned()),
                    l.get(1).map(|v| v.to_string()).unwrap_or("".to_owned()),
                )
            })
            .collect::<HashMap<_, _>>();
        Ok(Self {
            _project: paths
                .get("project")
                .cloned()
                .ok_or_else(|| anyhow!("No field project in {}", s))?,
            simics_root: paths
                .get("simics-root")
                .cloned()
                .ok_or_else(|| anyhow!("No field simics-root in {}", s))?,
            _simics_model_builder: paths
                .get("simics-model-builder")
                .cloned()
                .ok_or_else(|| anyhow!("No field simics-model-builder in {}", s))?,
            _mingw: paths
                .get("mingw")
                .cloned()
                .ok_or_else(|| anyhow!("No field mingw in {}", s))?,
        })
    }
}

pub struct Properties {
    _md5: PropertiesMd5,
    paths: PropertiesPaths,
}

impl TryFrom<PathBuf> for Properties {
    type Error = Error;
    fn try_from(value: PathBuf) -> Result<Self> {
        Self::try_from(&value)
    }
}
impl TryFrom<&PathBuf> for Properties {
    type Error = Error;
    fn try_from(value: &PathBuf) -> Result<Self> {
        let properties_dir = value.join(".project-properties");
        let md5_path = properties_dir.join("project-md5");
        let paths_path = properties_dir.join("project-paths");
        Ok(Self {
            _md5: read_to_string(md5_path)?.parse()?,
            paths: read_to_string(paths_path)?.parse()?,
        })
    }
}

#[derive(Builder)]
#[builder(pattern = "owned", build_fn(error = "Error"))]
pub struct Project {
    #[builder(setter(into), default = "ProjectPath::default()?")]
    /// The path to the project base directory.
    pub path: ProjectPath,
    #[builder(setter(into))]
    /// The base version constraint to use when building the project. You should never
    /// have to specify this.
    base: PathBuf,
    #[builder(setter(into))]
    /// The SIMICS Home directory. You should never need to manually specify this.
    home: PathBuf,
    #[builder(setter(each(name = "package", into), into), default)]
    packages: HashSet<PathBuf>,
}

impl Project {
    pub fn base(&self) -> &PathBuf {
        &self.base
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn packages(&self) -> &HashSet<PathBuf> {
        &self.packages
    }
}

impl TryFrom<PathBuf> for Project {
    type Error = Error;

    /// Initialize a project from an existing project on disk
    fn try_from(value: PathBuf) -> Result<Self> {
        let properties = Properties::try_from(&value)
            .map_err(|e| anyhow!("Failed to get properties from '{}': {}", value.display(), e))?;

        let simics_root = PathBuf::from(&properties.paths.simics_root);

        let home = simics_root
            .parent()
            .ok_or_else(|| anyhow!("No parent found for {}", properties.paths.simics_root))?;

        let base = PathBuf::from(properties.paths.simics_root);

        let packages = read_to_string(value.join(".package-list"))
            .unwrap_or_default()
            .lines()
            .filter(|s| !s.trim().is_empty())
            .filter_map(|l| {
                PathBuf::from(l.trim())
                    .canonicalize()
                    .map_err(|e| {
                        anyhow!("Error canonicalizing package list entry path {}: {}", l, e)
                    })
                    .ok()
            })
            .collect::<HashSet<_>>();

        Ok(Self {
            path: value.into(),
            base,
            home: home.to_path_buf(),
            packages,
        })
    }
}

impl From<Project> for ProjectBuilder {
    fn from(value: Project) -> Self {
        Self {
            path: Some(value.path),
            base: Some(value.base),
            home: Some(value.home),
            packages: Some(value.packages),
        }
    }
}

impl Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Project")
            .field("path", &self.path)
            .field("base", &self.base)
            .field("home", &self.home)
            .field("packages", &self.packages)
            .finish()
    }
}
