use confuse_simics::api::{SIM_register_class, class_data_t};

#[no_mangle]
pub extern "C" fn init_local() {
    // let class_data = class_data_t
    // let name: CString = CString::new("minimal_simics_module").expect("CString::new failed");
    
    // let cls = SIM_register_class(name.as_ptr(), class_data)
}
