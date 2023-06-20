pub mod command;
pub mod project;

use anyhow::{anyhow, Error, Result};
use clap::Parser;
use command::Command;
use confuse_module::config::TraceMode;
use project::{DirectoryArg, FileArg, ModuleArg, PackageArg, PathSymlinkArg};
use std::{path::PathBuf, str::FromStr};
use tracing::Level;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(short, long)]
    /// An optional path to the SIMICS project to fuzz on disk. If no path is specified,
    /// and the current directory is a simics project, the simics project in the current
    /// directory will be used. Otherwise if no path is specified, a temporary directory
    /// will be created and a project will be initialized there.  If a path is
    /// specified, but it does not exist, the directory will be created and a project
    /// will be initialized there.  If a path is specified and exists, the project there
    /// will be used.
    pub project: Option<PathBuf>,
    #[arg(short, long, default_value_t = false)]
    /// Whether to not keep temporary projects, deleting them from disk after use. Only applies
    /// if `project` is not specified, causing an ephemeral temporary project to be created.
    pub no_keep_temp_projects: bool,
    #[arg(short, long)]
    /// Path to input corpus. If not provided, a random input corpus will be generated. This is
    /// very inefficient and not recommended for real fuzzing, but is probably ok for testing
    /// purposes.
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    /// Path to output solutions. A `solutions` directory will be created if one does
    /// not exist. The given path may be inside the project, if an existing project is used.
    pub solutions: Option<PathBuf>,
    #[arg(short, long)]
    /// Path to in-progress test corpus. A `corpus` directory will be created if one
    /// does not exist. The given path may be inside the project, if an existing project is used.
    pub corpus: Option<PathBuf>,
    #[arg(short, long, default_value_t = Level::ERROR)]
    /// Logging level
    pub log_level: Level,
    #[arg(short = 'L', long)]
    /// Log file path to use. A new tmp file with the pattern confuse-log.XXXXX.log will be created
    /// in the project directory if no filename is specified
    pub log_file: Option<PathBuf>,
    #[arg(short = 'T', long, default_value_t = TraceMode::HitCount)]
    /// Mode to trace executions with
    pub trace_mode: TraceMode,
    #[arg(short = 'C', long, default_value_t = 1)]
    /// Number of fuzzer cores to run
    pub cores: usize,
    #[arg(short, long)]
    /// Whether to use a TUI for fuzzer output
    pub tui: bool,
    #[arg(short, long, default_value_t = true)]
    /// Whether grimoire should be used
    pub grimoire: bool,
    #[arg(long)]
    // TODO: This should have an effect with existing projects
    /// Packages to add to the project. This has no effect unless a new project is being
    /// created.  Packages are specified in the form NUMBER:VERSION_CONSTRAINT (e.g.
    /// 1000:6.0.166, 1000:>=6.0.100)
    pub package: Vec<PackageArg>,
    #[arg(long)]
    // TODO: Enable modules
    /// Modules to add to the project. This has no effect unless a new project is being
    /// created. This option is not yet working.
    pub module: Vec<ModuleArg>,
    #[arg(long)]
    /// Copy a directory into the project. This has no effect unless a new project is
    /// being created. Operations are specified in the form SRC_PATH:DST_PATH (e.g.
    /// /your/package/subdirectory:%simics%/subdirectory/)
    pub directory: Vec<DirectoryArg>,
    #[arg(long)]
    /// Copy a file into the project. This has no effect unless a new project is
    /// being created. Operations are specified in the form SRC_PATH:DST_PATH (e.g.
    /// /your/package/file:%simics%/subdirectory/thefile)
    pub file: Vec<FileArg>,
    #[arg(long)]
    /// Symbolically link a file into the project. This has no effect unless a new project is
    /// being created. Operations are specified in the form SRC_PATH:DST_PATH (e.g.
    /// /your/package/file:%simics%/subdirectory/thefile)
    pub path_symlink: Vec<PathSymlinkArg>,
    #[arg(long)]
    /// Command or file of the form `'TYPE:VALUE'` where `TYPE` is one of
    /// `PYTHON`, `COMMAND`, or `CONFIG` and `VALUE` is a path to a file when
    /// `TYPE` is `PYTHON` or `CONFIG` and a string to run as a command otherwise.
    /// Paths prefixed with `%simics%` will be resolved relative to the `project` directory.
    /// Most likely, you want to use this argument to specify the entrypoint to your
    /// SIMICS configuration. For example, if your SIMICS configuration is specified
    /// using a file `scripts/app.yml` in your SIMICS project that specifies a
    /// `script:`, you should use `--command CONFIG:%simics%/scripts/app.yml`
    pub command: Vec<Command>,
}
