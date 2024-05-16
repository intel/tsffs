// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure, Result};
use ispm_wrapper::ispm::{self, GlobalOptions};
use std::{
    env::var,
    fs::read_dir,
    path::{Path, PathBuf},
};

/// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
/// exist, returns an error
pub fn subdir<P>(dir: P) -> Result<PathBuf>
where
    P: AsRef<Path>,
{
    let subdirs = read_dir(dir)?
        .filter_map(|p| p.ok())
        .map(|p| p.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    ensure!(
        subdirs.len() == 1,
        "Expected exactly 1 sub-directory, found {}",
        subdirs.len()
    );

    subdirs
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("No sub-directories found"))
}

/// Emit expected CFG directives for check-cfg feature tests
pub fn emit_expected_cfg_directives() {
    println!("cargo:rustc-check-cfg=cfg(simics_version_6)");
    println!("cargo:rustc-check-cfg=cfg(simics_version_7)");

    // We emit all the way up to 9.99.999 as expected CFG directives
    for i in 6_00_000..7_99_999 {
        println!(
            "cargo:rustc-check-cfg=cfg(simics_version_{}_{}_{})",
            i / 100_000,
            i / 1_000 % 100,
            i % 1_000
        );
    }
}

/// Emit CFG directives for the version of the Simics API being compiled against. For example,
/// simics_version_6_0_185 and simics_version_6. Both a full triple version and a major version
/// directive is emitted.
///
/// This function can be used in the `build.rs` script of a crate that depends on the `simics`
/// crate to conditionally enable experimental features in its own code.
pub fn emit_cfg_directives() -> Result<()> {
    // Set configurations to conditionally enable experimental features that aren't
    // compatible with all supported SIMICS versions, based on the SIMICS version of the
    // low level bindings.

    emit_expected_cfg_directives();

    let simics_api_version = versions::Versioning::new(simics_api_sys::SIMICS_VERSION)
        .ok_or_else(|| anyhow!("Invalid version {}", simics_api_sys::SIMICS_VERSION))?;

    // Exports a configuration directive indicating which Simics version is *compiled* against.
    println!(
        "cargo:rustc-cfg=simics_version_{}",
        simics_api_version.to_string().replace('.', "_")
    );

    println!(
        "cargo:rustc-cfg=simics_version_{}",
        simics_api_version
            .to_string()
            .split('.')
            .next()
            .ok_or_else(|| anyhow!("No major version found"))?
    );

    Ok(())
}

pub fn emit_link_info() -> Result<()> {
    #[cfg(unix)]
    const HOST_DIRNAME: &str = "linux64";

    #[cfg(not(unix))]
    const HOST_DIRNAME: &'static str = "win64";

    let base_dir_path = if let Ok(simics_base) = var("SIMICS_BASE") {
        PathBuf::from(simics_base)
    } else {
        println!("cargo:warning=No SIMICS_BASE environment variable found, using ispm to find installed packages and using latest base version");

        let mut packages = ispm::packages::list(&GlobalOptions::default())?;

        packages.sort();

        let Some(installed) = packages.installed_packages.as_ref() else {
            anyhow::bail!("No SIMICS_BASE variable set and did not get any installed packages");
        };
        let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
            anyhow::bail!(
                "No SIMICS_BASE variable set and did not find a package with package number 1000"
            );
        };
        println!("cargo:warning=Using Simics base version {}", base.version);
        base.paths
            .first()
            .ok_or_else(|| anyhow!("No paths found for package with package number 1000"))?
            .clone()
    };

    #[cfg(unix)]
    {
        // Link `libsimics-common.so`, `libvtutils.so`, and `libpythonX.XX.so.X.X` if they exist
        let bin_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("bin")
            .canonicalize()?;
        let libsimics_common = bin_dir.join("libsimics-common.so").canonicalize()?;

        let libvtutils = bin_dir.join("libvtutils.so").canonicalize()?;

        let sys_lib_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("sys")
            .join("lib")
            .canonicalize()?;

        let libpython = sys_lib_dir.join(
            read_dir(&sys_lib_dir)?
                .filter_map(|p| p.ok())
                .filter(|p| p.path().is_file())
                .filter(|p| {
                    let path = p.path();

                    let Some(file_name) = path.file_name() else {
                        return false;
                    };

                    let Some(file_name) = file_name.to_str() else {
                        return false;
                    };

                    file_name.starts_with("libpython")
                        && file_name.contains(".so")
                        && file_name != "libpython3.so"
                })
                .map(|p| p.path())
                .next()
                .ok_or_else(|| {
                    anyhow!("No libpythonX.XX.so.X.X found in {}", sys_lib_dir.display())
                })?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libsimics_common.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = [
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
        ]
        .join(":");

        println!("cargo:rustc-env=LD_LIBRARY_PATH={}", ld_library_path);
    }

    #[cfg(windows)]
    {
        // Link `libsimics-common.so`, `libvtutils.so`, and `libpythonX.XX.so.X.X` if they exist
        let bin_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("bin")
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find bin dir {:?}: {}",
                    base_dir_path.join(HOST_DIRNAME).join("bin"),
                    e
                )
            })?;

        let libsimics_common = bin_dir
            .join("libsimics-common.dll")
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find libsimics-common {:?}: {}",
                    bin_dir.join("libsimics-common.dll"),
                    e
                )
            })?;

        let libvtutils = bin_dir.join("libvtutils.dll").canonicalize().map_err(|e| {
            anyhow!(
                "Could not find libvtutils {:?}: {}",
                bin_dir.join("libvtutils.dll"),
                e
            )
        })?;

        let python_include_dir = subdir(base_dir_path.join(HOST_DIRNAME).join("include"))?;
        // .ok_or_else(|| anyhow!("Did not get any subdirectory of {:?}", base_dir_path.join(HOST_DIRNAME).join("include")))?;

        let python_dir_name = python_include_dir
            .components()
            .last()
            .ok_or_else(|| {
                anyhow!(
                    "Did not get any last component of path {:?}",
                    python_include_dir
                )
            })?
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Could not convert python include dir name to string"))?
            .to_string();

        let sys_lib_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("lib")
            .join(python_dir_name)
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find sys lib dir {:?}: {}",
                    base_dir_path.join(HOST_DIRNAME).join("sys").join("lib"),
                    e
                )
            })?;

        let libpython = sys_lib_dir.join(
            read_dir(&sys_lib_dir)?
                .filter_map(|p| p.ok())
                .filter(|p| p.path().is_file())
                .filter(|p| {
                    let path = p.path();

                    let Some(file_name) = path.file_name() else {
                        return false;
                    };

                    let Some(file_name) = file_name.to_str() else {
                        return false;
                    };

                    file_name.starts_with("python")
                        && file_name.ends_with(".dll")
                        && file_name != "python3.dll"
                })
                .map(|p| p.path())
                .next()
                .ok_or_else(|| anyhow!("No pythonX.XX.dll found in {}", sys_lib_dir.display()))?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libsimics_common.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = vec![
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
        ]
        .join(":");
    }

    // NOTE: EVEN with all of the above, a binary built using `cargo build` will not
    // be able to find libsimics-common.so. Instead, when we build a binary that
    // transitively depends on this -sys crate, we compile it with `cargo rustc`,
    // passing the `-rpath` link argument like so. Note `--disable-new-dtags`,
    // otherwise `libsimics-common.so` cannot find `libpython3.9.so.1.0` because it
    // will be missing the recursive rpath lookup.

    // SIMICS_BASE=/home/rhart/simics-public/simics-6.0.174
    // PYTHON3_INCLUDE=-I/home/rhart/simics-public/simics-6.0.174/linux64/include/python3.9
    // INCLUDE_PATHS=/home/rhart/simics-public/simics-6.0.174/src/include
    // PYTHON3_LDFLAGS=/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/libpython3.so
    // LDFLAGS="-L/home/rhart/simics-public/simics-6.0.174/linux64/bin -z noexecstack -z relro -z now" LIBS="-lsimics-common -lvtutils"
    // cargo --features=auto,link --example simple-simics -- -C link-args="-Wl,--disable-new-dtags -Wl,-rpath,/home/rhart/simics-public/simics-6.0.174/linux64/bin;/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/"
    //
    // This command (the environment variables can be left out) can be
    // auto-generated in the SIMICS makefile build system.

    Ok(())
}
