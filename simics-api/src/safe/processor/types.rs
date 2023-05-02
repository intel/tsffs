use simics_api_sys::{physical_address_t, physical_block_t, x86_access_type_X86_Vanilla};

pub type PhysicalBlock = physical_block_t;
pub type PhysicalAddress = physical_address_t;

#[repr(u32)]
pub enum AccessType {
    X86Vanilla = x86_access_type_X86_Vanilla,
    // TODO: Populate
}
