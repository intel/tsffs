//! The simple-simics example from SIMICS, in (unsafe) Rust

use anyhow::Result;
use raw_cstr::raw_cstr;
use simics_api_sys::{
    init_arg_t, init_arg_t__bindgen_ty_1, SIM_init_command_line, SIM_init_environment,
    SIM_init_simulator2, SIM_main_loop,
};
use std::{mem::forget, ptr::null};

fn main() -> Result<()> {
    let mut init_args = vec![
        init_arg_t {
            name: raw_cstr("quiet")?,
            boolean: true,
            u: init_arg_t__bindgen_ty_1 { enabled: false },
        },
        init_arg_t {
            name: raw_cstr("project")?,
            boolean: false,
            u: init_arg_t__bindgen_ty_1 {
                string: raw_cstr(".")?,
            },
        },
        init_arg_t {
            name: raw_cstr("gui-mode")?,
            boolean: false,
            u: init_arg_t__bindgen_ty_1 {
                string: raw_cstr("no-gui")?,
            },
        },
        init_arg_t {
            name: null(),
            boolean: false,
            u: init_arg_t__bindgen_ty_1 { string: null() },
        },
    ];
    let mut args = vec![raw_cstr("simple-simics")?];
    let args_ptr = args.as_mut_ptr();
    forget(args);

    unsafe { SIM_init_environment(args_ptr, false, true) };
    unsafe { SIM_init_simulator2(init_args.as_mut_ptr()) };
    unsafe { SIM_init_command_line() };
    unsafe { SIM_main_loop() };
    unreachable!("SIM_main_loop should never return");
}
