//! The tsffs module build along with this binary

// TODO: Just use include_dir! for this now that the bug with massive compiles has been fixed
// so we can bundle it at a given version

// pub const TSFFS_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libtsffs_module.so"));
pub const TSFFS_MODULE_CRATE_NAME: &str = "tsffs_module";
pub const TSFFS_MODULE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tsffs_module/");
pub const TSFFS_WORKSPACE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

pub use tsffs_module::ModuleInterface;
