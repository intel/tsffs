use anyhow::{anyhow, bail, ensure, Context, Result};
use confuse_fuzz::message::{FuzzerEvent, Message, SimicsEvent};
use confuse_simics_api::{
    attr_attr_t_Sim_Attr_Pseudo, attr_value_t, cached_instruction_handle_t, class_data_t,
    class_kind_t_Sim_Class_Kind_Session, conf_class, conf_class_t, conf_object_t,
    cpu_cached_instruction_interface_t, cpu_instruction_query_interface_t,
    cpu_instrumentation_subscribe_interface_t, instruction_handle_t, int_register_interface_t,
    obj_hap_func_t, processor_info_v2_interface_t, set_error_t, set_error_t_Sim_Set_Ok,
    SIM_attr_object_or_nil, SIM_c_get_interface, SIM_hap_add_callback, SIM_make_attr_object,
    SIM_register_attribute, SIM_register_class,
};
use const_format::concatcp;
use env_logger::init as init_logging;
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
use ipc_shm::{IpcShm, IpcShmWriter};
use lazy_static::lazy_static;
use log::{error, info};
use raw_cstr::raw_cstr;

use crate::callbacks::{get_processor, set_processor};
use crate::nonnull;

use crate::{callbacks::core_magic_instruction_cb, interface::CLASS_NAME};

use std::{
    env::var,
    ffi::{c_void, CString},
    mem::transmute,
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
};

pub struct Processor {
    cpu: *mut conf_object_t,
    cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
    cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
    cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
    processor_info_v2: *mut processor_info_v2_interface_t,
    int_register: *mut int_register_interface_t,
}

impl Processor {
    pub fn try_new(
        cpu: *mut conf_object_t,
        // For information on these interfaces, see the "Model-to-simulator interfaces" part of the
        // documentation
        cpu_instrumentation_subscribe: *mut cpu_instrumentation_subscribe_interface_t,
        cpu_instrumentation_query: *mut cpu_instruction_query_interface_t,
        cpu_cached_instruction: *mut cpu_cached_instruction_interface_t,
        processor_info_v2: *mut processor_info_v2_interface_t,
        int_register: *mut int_register_interface_t,
    ) -> Result<Self> {
        Ok(Self {
            cpu: nonnull!(cpu)?,
            cpu_instrumentation_subscribe: nonnull!(cpu_instrumentation_subscribe)?,
            cpu_instrumentation_query: nonnull!(cpu_instrumentation_query)?,
            cpu_cached_instruction: nonnull!(cpu_cached_instruction)?,
            processor_info_v2: nonnull!(processor_info_v2)?,
            int_register: nonnull!(int_register)?,
        })
    }

    pub fn get_cpu(&self) -> *mut conf_object_t {
        self.cpu
    }

    pub fn get_int_register(&self) -> *mut int_register_interface_t {
        self.int_register
    }
}
