pub mod command;
pub mod path;

use anyhow::{anyhow, Error, Result};
use clap::Parser;
use command::Command;
use confuse_module::config::TraceMode;
use log::Level;
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
pub struct Args {
    #[arg(short, long)]
    /// An optional path to the SIMICS project to fuzz on disk:
    /// * If no path is specified, a temporary directory will be created and a project
    ///   will be initialized there.
    ///
    /// * If a path is specified, but it does not exist, the directory will be created and
    ///   a project will be initialized there.
    ///
    /// * If a path is specified and exists, the project there will be used.
    project: Option<PathBuf>,
    #[arg(short, long)]
    /// Command or file of the form `'TYPE=VALUE'` where `TYPE` is one of
    /// `PYTHON`, `COMMAND`, or `CONFIG` and `VALUE` is a path to a file when
    /// `TYPE` is `PYTHON` or `CONFIG` and a string to run as a command otherwise.
    ///
    /// Paths prefixed with `%simics%` will be resolved relative to the `project` directory.
    ///
    /// Most likely, you want to use this argument to specify the entrypoint to your
    /// SIMICS configuration. For example, if your SIMICS configuration is specified
    /// using a file `scripts/app.yml` in your SIMICS project that specifies a
    /// `script:`, you should use `--command CONFIG:%simics%/scripts/app.yml`
    command: Vec<Command>,
    #[arg(short, long)]
    /// Path to input corpus. If not provided, a random input corpus will be generated. This is
    /// very inefficient and not recommended for real fuzzing, but is probably ok for testing
    /// purposes.
    input: Option<PathBuf>,
    #[arg(short, long)]
    /// Path to output solutions. A `solutions` directory will be created if one does
    /// not exist.
    solutions: Option<PathBuf>,
    #[arg(short, long)]
    /// Path to in-progress test corpus. A `corpus` directory will be created if one
    /// does not exist.
    corpus: PathBuf,
    #[arg(short, long, default_value_t = Level::Error)]
    /// Logging level
    log_level: Level,
    #[arg(short = 'L', long)]
    /// Log file path to use. A new tmp file with the pattern confuse-log.XXXXX.log will be created
    /// in the project directory if no filename is specified
    log_file: Option<PathBuf>,
    #[arg(short, long, default_value_t = TraceMode::HitCount)]
    /// Mode to trace executions with
    trace_mode: TraceMode,
    #[arg(short = 'C', long, default_value_t = 1)]
    /// Number of fuzzer cores to run
    cores: usize,
    #[arg(short, long)]
    /// Whether to use a TUI for fuzzer output
    tui: bool,
    #[arg(short, long, default_value_t = true)]
    /// Whether grimoire should be used
    grimoire: bool,
}
