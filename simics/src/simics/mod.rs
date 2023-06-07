use anyhow::Result;
use simics_api::{
    call_python_module_function, free_attribute, init_command_line, init_environment,
    init_simulator, main_loop, make_attr_string_adopt, run_command, source_python,
    sys::SIM_make_attr_list, AttrValue, InitArgs,
};
use std::{env::current_exe, path::Path};

pub mod home;

pub struct Simics {}

impl Simics {
    pub fn try_init(mut args: InitArgs) -> Result<()> {
        let exe = current_exe()?;
        let argv = &[exe.to_string_lossy()];
        init_environment(argv, false, false)?;
        init_simulator(&mut args);
        Ok(())
    }

    pub fn run() -> ! {
        main_loop()
    }

    pub fn interactive() -> ! {
        init_command_line();
        main_loop()
    }

    pub fn command<S: AsRef<str>>(command: S) -> Result<()> {
        free_attribute(run_command(command)?);

        Ok(())
    }

    pub fn python<P: AsRef<Path>>(file: P) -> Result<()> {
        source_python(file)
    }

    pub fn config<P: AsRef<Path>>(file: P) -> Result<()> {
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
