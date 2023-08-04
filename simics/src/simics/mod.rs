//! High level control of SIMICS running inside the current process

use anyhow::{bail, Result};
use simics_api::{
    clear_exception, continue_simulation_alone, free_attribute, init_command_line,
    init_environment, init_simulator, last_error, main_loop, run_command, run_python,
    source_python, InitArgs, SimException,
};
use std::{env::current_exe, path::Path};
use tracing::info;

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
        continue_simulation_alone();
        main_loop()
    }

    pub fn interactive() -> ! {
        init_command_line();
        main_loop()
    }

    pub fn command<S>(command: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        info!("Running SIMICS command {}", command.as_ref());
        free_attribute(run_command(command)?);

        Ok(())
    }

    pub fn python<P>(file: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        info!("Running SIMICS Python file {}", file.as_ref().display());
        source_python(file)
    }

    pub fn config<P>(file: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        info!("Running SIMICS config {}", file.as_ref().display());

        // TODO: Figure out the C apis for doing this
        run_python(format!(
            "cli.global_cmds.run_script(script='{}')",
            file.as_ref().to_string_lossy()
        ))?;

        match clear_exception()? {
            SimException::NoException => Ok(()),
            exception => {
                bail!(
                    "Error running simics config: {:?}: {}",
                    exception,
                    last_error()
                );
            }
        }
    }
}
