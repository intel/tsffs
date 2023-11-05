// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Functionality for embedding SIMICS into a program or library. These functions are useful for
//! creating alternate SIMICS frontends

use crate::{
    api::sys::{
        cpu_variant_t, gui_mode_t, init_arg_t, init_arg_t__bindgen_ty_1, SIM_init_command_line,
        SIM_init_environment, SIM_init_simulator2, SIM_main_loop,
    },
    Result,
};
use clap::ValueEnum;
use paste::paste;
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::{mem::forget, ptr::null};

#[derive(Debug, Clone)]
pub struct GuiMode(gui_mode_t);

#[derive(Debug, Clone)]
pub struct CpuVariant(cpu_variant_t);

impl ToString for GuiMode {
    fn to_string(&self) -> String {
        match self.0 {
            gui_mode_t::GUI_Mode_None => "no-gui",
            gui_mode_t::GUI_Mode_Mixed => "mixed",
            gui_mode_t::GUI_Mode_Only => "gui",
            gui_mode_t::GUI_Mode_Default => "default",
        }
        .to_string()
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[repr(u32)]
/// Level of warning that will be printed when deprecated APIs are used
pub enum DeprecationLevel {
    NoWarnings = 0,
    MajorReleaseDeprecated = 1,
    NewAndFutureDeprecated = 2,
}

impl ToString for DeprecationLevel {
    fn to_string(&self) -> String {
        let val: u32 = *self as u32;
        val.to_string()
    }
}

/// Initialization arguments. See:
/// https://simics-download.pdx.intel.com/simics-6/docs/html/rm-base/simics.html
#[derive(Clone)]
pub struct InitArg(init_arg_t);

impl From<InitArg> for init_arg_t {
    fn from(value: InitArg) -> Self {
        value.0
    }
}

macro_rules! impl_bool_arg {
    ($struct_name:ident, $name:expr) => {
        paste! {
            impl $struct_name {
                pub fn [< $name:snake:lower >](value: bool) -> Result<Self> {
                    Self::boolean($name, value)
                }
            }
        }
    };
}

macro_rules! impl_string_arg {
    ($struct_name:ident, $name:expr) => {
        paste! {
            impl $struct_name {
                pub fn [< $name:snake:lower >]<T>(value: T) -> Result<Self> where T: ToString {
                    let value = value.to_string();
                    Self::string($name, &value.to_string())

                }
            }
        }
    };
}

impl InitArg {
    pub fn boolean<S>(name: S, enabled: bool) -> Result<Self>
    where
        S: AsRef<str>,
    {
        Ok(InitArg(init_arg_t {
            name: raw_cstr(name)?,
            boolean: true,
            u: init_arg_t__bindgen_ty_1 { enabled },
        }))
    }

    pub fn string<S>(name: S, value: S) -> Result<Self>
    where
        S: AsRef<str>,
    {
        Ok(InitArg(init_arg_t {
            name: raw_cstr(name)?,
            boolean: false,
            u: init_arg_t__bindgen_ty_1 {
                string: raw_cstr(value)?,
            },
        }))
    }

    pub fn last() -> Self {
        InitArg(init_arg_t {
            name: null(),
            boolean: false,
            u: init_arg_t__bindgen_ty_1 { string: null() },
        })
    }
}

// See
// https://simics-download.pdx.intel.com/simics-6/docs/html/reference-manual-api/simulator-api-functions.html#SIM_init_simulator2
// for the list of these pre-defined parameters.

impl_bool_arg!(InitArg, "batch-mode");
impl_string_arg!(InitArg, "deprecation-level");
impl_string_arg!(InitArg, "expire-time");
impl_string_arg!(InitArg, "gui-mode");
impl_bool_arg!(InitArg, "fail-on-warnings");
impl_string_arg!(InitArg, "license-file");
impl_bool_arg!(InitArg, "log-enable");
impl_string_arg!(InitArg, "log-file");
impl_bool_arg!(InitArg, "no-settings");
impl_bool_arg!(InitArg, "no-windows");
impl_bool_arg!(InitArg, "python-verbose");
impl_string_arg!(InitArg, "project");
impl_bool_arg!(InitArg, "quiet");
impl_bool_arg!(InitArg, "script-trace");
impl_bool_arg!(InitArg, "verbose");

// Intenal/deprecated options

impl_bool_arg!(InitArg, "allow-license-gui");
impl_string_arg!(InitArg, "alt-settings-dir");
impl_string_arg!(InitArg, "application-mode");
impl_bool_arg!(InitArg, "check-ifaces");
impl_bool_arg!(InitArg, "disable-dstc");
impl_bool_arg!(InitArg, "disable-istc");
impl_string_arg!(InitArg, "eclipse-params");
impl_string_arg!(InitArg, "package-list");
impl_bool_arg!(InitArg, "py3k-warnings");
impl_bool_arg!(InitArg, "sign-module");
impl_bool_arg!(InitArg, "as-py-module");
impl_bool_arg!(InitArg, "py-import-all");
impl_bool_arg!(InitArg, "use-module-cache");

#[derive(Clone)]
pub struct InitArgs {
    args: Vec<init_arg_t>,
}

impl Default for InitArgs {
    fn default() -> Self {
        Self {
            args: vec![InitArg::last().into()],
        }
    }
}

impl InitArgs {
    pub fn arg(mut self, arg: InitArg) -> Self {
        self.args.insert(0, arg.into());
        self
    }

    pub fn as_mut_ptr(&mut self) -> *mut init_arg_t {
        self.args.as_mut_ptr()
    }
}

#[simics_exception]
/// Initialize the environment (for SIMICS frontends)
pub fn init_environment<I, S>(argv: I, handle_signals: bool, allow_core_dumps: bool) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = Vec::new();

    for arg in argv {
        args.push(raw_cstr(arg)?);
    }

    let args_ptr = args.as_mut_ptr();

    forget(args);

    unsafe { SIM_init_environment(args_ptr, handle_signals, allow_core_dumps) };

    Ok(())
}

#[simics_exception]
/// Initialize the simulator with arguments.
pub fn init_simulator(args: &mut InitArgs) {
    unsafe { SIM_init_simulator2(args.as_mut_ptr()) };
}

#[simics_exception]
/// Initialize the SIMICS command line. [`main_loop`] needs to be called next otherwise the
/// command line will exit immediately.
pub fn init_command_line() {
    unsafe { SIM_init_command_line() };
}

/// Pass control to SIMICS and block until SIMICS exits. This is typically called after
/// [`init_command_line`].
pub fn main_loop() -> ! {
    unsafe { SIM_main_loop() };
    unreachable!("Something went wrong initializing the SIMICS main loop")
}
