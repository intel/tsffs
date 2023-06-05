use anyhow::Result;
use clap::ValueEnum;
use paste::paste;
use raw_cstr::raw_cstr;
use simics_api_sys::{
    cpu_variant_t_Cpu_Any, cpu_variant_t_Cpu_Fast, cpu_variant_t_Cpu_None, cpu_variant_t_Cpu_Stall,
    gui_mode_t_GUI_Mode_Default, gui_mode_t_GUI_Mode_Mixed, gui_mode_t_GUI_Mode_None,
    gui_mode_t_GUI_Mode_Only, init_arg_t, init_arg_t__bindgen_ty_1, SIM_init_command_line,
    SIM_init_environment, SIM_init_simulator2, SIM_main_loop,
};
use std::ptr::null;

#[derive(Copy, Clone, Debug, ValueEnum)]
#[repr(u32)]
pub enum GuiMode {
    None = gui_mode_t_GUI_Mode_None,
    Mixed = gui_mode_t_GUI_Mode_Mixed,
    Only = gui_mode_t_GUI_Mode_Only,
    Default = gui_mode_t_GUI_Mode_Default,
}

impl ToString for GuiMode {
    fn to_string(&self) -> String {
        match self {
            GuiMode::None => "no-gui",
            GuiMode::Mixed => "mixed",
            GuiMode::Only => "only",
            GuiMode::Default => "no-gui",
        }
        .to_string()
    }
}

#[repr(u32)]
pub enum CpuVariant {
    None = cpu_variant_t_Cpu_None,
    Any = cpu_variant_t_Cpu_Any,
    Fast = cpu_variant_t_Cpu_Fast,
    Stall = cpu_variant_t_Cpu_Stall,
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
                pub fn [< $name:snake:lower >]() -> Result<Self> {
                    Self::boolean($name, true)
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
    pub fn boolean<S: AsRef<str>>(name: S, enabled: bool) -> Result<Self> {
        Ok(InitArg(init_arg_t {
            name: raw_cstr(name)?,
            boolean: true,
            u: init_arg_t__bindgen_ty_1 { enabled },
        }))
    }

    pub fn string<S: AsRef<str>>(name: S, value: S) -> Result<Self> {
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

pub fn init_environment<I, S>(argv: I, handle_signals: bool, allow_core_dumps: bool) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = argv
        .into_iter()
        .filter_map(|s| raw_cstr(s).ok())
        .collect::<Vec<_>>();
    unsafe { SIM_init_environment(args.as_mut_ptr(), handle_signals, allow_core_dumps) };
    Ok(())
}

pub fn init_simulator(args: &mut InitArgs) {
    unsafe { SIM_init_simulator2(args.as_mut_ptr()) };
}

pub fn init_command_line() {
    unsafe { SIM_init_command_line() };
}

pub fn main_loop() -> ! {
    unsafe { SIM_main_loop() };
    unreachable!("Something went wrong initializing the SIMICS main loop")
}
