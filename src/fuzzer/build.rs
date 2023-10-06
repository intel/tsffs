use anyhow::{anyhow, Result};
use std::{env::var, fs::write, iter::once, path::PathBuf};

// NOTE: The following environment variables are set under the SIMICS package make system
//
// PYTHON: The path to SIMICS's mini-python
// PYTHON3_LDFLAGS: The path to SIMICS's libpython3.so
// LIBS: `-lsimics-common -lvtutils`
// CXX_INCLUDE_PATHS: Path to SIMICS_BASE/linux64/api
// PYTHON3: The path to SIMICS's mini-python
// SIMICS_WORKSPACE: The path to the package root
// MOD_MAKEFILE: The makefile being invoked to build the module currently being built
// SIMICS_BASE: The path to the simics base version for the package/project
// DMLC: The path to dml python
// PYTHON3_INCLUDE: -I flag for the path to SIMICS's python3 include directory
// DML_INCLUDE_PATHS: The path to the dml api include directory
// SIMICS_MAJOR_VERSION: The simics major version
// SIMICS_PROJECT: The path to the simics project
// PYTHON_LDFLAGS: The path to SIMICS's libpython3.so
// MODULE_MAKEFILE: The path to the module.mk makefile in simics base
// SRC_BASE: The path to the modules directory for the package
// CCLDFLAGS_DYN: CC/LD flags for dynamic linking
// LDFLAGS: Linker flags to link to libsimics and friends
// SIMICS_PACKAGE_LIST: The path to the .package-list file for this package
// SIMICS_MODEL_BUILDER: The simics used to build models
// PYTHON_INCLUDE: -I flag for the path to SIMICS's python3 include directory
// DMLC_DIR: The path to the simics bin directory containing the dml compiler
// PY2TO3: Path to the py-2to3 tool
// LDFLAGS_PY3: -L flag to include simics base's bin directory
// INCLUDE_PATHS: The path to the SIMICS base include directory

/// Name for the environment variable set by the SIMICS build system to the libpython3.so library
const PYTHON3_LDFLAGS_ENV: &str = "PYTHON3_LDFLAGS";
/// Name for the LDFLAGS environment variable set by the SIMICS build system containing
/// the link search path for the libsimics library, among other flags. e.g. -LPATH -z noexecstack
const LDFLAGS_ENV: &str = "LDFLAGS";

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    if cfg!(feature = "link") {
        let libpython_path =
            PathBuf::from(var(PYTHON3_LDFLAGS_ENV).map_err(|e| {
                anyhow!("No environment variable {PYTHON3_LDFLAGS_ENV} found: {e}")
            })?);

        let libpython_dir = libpython_path
            .parent()
            .ok_or_else(|| anyhow!("libpython path {} has no parent", libpython_path.display()))?
            .to_path_buf();

        let link_search_paths = var(LDFLAGS_ENV)
            .map_err(|e| anyhow!("No environment variable {LDFLAGS_ENV} found: {e}"))?
            .split_whitespace()
            .filter_map(|s| s.starts_with("-L").then_some(s.replace("-L", "")))
            .map(|p| PathBuf::from(p))
            .chain(once(libpython_dir))
            .collect::<Vec<_>>();

        link_search_paths.iter().for_each(|p| {
            println!("cargo:rustc-link-search=native={}", p.display());
            println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}", p.display());
        });

        let library_search_paths = link_search_paths
            .iter()
            .map(|p| {
                p.to_str()
                    .ok_or_else(|| anyhow!("Could not convert path {} to string", p.display()))
            })
            .collect::<Result<Vec<_>>>()?
            .join(":");

        // NOTE: This enables running binaries linked with this one when running with `cargo run`
        println!("cargo:rustc-env=LD_LIBRARY_PATH={}", library_search_paths);
    }

    Ok(())
}
