#![deny(clippy::unwrap_used)]

pub const HELLO_WORLD_EFI_MODULE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/HelloWorld.efi"));
