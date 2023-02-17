use anyhow::Result;
use cbindgen::Builder;
use std::{env::var, path::PathBuf};

/// Return the OUT_DIR build directory as a PathBuf
fn out_dir() -> Result<PathBuf> {
    match var("OUT_DIR") {
        Ok(out_dir) => Ok(PathBuf::from(out_dir)),
        Err(e) => Err(e.into()),
    }
}

/// Return the CARGO_MANIFEST_DIR directory as a PathBuf
fn cargo_manifest_dir() -> Result<PathBuf> {
    match var("CARGO_MANIFEST_DIR") {
        Ok(manifest_dir) => Ok(PathBuf::from(manifest_dir)),
        Err(e) => Err(e.into()),
    }
}
fn main() -> Result<()> {
    let bindings = out_dir()?.join("confuse_host_interface.h");

    Builder::new()
        .with_include_guard("CONFUSE_HOST_IF")
        .with_line_length(88)
        .with_tab_width(4)
        .with_language(cbindgen::Language::C)
        .with_documentation(true)
        .with_crate(cargo_manifest_dir()?)
        .generate()
        .expect("Unable to generate C bindings.")
        .write_to_file(&bindings);

    Ok(())
}
