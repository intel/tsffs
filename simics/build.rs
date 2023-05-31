use anyhow::Result;
use confuse_simics_project::link::link_simics;
use simics_api::unsafe_api::SIMICS_VERSION;

fn main() -> Result<()> {
    link_simics(format!("=={}", SIMICS_VERSION))?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
