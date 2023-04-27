use simics_api_sys::{SIM_get_processor, SIM_get_processor_number};

use crate::OwnedMutConfObjectPtr;

pub fn get_processor_number(cpu: &OwnedMutConfObjectPtr) -> i32 {
    unsafe { SIM_get_processor_number(cpu.as_const()) }
}

pub fn get_processor(number: i32) -> OwnedMutConfObjectPtr {
    unsafe { SIM_get_processor(number) }.into()
}
