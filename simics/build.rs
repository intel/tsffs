extern crate num_traits;
#[macro_use]
extern crate num_derive;

use simics_api::unsafe_api::SIMICS_VERSION;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bootstrap/mod.rs"));

fn main() -> Result<()> {
    #[cfg(feature = "link")]
    {
        #[cfg(target_family = "unix")]
        link_simics_linux(format!("=={}", SIMICS_VERSION))?;

        #[cfg(not(target_family = "unix"))]
        compile_error!("Non-unix-like platforms are not yet supported");

        println!("cargo:rerun-if-changed=build.rs");
        println!(
            "cargo:rerun-if-changed={}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/bootstrap/mod.rs")
        );
    }
    Ok(())
}
