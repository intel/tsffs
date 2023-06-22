use std::{env::var, fs::copy, path::PathBuf};

use anyhow::{anyhow, Result};
use artifact_dependency::{ArtifactDependencyBuilder, CrateType, Profile};
use simics_api::sys::SIMICS_VERSION;
use simics_link::link_simics_linux;

fn main() -> Result<()> {
    #[cfg(target_family = "unix")]
    link_simics_linux(SIMICS_VERSION)?;

    #[cfg(not(target_family = "unix"))]
    compile_error!("Non-unix-like platforms are not yet supported");

    println!("cargo:rerun-if-changed=build.rs");

    let confuse_module_dep = ArtifactDependencyBuilder::default()
        .crate_name("confuse_module")
        .artifact_type(CrateType::CDynamicLibrary)
        .profile(Profile::Release)
        .build_missing(true)
        // NOTE: build_always causes a conflict on the lockfile
        // TODO: Add ability to specify a different target directory to avoid lock contention
        .build_always(false)
        .feature(SIMICS_VERSION)
        .build()
        .map_err(|e| anyhow!("Error building artifact dependency: {}", e))?
        .build()
        .map_err(|e| anyhow!("Error building artifact for dependency: {}", e))?;

    let out_dir = PathBuf::from(var("OUT_DIR")?);

    copy(
        &confuse_module_dep.path,
        out_dir.join(confuse_module_dep.path.file_name().ok_or_else(|| {
            anyhow!(
                "No file name for confuse module dependency at {}",
                confuse_module_dep.path.display()
            )
        })?),
    )?;

    Ok(())
}
