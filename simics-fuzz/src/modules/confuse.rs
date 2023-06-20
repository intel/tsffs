//! The confuse module build along with this binary

// TODO: Just use include_dir! for this now that the bug with massive compiles has been fixed
// so we can bundle it at a given version

pub const CONFUSE_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libconfuse_module.so"));
pub const CONFUSE_MODULE_CRATE_NAME: &str = "confuse_module";
pub const CONFUSE_MODULE_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../modules/confuse_module/");
pub const CONFUSE_WORKSPACE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

pub use confuse_module::ConfuseModuleInterface;
