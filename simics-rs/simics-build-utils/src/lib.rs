// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure};
use ispm_wrapper::ispm::{self, GlobalOptions};
use std::{
    env::var,
    fs::read_dir,
    path::{Path, PathBuf},
};

/// Configuration indicating that the experimental snapshots API is available (as of
/// 6.0.173)
pub const CFG_SIMICS_EXPERIMENTAL_API_SNAPSHOTS: &str = "simics_experimental_api_snapshots";
/// Configuration indicating that the experimental snapshots API is available under the
/// new `VT_take_snapshot` API name instead of the original `VT_save_snapshot` API name
/// (as of 6.0.180))
pub const CFG_SIMICS_EXPERIMENTAL_API_SNAPSHOTS_V2: &str = "simics_experimental_api_snapshots_v2";
/// Configuration indicating that SIM_log_info is deprecated and should be replaced with
/// VT_log_info until an API update
pub const CFG_SIMICS_DEPRECATED_API_SIM_LOG: &str = "simics_deprecated_api_sim_log";
/// Configuration indicating that `SIM_register_copyright` is deprecated (as of 7.0.0)
pub const CFG_SIMICS_DEPRECATED_API_SIM_REGISTER_COPYRIGHT: &str =
    "simics_deprecated_api_sim_register_copyright";
/// Configuration indicating that all rev-exec features are deprecated (as of 7.0.0)
pub const CFG_SIMICS_DEPRECATED_API_REV_EXEC: &str = "simics_deprecated_api_rev_exec";
/// Configuration indicating that the snapshots API has been stabilized and is available under
/// the name `SIM_` instead of `VT_` (as of 7.0.0)
pub const CFG_SIMICS_STABLE_API_SNAPSHOTS: &str = "simics_stable_api_snapshots";
/// Configuration indicating that the `cpu_variant_t` and `gui_mode_t` command-line options are
/// deprecated (as of 7.0.0)
pub const CFG_SIMICS_DEPRECATED_API_CPU_VARIANT_GUI_MODE: &str =
    "simics_deprecated_api_cpu_variant_gui_mode";

/// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
/// exist, returns an error
pub fn subdir<P>(dir: P) -> anyhow::Result<PathBuf>
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

/// Emit configuration directives used in the build process to conditionally enable
/// features that aren't compatible with all supported SIMICS versions, based on the
/// SIMICS version of the low level bindings. This is not needed for all consumers of the
/// API, but is useful for consumers which need to remain compatible with a wide range of
/// SIMICS base versions.
pub fn emit_cfg_directives() -> anyhow::Result<()> {
    // Set configurations to conditionally enable experimental features that aren't
    // compatible with all supported SIMICS versions, based on the SIMICS version of the
    // low level bindings.

    let simics_api_version = versions::Versioning::new(simics_api_sys::SIMICS_VERSION)
        .ok_or_else(|| anyhow::anyhow!("Invalid version {}", simics_api_sys::SIMICS_VERSION))?;

    // Conditional configurations for API versions

    if <versions::Requirement as std::str::FromStr>::from_str("<6.0.163")?
        .matches(&simics_api_version)
    {
        // Bail out if we are targeting a version before 6.0.163. We don't test any earlier than
        // this.
        panic!("Target SIMICS API version is too old. The minimum version supported is 6.0.163.");
    }

    if <versions::Requirement as std::str::FromStr>::from_str(">=6.0.177")?
        .matches(&simics_api_version)
    {
        // Deprecate (temporarily) the SIM_log APIs for versions over 6.0.177 (where the API
        // was first deprecated)
        // NOTE: This will be un-deprecated at an unspecified time in the future
        println!("cargo:rustc-cfg={CFG_SIMICS_DEPRECATED_API_SIM_LOG}");
    }

    if <versions::Requirement as std::str::FromStr>::from_str(">=6.0.173")?
        .matches(&simics_api_version)
        && <versions::Requirement as std::str::FromStr>::from_str("<6.0.180")?
            .matches(&simics_api_version)
    {
        // Enable the experimental snapshots api for versions over 6.0.173 (where the API first
        // appears)
        println!("cargo:rustc-cfg={CFG_SIMICS_EXPERIMENTAL_API_SNAPSHOTS}");
    }

    if <versions::Requirement as std::str::FromStr>::from_str(">=6.0.180")?
        .matches(&simics_api_version)
        && <versions::Requirement as std::str::FromStr>::from_str("<7.0.0")?
            .matches(&simics_api_version)
    {
        // Enable the changed snapshot API (VT_save_snapshot has been renamed to
        // VT_take_snapshot) as of 6.0.180
        println!("cargo:rustc-cfg={CFG_SIMICS_EXPERIMENTAL_API_SNAPSHOTS_V2}");
    }

    if <versions::Requirement as std::str::FromStr>::from_str(">=7.0.0")?
        .matches(&simics_api_version)
    {
        println!("cargo:rustc-cfg={CFG_SIMICS_DEPRECATED_API_SIM_REGISTER_COPYRIGHT}");
        println!("cargo:rustc-cfg={CFG_SIMICS_DEPRECATED_API_REV_EXEC}");
        println!("cargo:rustc-cfg={CFG_SIMICS_STABLE_API_SNAPSHOTS}");
        println!("cargo:rustc-cfg={CFG_SIMICS_DEPRECATED_API_CPU_VARIANT_GUI_MODE}");
    }

    Ok(())
}

pub fn emit_link_info() -> anyhow::Result<()> {
    #[cfg(unix)]
    const HOST_DIRNAME: &str = "linux64";

    #[cfg(not(unix))]
    const HOST_DIRNAME: &'static str = "win64";

    let base_dir_path = if let Ok(simics_base) = var("SIMICS_BASE") {
        PathBuf::from(simics_base)
    } else {
        println!("cargo:warning=No SIMICS_BASE environment variable found, using ispm to find installed packages and using latest base version");

        let mut packages = ispm::packages::list(&GlobalOptions::default())?;

        println!(
            "cargo:warning=Found {:?} installed packages",
            packages.installed_packages
        );

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
            .ok_or_else(|| anyhow::anyhow!("No paths found for package with package number 1000"))?
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
                    anyhow::anyhow!("No libpythonX.XX.so.X.X found in {}", sys_lib_dir.display())
                })?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow::anyhow!(
                    "No file name found for {}",
                    libsimics_common.display()
                ))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = [
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?,
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
                .ok_or_else(|| {
                    anyhow::anyhow!("No pythonX.XX.dll found in {}", sys_lib_dir.display())
                })?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow::anyhow!(
                    "No file name found for {}",
                    libsimics_common.display()
                ))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = vec![
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert path to string"))?,
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
