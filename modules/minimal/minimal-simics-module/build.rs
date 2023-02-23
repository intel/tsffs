use anyhow::{bail, ensure, Result};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use dotenvy_macro::dotenv;
use std::{
    env::var,
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
const EDK2_FEDORA35_REPO_URL: &str = "ghcr.io/tianocore/containers/fedora-35-build";
const EDK2_FEDORA35_BUILD_TAG: &str = "5b8a008";

/// Return the SIMICS_HOME directory as a PathBuf
fn simics_home() -> Result<PathBuf> {
    let simics_home = PathBuf::from(SIMICS_HOME);
    match simics_home.exists() {
        true => Ok(simics_home),
        false => {
            bail!(
                "SIMICS_HOME is defined, but {} does not exist.",
                SIMICS_HOME
            )
        }
    }
}

/// Return the OUT_DIR build directory as a PathBuf
fn out_dir() -> Result<PathBuf> {
    match var("OUT_DIR") {
        Ok(out_dir) => Ok(PathBuf::from(out_dir)),
        Err(e) => Err(e.into()),
    }
}

fn link_simics() -> Result<()> {
    let simics_bin_dir = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("bin");

    let simics_sys_lib_dir = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("sys")
        .join("lib");

    println!(
        "cargo:rustc-link-search=native={}",
        simics_bin_dir.display()
    );

    println!(
        "cargo:rustc-link-search=native={}",
        simics_sys_lib_dir.display()
    );

    println!("cargo:rustc-link-lib=simics-common");
    println!("cargo:rustc-link-lib=vtutils");
    println!("cargo:rustc-link-lib=package-paths");
    println!("cargo:rustc-link-lib=dylib:+verbatim=libpython3.9.so.1.0");

    // NOTE: This only works for `cargo run` and `cargo test` and won't work for just running
    // the output binary
    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={}",
        &format!(
            "{};{}",
            simics_bin_dir.to_string_lossy(),
            simics_sys_lib_dir.to_string_lossy(),
        )
    );
    Ok(())
}

fn copy_dir<P: AsRef<Path>>(src_dir: P, dst_dir: P) -> Result<()> {
    let src_dir = src_dir.as_ref().to_path_buf();

    ensure!(src_dir.is_dir(), "Source must be a directory");

    let dst_dir = dst_dir.as_ref().to_path_buf();
    for (src, dst) in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let src = p.path().to_path_buf();
            match src_dir.strip_prefix(&src) {
                Ok(suffix) => Some((src, dst_dir.join(suffix))),
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

fn build_efi_module() -> Result<()> {
    let edk2_dir = out_dir()?.join("edk2");
    let edk2_libc_dir = out_dir()?.join("edk2-libc");

    if !edk2_dir.join(".git").is_dir() {
        Command::new("git")
            .arg("clone")
            .arg(EDK2_GIT_REPOSITORY)
            .arg(&edk2_dir)
            .status()?;
        Command::new("git")
            .arg("-C")
            .arg(&edk2_dir)
            .arg("submodule")
            .arg("update")
            .arg("--init")
            .status()?;
    }

    if !edk2_libc_dir.join(".git").is_dir() {
        Command::new("git")
            .arg("clone")
            .arg(EDK2_LIBC_GIT_REPOSITORY)
            .arg(&edk2_libc_dir)
            .status()?;
    }

    ensure!(edk2_dir.join(".git").is_dir(), "Failed to obtain EDK2");
    ensure!(
        edk2_libc_dir.join(".git").is_dir(),
        "Failed to obtain EDK2 libc"
    );

    Command::new("make")
        .arg("-C")
        .arg(&edk2_dir.join("BaseTools"))
        .env("PYTHON_COMMAND", "python2")
        .status()?;

    Command::new("bash")
        .arg("-c")
        .arg("source")
        .arg("edksetup.sh")
        .env("PYTHON_COMMAND", "python2")
        .current_dir(&edk2_dir)
        .status()?;

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    link_simics()?;
    Ok(())
}
