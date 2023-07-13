//! Utilities for managing simics modules, specifically adding them to a project.
//!
//! Rust Simics Modules are Rust cdylib crates that are linked into a SIMICS module to provide
//! a way to essentially write your SIMICS module in Rust, or as a hybrid of Rust and C. They may
//! also provide a SIMICS interface so the module can be interacted with from  C, DML, and
//! Python.
//!
//! A Simics Module specifies the directories for its module C source and optional
//! interface source in its `Cargo.toml`. For example, the canonical `tsffs_module`'s source
//! directory is laid out like so:
//!
//! - `tsffs_module`
//!     - `stubs`
//!         - `tsffs_module`
//!             - `tsffs_module.c`
//!             - `Makefile`
//!         - `tsffs_module-interface`
//!             - `tsffs_module-interface.h`
//!             - `tsffs_module-interface.dml`
//!             - `Makefile`
//!     - `src`
//!     - `Cargo.toml`
//!
//! The module source's only C file just defines the `init_local` function like so:
//!
//! ```c
//! void init_local(void) {
//!     tsffs_init_local();
//! }
//! ```
//!
//! This function is a stub that calls into the linked Rust cdylib that provides the actual
//! functionality of the module. The Makefile can be set up however you need, whether you need to
//! link additional libraries, specify python scripts, etc.
//!
//! A Simics Module needs to specify a few keys in its `Cargo.toml` to inform this project
//! management system how it should be set up and built.
//!
//! The table `package.metadata.tsffs` can contain the following keys:
//!
//! - `module`: [Required] A relative path from the crate root to the directory containing the
//!             `Makefile` for the module's C stub
//! - `lib`: [Required] A relative path in the SIMICS project to place the built cdylib. When the
//!         `SimicsModule` is used in a `SimicsProject`, the resulting cdylib will be placed in
//!         this location so that the module and (optionally) the interface Makefiles can locate
//!         and link against it, probably using the `rpath` linker argument. This path must be a
//!         path to a file, it cannot be a directory. Any subdirectories in the relative path
//!         will be created if they do not exist. If the extension provided is `.so`, this
//!         library will be copied from the `cdylib` crate output. If it is `.a`, this
//!         library will be copied from the `staticlib` crate output. If the crate doesn't
//!         have an output matching this field's library type, it is an error.
//! - `interface`: [Optional] A relative path from the crate root to the directory containing the
//!                `Makefile` for the interface

use crate::{project::Project, traits::Setup, util::copy_dir_contents};
use anyhow::{anyhow, ensure, Error, Result};
use artifact_dependency::Artifact;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::from_value as from_json_value;
use std::{
    fs::{copy, create_dir_all},
    path::PathBuf,
    process::{Command, Stdio},
};
use tracing::{debug, info};

#[derive(Clone, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct ModuleCargoMetadata {
    /// A relative path inside the module crate to the directory containing the module
    /// Makefile for the module
    module: String,
    /// The relative path inside a simics project where the module's library will be
    /// placed    
    lib: String,
    /// The optional relative path inside the module crate to the directory containing
    /// the interface Makefile for the module
    interface: Option<String>,
}

impl TryFrom<&Artifact> for ModuleCargoMetadata {
    type Error = Error;
    fn try_from(value: &Artifact) -> Result<Self> {
        from_json_value(
            value
                .package
                .metadata
                .get("module")
                .ok_or_else(|| anyhow!("No field 'module' in package.metadata"))?
                .clone(),
        )
        .map_err(|e| {
            anyhow!(
                "Could not extract module metadata from package artifact {:?}: {}",
                value,
                e
            )
        })
    }
}

#[derive(Builder, Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[builder(build_fn(skip))]
pub struct Module {
    #[builder(setter(skip))]
    pub metadata: ModuleCargoMetadata,
    pub artifact: Artifact,
}

impl ModuleBuilder {
    pub fn build(&self) -> Result<Module> {
        Ok(Module {
            metadata: self
                .artifact
                .as_ref()
                .map(ModuleCargoMetadata::try_from)
                .ok_or_else(|| anyhow!("No artifact set, could not extract metadata"))??,

            artifact: self
                .artifact
                .as_ref()
                .ok_or_else(|| anyhow!("No artifact set, could not create module"))
                .cloned()?,
        })
    }
}

impl Setup for Module {
    fn setup(&self, project: &Project) -> Result<&Self>
    where
        Self: Sized,
    {
        debug!(
            "Setting up module {} from {}",
            self.artifact.package.name,
            self.artifact.path.display()
        );
        let lib_target_path = project.path.path.join(&self.metadata.lib);
        lib_target_path
            .parent()
            .ok_or_else(|| anyhow!("No parent of library path {}", lib_target_path.display()))
            .and_then(|p| {
                create_dir_all(p)
                    .map_err(|e| anyhow!("Couldn't create directory {}: {}", p.display(), e))
            })
            .and_then(|_| {
                copy(&self.artifact.path, &lib_target_path).map_err(|e| {
                    anyhow!(
                        "Couldn't copy module library artifact from {} to {}: {}",
                        self.artifact.path.display(),
                        lib_target_path.display(),
                        e
                    )
                })
            })?;

        let module_src_path: PathBuf = self
            .artifact
            .package
            .manifest_path
            .parent()
            .ok_or_else(|| {
                anyhow!(
                    "No parent of package manifest path {}",
                    self.artifact.package.manifest_path
                )
            })?
            .join(&self.metadata.module)
            .into();

        let module_dir_name = module_src_path
            .components()
            .last()
            .ok_or_else(|| anyhow!("No final path component of {}", module_src_path.display()))?
            .as_os_str()
            .to_string_lossy()
            .to_string();

        let module_target_path = project.path.path.join("modules").join(&module_dir_name);

        create_dir_all(&module_target_path)?;
        copy_dir_contents(&module_src_path, &module_target_path)?;

        if let Some(interface_src_dir) = self.metadata.interface.as_ref() {
            let interface_src_path: PathBuf = self
                .artifact
                .package
                .manifest_path
                .parent()
                .ok_or_else(|| {
                    anyhow!(
                        "No parent of package manifest path {}",
                        self.artifact.package.manifest_path
                    )
                })?
                .join(interface_src_dir)
                .into();

            let interface_target_path = project.path.path.join("modules").join(
                interface_src_path.components().last().ok_or_else(|| {
                    anyhow!(
                        "No final path component of {}",
                        interface_src_path.display()
                    )
                })?,
            );
            create_dir_all(&interface_target_path)?;
            copy_dir_contents(&interface_src_path, &interface_target_path)?;
        }

        info!("Running make in project");

        let output = Command::new("make")
            .current_dir(&project.path.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        output.status.success().then_some(()).ok_or_else(|| {
            anyhow!(
                "Failed to run make:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })?;

        #[cfg(target_family = "unix")]
        let lib_build_path = project
            .path
            .path
            .join("linux64")
            .join("lib")
            .join(&module_dir_name)
            .with_extension("so");

        ensure!(
            lib_build_path.exists(),
            "Failed to build module library {}",
            lib_build_path.display()
        );

        Ok(self)
    }
}
