//! The confuse module build along with this binary

pub const CONFUSE_MODULE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libconfuse_module.so"));
