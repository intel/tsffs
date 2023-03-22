use std::num::Wrapping;

use anyhow::Result;
use confuse_simics_module::{
    calculate_module_checksum, get_mod_capabilities, get_mod_capabilities_data, parse_module,
    sign_simics_module_data,
};
use object::ObjectSymbol;

// A known good module that simics can load correctly
const AFL_BT_MODULE: &[u8] = include_bytes!("./resource/afl-branch-tracer.so");

#[test]
fn test_read_simics_module() -> Result<()> {
    let elf = parse_module(AFL_BT_MODULE)?;

    let mod_capabilities = get_mod_capabilities(&elf)?;

    assert_eq!(mod_capabilities.size(), 194, "Incorrect size!");

    let csum = calculate_module_checksum(&elf)?;

    assert_eq!(
        csum,
        Wrapping(0x52611201),
        "Checksum not correct: {:#x}",
        csum
    );

    Ok(())
}

#[test]
fn test_sign_simics_module() -> Result<()> {
    let orig_elf = parse_module(AFL_BT_MODULE)?;
    let orig_signature = get_mod_capabilities_data(&orig_elf)?;
    let signed_module = sign_simics_module_data("rhart", AFL_BT_MODULE)?;
    let elf = parse_module(&signed_module)?;
    let signed_signature_data = get_mod_capabilities_data(&elf)?;
    assert_ne!(
        orig_signature, signed_signature_data,
        "Signatures are the same!",
    );

    Ok(())
}
