//! Utilities for managing simics modules, specifically adding them to a project.
//!
//! Confuse Simics Modules are Rust cdylib crates that are linked into a SIMICS module to provide
//! a way to essentially write your SIMICS module in Rust, or as a hybrid of Rust and C. They may
//! also provide a SIMICS interface so the module can be interacted with from  C, DML, and
//! Python.
//!
//! A Confuse Simics Module specifies the directories for its module C source and optional
//! interface source in its `Cargo.toml`. For example, the canonical `confuse_module`'s source
//! directory is laid out like so:
//!
//! - `confuse_module`
//!     - `stubs`
//!         - `confuse_module`
//!             - `confuse_module.c`
//!             - `Makefile`
//!         - `confuse_module-interface`
//!             - `confuse_module-interface.h`
//!             - `confuse_module-interface.dml`
//!             - `Makefile`
//!     - `src`
//!     - `Cargo.toml`
//!
//! The module source's only C file just defines the `init_local` function like so:
//!
//! ```c
//! void init_local(void) {
//!     confuse_init_local();
//! }
//! ```
//!
//! This function is a stub that calls into the linked Rust cdylib that provides the actual
//! functionality of the module. The Makefile can be set up however you need, whether you need to
//! link additional libraries, specify python scripts, etc.
//!
//! A Confuse Simics Module needs to specify a few keys in its `Cargo.toml` to inform this project
//! management system how it should be set up and built.
//!
//! The table `package.metadata.confuse` can contain the following keys:
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

use crate::{
    project::Project,
    traits::Setup,
    util::{copy_dir_contents, find_crate, find_library, LibraryType},
};
use anyhow::{anyhow, ensure, Context, Error, Result};
use artifact_dependency::{Artifact, ArtifactDependency};
use derive_builder::Builder;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::from_value as from_json_value;
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Eq, Hash, PartialEq)]
/// Represents a simics module that can be added to a project
pub struct SimicsModule {
    /// The crate name of the crate implementing the module
    pub crate_name: String,
    /// The metadata provided in the crate's Cargo.toml file
    metadata: SimicsModuleMetadata,
    /// An absolute path to the directory containing the module Makefile for the module
    /// in the crate
    module_path: PathBuf,
    /// The relative path inside a simics project where the module's library will be
    /// placed    
    lib_relative_path: String,
    /// The optional absolute path inside the module crate to the directory containing
    /// the interface Makefile for the module
    interface_path: Option<PathBuf>,
}

#[derive(Clone, Eq, Hash, PartialEq, Debug, Deserialize)]
pub struct SimicsModuleConfuseMetadata {
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

#[derive(Clone, Eq, Hash, PartialEq, Debug, Deserialize)]
struct SimicsModuleMetadata {
    /// Confuse metadata
    confuse: SimicsModuleConfuseMetadata,
}

impl SimicsModule {
    /// Try to create a simics module that can be installed into a SIMICS project
    /// from an existing crate. The crate should contain the required metadata section
    /// of its `Cargo.toml` as mentioned above,
    pub fn try_new<S: AsRef<str>>(crate_name: S) -> Result<Self> {
        let crate_package = find_crate(crate_name.as_ref())?;
        let metadata: SimicsModuleMetadata =
            from_json_value(crate_package.metadata).context(format!(
                "Invalid or missing metadata section in metadata for crate {}",
                crate_name.as_ref()
            ))?;
        let crate_dir_path: PathBuf = crate_package
            .manifest_path
            .parent()
            .context(format!(
                "Manifest for {} has no parent directory",
                crate_name.as_ref()
            ))?
            .to_path_buf()
            .into();
        ensure!(
            crate_dir_path.is_dir(),
            "Directory for crate {} does not exist",
            crate_name.as_ref()
        );

        let module_path = crate_dir_path.join(&metadata.confuse.module);

        ensure!(
            module_path.is_dir(),
            "Directory {} for module source in crate {} does not exist",
            module_path.display(),
            crate_name.as_ref()
        );

        let interface_path = if let Some(interface) = &metadata.confuse.interface {
            let interface_path = crate_dir_path.join(interface);
            ensure!(
                interface_path.is_dir(),
                "Directory {} for interface source in crate {} does not exist",
                interface_path.display(),
                crate_name.as_ref()
            );
            Some(interface_path)
        } else {
            None
        };

        let lib_relative_path = metadata.confuse.lib.clone();

        Ok(Self {
            crate_name: crate_name.as_ref().to_string(),
            metadata,
            module_path,
            lib_relative_path,
            interface_path,
        })
    }

    /// Install the simics module to an existing Simics Project
    pub fn install<P: AsRef<Path>>(&self, simics_project_base: P) -> Result<()> {
        // First, copy the library into the project so any module or interface
        // Makefiles can find it

        let lib_path = simics_project_base
            .as_ref()
            .to_path_buf()
            .join(&self.lib_relative_path);
        let lib_dir_path = lib_path
            .parent()
            .context(format!("No parent of library path {}", lib_path.display()))?
            .to_path_buf();

        if !lib_dir_path.is_dir() {
            create_dir_all(&lib_dir_path)?;
        }

        let lib_type: LibraryType = self.metadata.confuse.lib.parse()?;

        let lib_file_path = find_library(&self.crate_name, lib_type)?;

        copy(lib_file_path, &lib_path)?;

        let module_path_dirname = self
            .module_path
            .components()
            .last()
            .context(format!(
                "No final component of module path {}",
                self.module_path.display()
            ))?
            .as_os_str()
            .to_string_lossy()
            .to_string();

        let modules_dir_path = simics_project_base.as_ref().to_path_buf().join("modules");
        let module_dir_path = modules_dir_path.join(module_path_dirname);

        create_dir_all(&module_dir_path)?;

        copy_dir_contents(&self.module_path, &module_dir_path)?;

        if let Some(interface_path) = &self.interface_path {
            let interface_path_dirname = interface_path
                .components()
                .last()
                .context(format!(
                    "No final component of interface path {}",
                    interface_path.display()
                ))?
                .as_os_str()
                .to_string_lossy()
                .to_string();
            let interface_dir_path = modules_dir_path.join(interface_path_dirname);
            create_dir_all(&interface_dir_path)?;

            copy_dir_contents(&interface_path, &&interface_dir_path)?;
        }

        Ok(())
    }
}

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
    metadata: ModuleCargoMetadata,
    artifact: Artifact,
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
