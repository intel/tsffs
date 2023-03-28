use anyhow::Result;
use confuse_simics_module::generate_signature_header;
use confuse_simics_project::{link_simics, simics_home};
use std::{env::var, fs::OpenOptions, io::Write, path::PathBuf};

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
        .open(simics_module_header_path)?;

    write!(&mut simics_module_header, "{}", header_contents)?;

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    write_simics_constants()?;
    link_simics("*")?;
    Ok(())
}
