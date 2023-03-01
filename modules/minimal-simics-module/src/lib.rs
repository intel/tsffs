//! Small guide on how to trick simics into loading rust code.

include!(concat!(env!("OUT_DIR"), "/simics_module_header.rs"));

use confuse_simics_api::{class_data_t, class_kind_t_Sim_Class_Kind_Session, SIM_register_class};
use std::ffi::CString;

pub const HELLO_WORLD_EFI_MODULE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/HelloWorld.efi"));

#[no_mangle]
pub extern "C" fn init_local() {
    let class_name: CString = CString::new(CLASS_NAME).expect("CString::new failed");
    let class_data_desc = CString::new("Minimal module").expect("CString::new failed");
    let class_data_class_desc = CString::new("Minimal module class").expect("CString::new failed");

    let class_data = class_data_t {
        alloc_object: None,
        init_object: None,
        finalize_instance: None,
        pre_delete_instance: None,
        delete_instance: None,
        description: class_data_desc.into_raw(),
        class_desc: class_data_class_desc.into_raw(),
        kind: class_kind_t_Sim_Class_Kind_Session,
    };

    let _cls =
        unsafe { SIM_register_class(class_name.into_raw(), &class_data as *const class_data_t) };

    println!("Module initialized!");
}
