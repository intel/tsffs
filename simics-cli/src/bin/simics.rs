use anyhow::Result;
use clap::Parser;
use simics::simics::Simics;
use simics_cli::{Args, Command};

fn main() -> Result<()> {
    let args = Args::parse();
    let simics_args = Args::parse_as_init_args()?;

    let simics = Simics::try_new(simics_args)?;

    for command in args.command {
        match command {
            Command::Command { command } => simics.command(command)?,
            Command::Python { file } => simics.python(file)?,
            Command::Config { config } => simics.config(config)?,
        };
    }

    if args.interactive {
        simics.interactive();
    }

    Ok(())
}
