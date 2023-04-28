use crate::ConfObject;
use simics_api_sys::{SIM_get_processor, SIM_get_processor_number};

pub fn get_processor_number(cpu: *mut ConfObject) -> i32 {
    unsafe { SIM_get_processor_number(cpu as *const ConfObject) }
}

pub fn get_processor(number: i32) -> *mut ConfObject {
    unsafe { SIM_get_processor(number) }
}
