use anyhow::Result;
use simics_api::{
    call_python_module_function, free_attribute, init_command_line, init_environment,
    init_simulator, main_loop, make_attr_string_adopt, run_command, source_python,
    unsafe_api::{SIM_make_attr_list, SIMICS_VERSION},
    AttrValue, InitArg, InitArgs,
};
use std::{
    env::{current_exe, set_var},
    path::Path,
};

use crate::bootstrap::simics_home;

pub struct Simics {}

impl Simics {
    pub fn try_new(mut args: InitArgs) -> Result<Self> {
        #[cfg(target_family = "unix")]
        let python_home = simics_home()?.join("linux64");
        #[cfg(not(target_family = "unix"))]
        compile_error!("Target families other than unix-like are not supported yet");

        set_var("PYTHONHOME", python_home.to_string_lossy().to_string());

        let exe = current_exe()?;
        let argv = &[exe.to_string_lossy()];
        println!("Initializing environment");
        init_environment(argv, false, false)?;
        println!("Initializing simulator");
        init_simulator(&mut args);
        Ok(Self {})
    }

    pub fn run() -> ! {
        main_loop()
    }

    pub fn interactive() -> ! {
        init_command_line();
        main_loop()
    }

    pub fn command<S: AsRef<str>>(&self, command: S) -> Result<()> {
        free_attribute(run_command(command)?);

        Ok(())
    }

    pub fn python<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        source_python(file)
    }

    pub fn config<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        let mut args = unsafe {
            SIM_make_attr_list(1, make_attr_string_adopt(file.as_ref().to_string_lossy()))
        };

        free_attribute(call_python_module_function(
            "sim_commands",
            "cmdline_read_configuration",
            &mut args as *mut AttrValue,
        )?);

        free_attribute(args);

        Ok(())
    }
}

pub struct SimicsBinary {}

impl SimicsBinary {
    pub fn try_new<S: AsRef<str>>(name: S) -> Result<Self> {
        Ok(SimicsBinary {})
    }
}
