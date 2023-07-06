pub mod command;
pub mod project;

use clap::Parser;
use command::Command;
use confuse_module::config::TraceMode;
use project::{DirectoryArg, FileArg, ModuleArg, PackageArg, PathSymlinkArg};
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;

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
    #[arg(short, long, default_value_t = LevelFilter::ERROR)]
    /// Logging level
    pub log_level: LevelFilter,
    #[arg(short = 'L', long)]
    /// Log file path to use. Logger will only log to stdout if not specified
    pub log_file: Option<PathBuf>,
    #[arg(short = 'T', long, default_value_t = TraceMode::HitCount)]
    /// Mode to trace executions with
    pub trace_mode: TraceMode,
    #[arg(long, default_value_t = false)]
    /// Whether to enable the SIMICS GUI (this may make the fuzzer run much slower, but is good
    /// for debugging purposes)
    pub enable_simics_gui: bool,
    #[arg(long, default_value_t = 3.0)]
    /// Timeout in seconds (in simulated time) of a run that has timed out and will be
    /// kept as a timeout test case
    pub timeout: f64,
    #[arg(long, default_value_t = 60)]
    /// Timeout (in real time) of a fuzzer run that is running too long and the fuzzer
    /// needs to be restarted (completely re-initializing the fuzzer and simics state)
    pub executor_timeout: u64,
    #[arg(short = 'C', long, default_value_t = 1)]
    /// Number of fuzzer cores to run
    pub cores: usize,
    #[arg(short, long)]
    /// Whether to use a TUI for fuzzer output
    pub tui: bool,
    #[arg(long, default_value = PathBuf::from("/dev/null").into_os_string())]
    /// An optional path to send stdout to when running with the TUI. This option is mostly only
    /// useful if you need to debug the output from SIMICS itself (which is printed, not
    /// traced with the log macros). If you only need to capture logs to a file, you should use
    /// the `--log-file` argument instead.
    pub tui_stdout_file: PathBuf,
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
    #[arg(long)]
    /// Number of iterations to fuzz for. This is is the number of executions that will be
    /// performed. If not specified, the fuzzer will run infinitely. This argument should not
    /// be used for CI fuzzing, instead run the fuzzer with the `timeout` shell command to run
    /// for a specific amount of time.
    pub iterations: Option<u64>,
}
