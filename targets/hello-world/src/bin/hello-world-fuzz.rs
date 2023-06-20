use anyhow::Result;
use artifact_dependency::{ArtifactDependencyBuilder, CrateType};
use clap::Parser;
use confuse_module::{
    config::{InputConfig, TraceMode},
    faults::{x86_64::X86_64Fault, Fault},
};
use hello_world::HELLO_WORLD_EFI_MODULE;
use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::delete::DeleteRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    init_config, Config,
};
use simics::{
    api::sys::SIMICS_VERSION,
    module::ModuleBuilder,
    project::{ProjectPathBuilder, SimicsProject},
};
use simics::{
    package::{PackageBuilder, PublicPackageNumber},
    project::{Project, ProjectBuilder},
};
use std::path::PathBuf;
use tempfile::Builder as NamedTempFileBuilder;
use tracing::{info, Level};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    /// Path to the initial input corpus for the fuzzer
    input: PathBuf,
    #[arg(short, long)]
    /// Path to the initial input corpus for the fuzzer
    output: PathBuf,
    #[arg(short, long, default_value_t = Level::Error)]
    /// Logging level
    log_level: Level,
    #[arg(short, long, default_value_t = 0)]
    /// Number of cycles to fuzz for, or forever if zero
    cycles: u64,
    #[arg(short = 'L', long)]
    /// Log file path to use. A new tmp file with the pattern confuse-log.XXXXX.log will be used
    /// if not specified
    log_file: Option<PathBuf>,
    #[arg(short, long, default_value_t = TraceMode::HitCount)]
    /// Mode to trace executions with
    trace_mode: TraceMode,
    #[arg(short = 'C', long)]
    /// Expression for the set of cores. For example
    /// 1,2-4,6: clients run in cores 1,2,3,4,6
    /// all: one client runs on each available core
    cores: String,
}

fn init_logging(level: Level, log_file: Option<PathBuf>) -> Result<()> {
    // This line is very important! Otherwise the file drops after this function returns :)
    let logfile_path = if let Some(log_file) = log_file {
        log_file
    } else {
        let logfile = NamedTempFileBuilder::new()
            .prefix("confuse-log")
            .suffix(".log")
            .rand_bytes(4)
            .tempfile()?;

        logfile.into_temp_path().to_path_buf()
    };

    let size_trigger = Box::new(SizeTrigger::new(100_000_000));
    let roller = Box::new(DeleteRoller::new());
    let policy = Box::new(CompoundPolicy::new(size_trigger, roller));
    let encoder = Box::new(PatternEncoder::new("{l:5.5} | {d(%H:%M:%S)} | {m}{n}"));
    let appender = RollingFileAppender::builder()
        .encoder(encoder)
        .build(logfile_path, policy)?;
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(appender)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(level.to_level_filter()),
        )?;
    let _handle = init_config(config)?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_logging(args.log_level, args.log_file)?;

    // init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    // Paths of
    const APP_SCRIPT_PATH: &str = "%simics%/scripts/app.py";
    const APP_YML_PATH: &str = "%simics%/scripts/app.yml";
    const BOOT_DISK_PATH: &str = "%simics%/targets/hello-world/minimal_boot_disk.craff";
    const STARTUP_NSH_PATH: &str = "%simics%/targets/hello-world/run_uefi_app.nsh";
    const STARTUP_SIMICS_PATH: &str = "%simics%/targets/hello-world/run-uefi-app.simics";
    const UEFI_APP_PATH: &str = "%simics%/targets/hello-world/HelloWorld.efi";

    let app_yml = include_bytes!("resource/app.yml");
    let app_script = include_bytes!("resource/app.py");
    let boot_disk = include_bytes!("resource/minimal_boot_disk.craff");
    let run_uefi_app_nsh_script = include_bytes!("resource/run_uefi_app.nsh");
    let run_uefi_app_simics_script = include_bytes!("resource/run-uefi-app.simics");

    let project = ProjectBuilder::default()
        .path(ProjectPathBuilder::default().temporary(false).build()?)
        .package(
            PackageBuilder::default()
                .package_number(PublicPackageNumber::QspX86)
                .build()?,
        )
        .module(
            ModuleBuilder::default()
                .artifact(
                    ArtifactDependencyBuilder::default()
                        .crate_name("confuse_module")
                        .artifact_type(CrateType::CDynamicLibrary)
                        .build_missing(true)
                        .feature(SIMICS_VERSION)
                        .build()?
                        .build()?,
                )
                .build()?,
        )
        .file_content((HELLO_WORLD_EFI_MODULE.to_vec(), UEFI_APP_PATH.parse()?))
        .file_content((app_yml.to_vec(), APP_YML_PATH.parse()?))
        .file_content((app_script.to_vec(), APP_SCRIPT_PATH.parse()?))
        .file_content((boot_disk.to_vec(), BOOT_DISK_PATH.parse()?))
        .file_content((run_uefi_app_nsh_script.to_vec(), STARTUP_NSH_PATH.parse()?))
        .file_content((
            run_uefi_app_simics_script.to_vec(),
            STARTUP_SIMICS_PATH.parse()?,
        ))
        .build()?
        .setup()?;

    info!("Project: {:?}", project);

    Ok(())
}
