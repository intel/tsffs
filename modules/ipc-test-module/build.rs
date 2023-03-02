use anyhow::{bail, Result};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use confuse_simics_modsign::generate_signature_header;
use dotenvy_macro::dotenv;
use std::{env::var, fs::OpenOptions, io::Write, path::PathBuf};

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

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

fn write_simics_constants() -> Result<()> {
    let simics_home = simics_home()?;
    let crate_name = var("CARGO_PKG_NAME")?;
    let simics_module_header_path = out_dir()?.join("simics_module_header.rs");

    let header_contents = generate_signature_header(crate_name, simics_home)?;

    let mut simics_module_header = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&simics_module_header_path)?;

    write!(&mut simics_module_header, "{}", header_contents)?;

    Ok(())
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
    // TODO: Get this full path from the simics lib
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

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    write_simics_constants()?;
    link_simics()?;
    Ok(())
}
