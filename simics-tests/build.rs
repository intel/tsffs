use anyhow::Result;
use simics::link::link_simics_linux;
use simics_api::sys::SIMICS_VERSION;

fn main() -> Result<()> {
    #[cfg(target_family = "unix")]
    link_simics_linux(SIMICS_VERSION)?;

    #[cfg(not(target_family = "unix"))]
    compile_error!("Non-unix-like platforms are not yet supported");

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
