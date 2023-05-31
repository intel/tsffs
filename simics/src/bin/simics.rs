use anyhow::{anyhow, Error, Result};
use clap::Parser;
use simics::Simics;
use simics_api::{DeprecationLevel, GuiMode, InitArg, InitArgs};
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    batch_mode: bool,
    #[arg(long)]
    deprecation_level: Option<DeprecationLevel>,
    #[arg(long)]
    expire_time: Option<String>,
    #[arg(long)]
    gui_mode: Option<GuiMode>,
    #[arg(long)]
    fail_on_warnings: bool,
    #[arg(long)]
    license_file: Option<PathBuf>,
    #[arg(long)]
    log_enable: bool,
    #[arg(long)]
    log_file: Option<PathBuf>,
    #[arg(long)]
    no_settings: bool,
    #[arg(long)]
    no_windows: bool,
    #[arg(long)]
    python_verbose: bool,
    #[arg(long)]
    project: Option<PathBuf>,
    #[arg(long)]
    quiet: bool,
    #[arg(long)]
    script_trace: bool,
    #[arg(long)]
    verbose: bool,
    /// Command or file of the form `'TYPE=VALUE'` where `TYPE` is one of
    /// `PYTHON`, `COMMAND`, or `CONFIG` and `VALUE` is a path to a file when
    /// `TYPE` is `PYTHON` or `CONFIG` and a string to run as a command otherwise
    #[arg(long)]
    command: Vec<Command>,
}

#[derive(Clone, Debug)]
enum Command {
    Command { command: String },
    Python { file: PathBuf },
    Config { config: PathBuf },
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts = s.split(':').collect::<Vec<_>>();
        match (parts.first(), parts.get(1)) {
            (Some(&"PYTHON"), Some(value)) => Ok(Command::Python {
                file: PathBuf::from(value)
                    .canonicalize()
                    .map_err(|e| anyhow!("File {} not found: {}", value, e))?,
            }),
            (Some(&"COMMAND"), Some(value)) => Ok(Command::Command {
                command: value.to_string(),
            }),
            (Some(&"CONFIG"), Some(value)) => Ok(Command::Config {
                config: PathBuf::from(value)
                    .canonicalize()
                    .map_err(|e| anyhow!("File {} not found: {}", value, e))?,
            }),
            _ => Err(anyhow!("Invalid command {}", s)),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut simics_args = InitArgs::default();

    if args.batch_mode {
        simics_args = simics_args.arg(InitArg::batch_mode()?);
    }

    if let Some(level) = args.deprecation_level {
        simics_args = simics_args.arg(InitArg::deprecation_level(level)?);
    }

    if let Some(expire_time) = args.expire_time {
        simics_args = simics_args.arg(InitArg::expire_time(expire_time)?);
    }

    if let Some(gui_mode) = args.gui_mode {
        simics_args = simics_args.arg(InitArg::gui_mode(gui_mode)?);
    }

    if args.fail_on_warnings {
        simics_args = simics_args.arg(InitArg::fail_on_warnings()?);
    }

    if let Some(license_file) = args.license_file {
        let license_file = license_file.canonicalize()?;
        let license_file_str = license_file.to_string_lossy().to_string();
        simics_args = simics_args.arg(InitArg::license_file(license_file_str)?);
    }

    if args.log_enable {
        simics_args = simics_args.arg(InitArg::log_enable()?);
    }

    if let Some(log_file) = args.log_file {
        let log_file = log_file.canonicalize()?;
        let log_file_str = log_file.to_string_lossy().to_string();
        simics_args = simics_args.arg(InitArg::log_file(log_file_str)?);
    }

    if args.no_settings {
        simics_args = simics_args.arg(InitArg::no_settings()?);
    }

    if args.no_windows {
        simics_args = simics_args.arg(InitArg::no_windows()?);
    }

    if args.python_verbose {
        simics_args = simics_args.arg(InitArg::python_verbose()?);
    }

    if let Some(project) = args.project {
        let project = project.canonicalize()?;
        let project_str = project.to_string_lossy().to_string();
        simics_args = simics_args.arg(InitArg::project(project_str)?);
    }

    if args.quiet {
        simics_args = simics_args.arg(InitArg::quiet()?);
    }

    if args.script_trace {
        simics_args = simics_args.arg(InitArg::script_trace()?);
    }

    if args.verbose {
        simics_args = simics_args.arg(InitArg::verbose()?);
    }

    let simics = Simics::try_new(&mut simics_args)?;

    for command in args.command {
        match command {
            Command::Command { command } => simics.command(command)?,
            Command::Python { file } => simics.python(file)?,
            Command::Config { config } => simics.config(config)?,
        };
    }

    Ok(())
}
