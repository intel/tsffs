use anyhow::Result;
use clap::Parser;
use simics::simics::Simics;
use simics_cli::{Args, Command};

fn main() -> Result<()> {
    let args = Args::parse();
    let simics_args = Args::parse_as_init_args()?;

    Simics::try_init(simics_args)?;

    for command in args.command {
        match command {
            Command::Command { command } => Simics::command(command)?,
            Command::Python { file } => Simics::python(file)?,
            Command::Config { config } => Simics::config(config)?,
        };
    }

    if args.interactive {
        Simics::interactive();
    }

    Ok(())
}
