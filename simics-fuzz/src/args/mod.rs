// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

pub mod command;
pub mod project;

use clap::Parser;
use command::Command;
use project::{DirectoryArg, FileArg, PackageArg, PathSymlinkArg};
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;
use tsffs_module::config::TraceMode;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short = 'p', long)]
    /// Optional SIMICS project path to use for fuzzing
    ///
    /// If -p/--project is specified and the path already exists, the existing project
    /// there will be used. If -p/--project is specified and the path does not exist, a
    /// directory will be created at the path and a new project will be initialized in
    /// it.
    ///
    /// If -p/--project is NOT specified and the current working directory is a SIMICS
    /// project, the SIMICS project in the current working directory will be used. If
    /// -p/--project is NOT specified, and the current working directory is not a SIMICS
    /// project, a new temporary directory will be created and a new project will be
    /// initialized in it. In this case, --no-keep-temp-projects may be used to
    /// automatically clean up the temporary project.
    pub project: Option<PathBuf>,
    #[arg(short = 'N', long, default_value_t = false)]
    /// Delete temporary SIMICS projects on exit
    ///
    /// If specified, temporary projects will be deleted from disk after the fuzzer
    /// exists. This option only applies if -p/--project is not specified and the
    /// current working directory is not a SIMICS project causing a temporary SIMICS
    /// project to be created. By default, temporary projects are kept for debugging and
    /// triage purposes, but keeping them may not be desired in testing or CI scenarios.
    pub no_keep_temp_projects: bool,
    #[arg(short = 'i', long)]
    /// Optional input corpus path
    ///
    /// If not provided, a random input corpus of printable characters will be
    /// generated. This is very inefficient and not recommended for real fuzzing, but is
    /// useful for testing and demonstration purposes.
    pub input: Option<PathBuf>,
    #[arg(short = 'o', long)]
    /// Optional output solution/objective path
    ///
    /// Solutions (or objectives) will be stored in this directory as they are found by
    /// the fuzzer. If not provided, a directory 'solutions' will be created in the
    /// current working directory.  The given path may be inside the project, if an
    /// existing project is used.
    pub solutions: Option<PathBuf>,
    #[arg(short = 'c', long)]
    /// Optional in-progress corpus path
    ///
    /// The working corpus will be stored in this directory as new interesting inputs
    /// are found by the fuzzer. If not provided, a directory 'corpus' will be created
    /// in the current working directory. The given path may be inside the project, if
    /// an existing project is used.
    pub corpus: Option<PathBuf>,
    #[arg(short = 'l', long, default_value_t = LevelFilter::ERROR)]
    /// Output log level
    ///
    /// Logging level may be set to ERROR, WARN, INFO, DEBUG, or TRACE
    pub log_level: LevelFilter,
    #[arg(short = 'T', long, default_value_t = TraceMode::HitCount)]
    /// Branch tracing mode
    ///
    /// Specifies whether 'hit_count' or 'once' branch tracing mode should be used. In 'hit_count'
    /// mode, every execution of every instruction is traced, meaning slower but more accurate
    /// fuzzer executions. 'once' mode traces only the first execution of each instruction, which
    /// is much faster but less precise, particularly when fuzzing code with loops.
    pub trace_mode: TraceMode,
    #[arg(short = 'S', long, default_value_t = 3.0)]
    /// Simulator-time timeout, in seconds, for timed out test cases
    ///
    /// If this timeout is exceeded in simulated time for a single run, the input
    /// testcase will be treated as causing a timeout in the target software. For most
    /// purposes, 3-5 seconds is ideal, but for very complex target software a higher
    /// timeout may be desired. Keep in mind that in many cases virtual time runs
    /// *faster* than real wall-clock time, so test cases may report as timed out even
    /// if they do not execute for the entire timeout in real-world wall clock time.
    pub timeout: f64,
    #[arg(short = 'E', long, default_value_t = 60)]
    /// Executor timeout, in seconds, for stuck executor detection
    ///
    /// If this timeout is exceeded in real-world wall clock time, the executor will be treated as
    /// stuck and will be restarted. This can happen in some rare cases such as invalid executor
    /// state, uncaught SIMICS exception, memory leak in SIMICS or the running model(s),
    /// and so forth. Some of these conditions are recoverable with a restart of the
    /// executor, which will re-start the SIMICS instance from scratch.
    ///
    /// Some targets require significant time to start up, in which case this value may need to be
    /// increased. For example, modern server platforms require many minutes to boot into Linux on
    /// the simulated platform, so this timeout must be long enough to allow the model to start up
    /// and run the initial start harness.
    pub executor_timeout: u64,
    #[arg(short = 'C', long, default_value_t = 1)]
    /// Number of fuzzer cores to run
    ///
    /// For each fuzzer core, a new fuzzer client will start with its own instance of SIMICS and
    /// its own fuzzer. These fuzzers will synchronize with each other and use the same corpus
    /// to parallelize and speed up fuzzing. The number of fuzzers should be no more than the
    /// number of physical CPU cores on the host machine, as each client process spawns multiple
    /// threads.
    pub cores: usize,
    #[arg(short = 't', long)]
    /// Enable the TUI
    ///
    /// The TUI provides less visibility into fuzzing state, but is familiar to most
    /// users. Enabling the TUI implicitly sets the log level to ERROR.
    pub tui: bool,
    #[arg(short = 'O', long, default_value = PathBuf::from("/dev/null").into_os_string())]
    /// Optional file to send stdout (logs and output) to when using TUI
    ///
    /// When running the TUI, logs cannot be printed but may be useful. This option
    /// allows you to send all stdout output (logging and all other output) to a file
    /// while using the TUI, for example to view the log with tail -F <TUI_STDOUT_FILE>
    /// while viewing the TUI output in a separate terminal.
    pub tui_stdout_file: PathBuf,
    #[arg(short = 'D', long)]
    /// An optional token file in 'id = "token data"' format
    ///
    /// A token file can be provided with pre-extracted tokens from a target, for example by using
    /// the LLVM dict2file pass. This token file will be leveraged for token mutations during
    /// fuzzing. If both --tokens-file and --executable are passed, both will be used.
    pub tokens_file: Option<PathBuf>,
    #[arg(short = 'e', long)]
    /// An optional path to the executable file for the target software
    ///
    /// When provided, the path to the executable file will be used for several optional
    /// analyses that improve fuzzing, including token extraction, function call
    /// analysis for Redqueen's function call tracing mode, and more. These techniques
    /// will be used automatically when they are possible and the executable path is
    /// given.
    pub executable: Option<PathBuf>,
    #[arg(short = 'P', long)]
    /// Add a package to the working project
    ///
    /// This option can be specified multiple times. This has no effect unless a new
    /// project is being Packages are specified in the form NUMBER:VERSION_CONSTRAINT
    /// (e.g. 1000:6.0.167, 1000:>=6.0.163)
    pub package: Vec<PackageArg>,
    #[arg(short = 'd', long)]
    /// Copy a directory into the project
    ///
    /// Recursively (as in cp -a) copies a directory into the SIMICS project in use.
    /// This argument is specified in the form SRC_PATH:DST_PATH. For example,
    /// --directory '/your/package/subdirectory:%simics%/subdirectory/', where
    /// '/your/package/subdirectory' will be copied to the 'subdirectory' subdirectory
    /// of the SIMICS project in use. If intermediate subdirectories do not yet exist,
    /// they will be created. This argument can be specified multiple times.
    pub directory: Vec<DirectoryArg>,
    #[arg(short = 'f', long)]
    /// Copy a file into the project
    ///
    /// Recursively (as in cp -a) copies a file into the SIMICS project in use. This
    /// argument is specified in the form SRC_PATH:DST_PATH. For example, --file
    /// '/your/package/file:%simics%/subdirectory/thefile', where '/your/package/file'
    /// will be copied to the 'subdirectory' subdirectory of the SIMICS project in use.
    /// If intermediate subdirectories do not yet exist, they will be created. This
    /// argument can be specified multiple times.
    pub file: Vec<FileArg>,
    #[arg(short = 's', long)]
    /// Symbolically link a file or directory into the project
    ///
    /// Symbolically a file or directory into the SIMICS project in use. This argument
    /// is specified in the form SRC_PATH:DST_PATH. For example, --path-symlink
    /// '/your/package/file:%simics%/subdirectory/thefile', where '/your/package/file'
    /// will be linked into to the 'subdirectory' subdirectory of the SIMICS project in
    /// use.  If intermediate subdirectories do not yet exist, they will be created.
    pub path_symlink: Vec<PathSymlinkArg>,
    #[arg(short = 'x', long)]
    /// Python script, command, or startup configuration to execute on startup
    ///
    /// Command or file of the form `'TYPE:VALUE'` where `TYPE` is one of `PYTHON`,
    /// `COMMAND`, or `CONFIG` and `VALUE` is a path to a file when `TYPE` is `PYTHON`
    /// or `CONFIG` and a string to run as a command otherwise.  Paths prefixed with
    /// `%simics%` will be resolved relative to the `project` directory.  Most likely,
    /// you want to use this argument to specify the entrypoint to your SIMICS
    /// configuration. For example, if your SIMICS configuration is specified using a
    /// file `scripts/app.yml` in your SIMICS project that specifies a `script:`, you
    /// should use `--command CONFIG:%simics%/scripts/app.yml`
    pub command: Vec<Command>,
    #[arg(short = 'r', long)]
    /// Enter repro mode and reproduce a run of a given solution on the target software
    ///
    /// Reproduce a solution on a target. When specified, SIMICS will be run with the
    /// module installed, and you will be dropped into the SIMICS REPL at the crash
    /// location with a reverse execution recording from the start harness. Typically,
    /// the same command used to run the fuzzer should be used to run repro mode, with
    /// the addition of this flag.
    ///
    /// For example, --repro 'solutions/008e7aaa8871b4a8' after finding the solution with the
    /// fuzzer will drop into a SIMICS REPL session at the point of the fault. See the
    /// documentation for details.
    pub repro: Option<PathBuf>,
    #[arg(short = 'g', long, default_value_t = false)]
    /// Enable the SIMICS GUI
    ///
    /// Enabling the SIMICS GUI during fuzzing is useful for debugging and demonstration
    /// purposes, but is strongly discouraged for real testing and reduces the speed of
    /// the fuzzer significantly. The SIMICS GUI may be also useful for configuring
    /// startup scripts, particularly when pseudo-graphical GUI interaction is needed
    /// such as virtual keyboard inputs.
    pub enable_simics_gui: bool,
    #[arg(short = 'I', long)]
    /// Number of iterations to fuzz for, should only be used for testing or demonstration
    ///
    /// Number of iterations to fuzz for. This is the number of times all fuzzing stages
    /// will be executed. If not specified, the fuzzer will run infinitely. This
    /// argument should not be used for CI fuzzing, instead run the fuzzer with the
    /// 'timeout' shell command to run for a specific amount of time.
    pub iterations: Option<u64>,
    #[arg(long, default_value_t = false)]
    /// Disable grimoire
    ///
    /// Grimoire is an extension to Redqueen that enables structured input fuzzing. It
    /// is enabled by default and requires no extra configuration to use. It should only
    /// be disabled if a particular target behaves poorly when Grimoire is enabled (for
    /// example, infinite solving or excessive solving time). Disabling Redqueen
    /// implicitly disables Grimoire.
    pub disable_grimoire: bool,
    #[arg(long, default_value_t = false)]
    /// Disable redqueen
    ///
    /// Redqueen implements Input-to-State (I2S) fuzzing methods for lightweight
    /// taint-like analysis for effective input mutation by logging comparison values
    /// during target software execution. It is enabled by default and requires no extra
    /// configuration to use, but for some targets may slow down fuzzing. It should only
    /// be disabled if a particular target behaves poorly when Redqueen is enabled (for
    /// example, infinite solving or excessive solving time).
    pub disable_redqueen: bool,
}
