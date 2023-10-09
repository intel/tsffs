// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! High level control of SIMICS running inside the current process

use crate::api::{
    clear_exception, continue_simulation_alone, free_attribute, init_command_line,
    init_environment, init_simulator, last_error, main_loop, run_command, run_python,
    source_python, InitArgs, SimException,
};
use anyhow::{bail, Result};
use std::{env::current_exe, path::Path};
use tracing::{error, info};

pub struct Simics {}

impl Simics {
    pub fn try_init(mut args: InitArgs) -> Result<()> {
        let exe = current_exe()?;
        let argv = &[exe.to_string_lossy()];
        init_environment(argv, false, false)?;
        init_simulator(&mut args);
        Ok(())
    }

    #[allow(unreachable_code)]
    pub fn run() -> ! {
        continue_simulation_alone();
        main_loop();
        error!("Main loop exited while running simulation. This indicates a problem.");
    }

    #[allow(unreachable_code)]
    pub fn interactive() -> ! {
        init_command_line();
        main_loop();
        error!("Main loop exited while running simulation. This indicates a problem.");
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

        match clear_exception() {
            SimException::SimExc_No_Exception => Ok(()),
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
