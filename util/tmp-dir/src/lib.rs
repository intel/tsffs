// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Temporary directory management with more explicit options than the language team crate `tmpdir`
//! and correct temporary directory permissions (user-only r-x)

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use anyhow::{anyhow, bail, ensure, Result};
use derive_builder::Builder;
#[cfg(unix)]
use libc::S_IFDIR;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

use std::{
    env::temp_dir,
    fs::{create_dir_all, remove_dir_all, set_permissions, Permissions},
    path::{Path, PathBuf},
};

#[derive(Builder, Debug, Clone)]
#[builder(build_fn(skip))]
/// Temporary directory
pub struct TmpDir {
    #[builder(setter(into))]
    /// An optional prefix to prepend to the generated identifier
    prefix: String,
    #[builder(setter(into))]
    /// An optional suffix to append to the generated identifier
    suffix: String,
    /// Whether the directory will be removed on drop. The default behavior is for the directory
    /// to be removed when the [`TmpDir`] is dropped (that is, it goes out of scope).
    remove_on_drop: bool,
    #[builder(setter(skip))]
    /// The resulting path to the temporary directory
    path: PathBuf,
    #[builder(setter(into))]
    /// The maximum number of attempts to make to create the temporary directory by generating
    /// a unique alphanumeric identifier
    tries: usize,
    #[builder(setter(into))]
    /// The number of random characters to use as the unique component of the generated directory
    /// name. Longer random strings have a higher probability of collision avoidance in exchange
    /// for higher cost to generate and less readability
    random_len: usize,
    #[builder(setter(into))]
    /// Permissions to set on the created directory. This has no effect on non-unix platforms.
    permissions: u32,
    panic_on_drop_failure: bool,
}

impl TmpDir {
    const DEFAULT_REMOVE_ON_DROP: bool = true;
    const DEFAULT_TRIES: usize = 32;
    const DEFAULT_RANDOM_LEN: usize = 8;
    const DEFAULT_PERMISSIONS: u32 = 0o40700;
    const DEFAULT_PREFIX: &'static str = "";
    const DEFAULT_SUFFIX: &'static str = "";
    const DEFAULT_PANIC_ON_DROP_FAILURE: bool = false;

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    pub fn tries(&self) -> usize {
        self.tries
    }

    pub fn random_len(&self) -> usize {
        self.random_len
    }

    pub fn permissions(&self) -> u32 {
        self.permissions
    }

    pub fn remove_on_drop(&mut self, remove_on_drop: bool) {
        self.remove_on_drop = remove_on_drop;
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        if self.remove_on_drop {
            if let Err(e) = remove_dir_all(&self.path) {
                if self.panic_on_drop_failure {
                    panic!(
                        "Failed to remove directory {} on drop: {}",
                        self.path.display(),
                        e
                    );
                }
            }
        }
    }
}

impl TmpDirBuilder {
    pub fn build(&mut self) -> Result<TmpDir> {
        #[cfg(unix)]
        let permissions =
            Permissions::from_mode(self.permissions.unwrap_or(TmpDir::DEFAULT_PERMISSIONS));
        #[cfg(unix)]
        ensure!(
            permissions.mode() & S_IFDIR != 0,
            "Permissions for directory must have directory bit ({:#o}) set (got {:#o})",
            S_IFDIR,
            permissions.mode()
        );

        #[cfg(not(unix))]
        compile_error!("Non-unix-like operating systems are not supported yet because directory permissions cannot be set securely");

        for _ in 0..self.tries.unwrap_or(TmpDir::DEFAULT_TRIES) {
            let tmpnam = format!(
                "{}tmp.{}{}",
                self.prefix
                    .as_ref()
                    .unwrap_or(&TmpDir::DEFAULT_PREFIX.to_owned()),
                thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(self.random_len.unwrap_or(TmpDir::DEFAULT_RANDOM_LEN))
                    .map(char::from)
                    .collect::<String>(),
                self.suffix
                    .as_ref()
                    .unwrap_or(&TmpDir::DEFAULT_SUFFIX.to_owned())
            );
            let tmpdir_path = temp_dir().join(tmpnam);
            if let Err(e) = create_dir_all(&tmpdir_path) {
                match e.kind() {
                    std::io::ErrorKind::AlreadyExists => {
                        continue;
                    }
                    _ => bail!(
                        "Could not create temporary directory. Unrecoverable error: {}",
                        e
                    ),
                }
            } else {
                return if let Err(e) = set_permissions(&tmpdir_path, permissions) {
                    remove_dir_all(&tmpdir_path).map_err(|ee| {
                        anyhow!("Failed to remove directory with err: {} after failing to set permissions: {}", ee, e)
                    })?;
                    Err(e.into())
                } else {
                    Ok(TmpDir {
                        prefix: self
                            .prefix
                            .as_ref()
                            .unwrap_or(&TmpDir::DEFAULT_PREFIX.to_owned())
                            .to_owned(),
                        suffix: self
                            .suffix
                            .as_ref()
                            .unwrap_or(&TmpDir::DEFAULT_PREFIX.to_owned())
                            .to_owned(),
                        remove_on_drop: self
                            .remove_on_drop
                            .unwrap_or(TmpDir::DEFAULT_REMOVE_ON_DROP),
                        path: tmpdir_path,
                        tries: self.tries.unwrap_or(TmpDir::DEFAULT_TRIES),
                        random_len: self.random_len.unwrap_or(TmpDir::DEFAULT_RANDOM_LEN),
                        permissions: self.permissions.unwrap_or(TmpDir::DEFAULT_PERMISSIONS),
                        panic_on_drop_failure: self
                            .panic_on_drop_failure
                            .unwrap_or(TmpDir::DEFAULT_PANIC_ON_DROP_FAILURE),
                    })
                };
            }
        }

        bail!(
            "Unable to generate a unique identifier in {} attempts",
            self.tries.unwrap_or(TmpDir::DEFAULT_TRIES)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_delete_on_drop_samefunc() -> Result<()> {
        let t = TmpDirBuilder::default()
            .prefix("testp")
            .suffix("tests")
            .remove_on_drop(true)
            .build()?;
        let tp = t.path().to_path_buf();
        assert!(t.path().exists(), "Tempdir does not exist");
        drop(t);
        assert!(!tp.exists(), "Tempdir should have been deleted on drop");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_permissions() -> Result<()> {
        const PERMISSIONS: u32 = 0o40755;
        let t = TmpDirBuilder::default()
            .prefix("testp")
            .suffix("tests")
            .permissions(PERMISSIONS)
            .remove_on_drop(true)
            .build()?;
        assert_eq!(
            t.path().metadata()?.permissions(),
            Permissions::from_mode(PERMISSIONS),
            "Permissions were not set correctly"
        );
        Ok(())
    }

    fn make_and_drop() -> Result<PathBuf> {
        let t = TmpDirBuilder::default()
            .prefix("testp")
            .suffix("tests")
            .remove_on_drop(true)
            .build()?;
        assert!(t.path().exists(), "Tempdir does not exist");
        let tp = t.path().to_path_buf();
        Ok(tp)
    }

    fn make_and_dont_drop() -> Result<PathBuf> {
        let t = TmpDirBuilder::default()
            .prefix("testp")
            .suffix("tests")
            .remove_on_drop(false)
            .build()?;
        assert!(t.path().exists(), "Tempdir does not exist");
        let tp = t.path().to_path_buf();
        Ok(tp)
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_delete_on_drop_other_func() -> Result<()> {
        let tp = make_and_drop()?;
        assert!(!tp.exists(), "Tempdir does not exist");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_not_delete_on_drop_other_func() -> Result<()> {
        let tp = make_and_dont_drop()?;
        assert!(tp.exists(), "Tempdir does not exist");
        remove_dir_all(&tp)?;
        Ok(())
    }

    fn make_and_return() -> Result<TmpDir> {
        let t = TmpDirBuilder::default()
            .prefix("testp")
            .suffix("tests")
            .remove_on_drop(true)
            .build()?;
        assert!(t.path().exists(), "Tempdir does not exist");
        Ok(t)
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_not_delete_on_return_other_func() -> Result<()> {
        let t = make_and_return()?;
        assert!(t.path().exists(), "Tempdir does not exist");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_delete_on_clone() -> Result<()> {
        let t = make_and_return()?;
        let tt = t.clone();
        drop(t);
        assert!(!tt.path().exists(), "Tempdir exists after clone");
        Ok(())
    }
}
