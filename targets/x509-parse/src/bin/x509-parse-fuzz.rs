use std::path::PathBuf;

use anyhow::{Error, Result};
use clap::Parser;
use confuse_fuzz::fuzzer::Fuzzer;
// use confuse_module::module::{
//     components::detector::fault::{Fault, X86_64Fault},
//     config::InitializeConfig,
// };
use confuse_simics_manifest::PublicPackageNumber;
use confuse_simics_project::SimicsProject;
use indoc::{formatdoc, indoc};
use log::{error, Level};
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
use tempfile::Builder as NamedTempFileBuilder;

use x509_parse::X509_PARSE_EFI_MODULE;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to the initial input corpus for the fuzzer
    #[arg(short, long)]
    input: PathBuf,
    /// Logging level
    #[arg(short, long, default_value_t = Level::Error)]
    log_level: Level,
    #[arg(short, long, default_value_t = 1000)]
    cycles: u64,
}

fn init_logging(level: Level) -> Result<()> {
    let logfile = NamedTempFileBuilder::new()
        .prefix("confuse-log")
        .suffix(".log")
        .rand_bytes(4)
        .tempfile()?;
    // This line is very important! Otherwise the file drops after this function returns :)
    let logfile_path = logfile.into_temp_path().to_path_buf();
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
    // Set up logging to a temp file that will roll around every 100mb (this is pretty small for
    // the amount of output we get, so you can increase this if you are debugging)
    init_logging(args.log_level)?;

    // Paths of
    const APP_SCRIPT_PATH: &str = "scripts/app.py";
    const APP_YML_PATH: &str = "scripts/app.yml";
    const BOOT_DISK_PATH: &str = "targets/x509-parse/minimal_boot_disk.craff";
    const STARTUP_NSH_PATH: &str = "targets/x509-parse/run_uefi_app.nsh";
    const STARTUP_SIMICS_PATH: &str = "targets/x509-parse/run-uefi-app.simics";
    const UEFI_APP_PATH: &str = "X509Parse.efi";

    let app = include_bytes!("resource/app.yml");

    let app_script = include_bytes!("resource/app.py");

    let boot_disk = include_bytes!("resource/minimal_boot_disk.craff");

    let run_uefi_app_nsh_script = include_bytes!("resource/run_uefi_app.nsh");

    let run_uefi_app_simics_script = include_bytes!("resource/run-uefi-app.simics");

    let simics_project = SimicsProject::try_new_latest()?
        .try_with_package_latest(PublicPackageNumber::QspX86)?
        .try_with_file_contents(X509_PARSE_EFI_MODULE, UEFI_APP_PATH)?
        .try_with_file_contents(app, APP_YML_PATH)?
        .try_with_file_contents(app_script, APP_SCRIPT_PATH)?
        .try_with_file_contents(boot_disk, BOOT_DISK_PATH)?
        .try_with_file_contents(run_uefi_app_nsh_script, STARTUP_NSH_PATH)?
        .try_with_file_contents(run_uefi_app_simics_script, STARTUP_SIMICS_PATH)?;

    // let init_info = InitializeConfig::default()
    //     .with_faults([
    //         Fault::X86_64(X86_64Fault::Page),
    //         Fault::X86_64(X86_64Fault::InvalidOpcode),
    //     ])
    //     .with_timeout_seconds(3.0);

    // let mut fuzzer = Fuzzer::try_new(
    //     args.input,
    //     init_info,
    //     APP_YML_PATH,
    //     simics_project,
    //     args.log_level,
    // )?;

    // // Workaround to make sure we always stop even if we get a failure
    // fuzzer
    //     .run_cycles(args.cycles)
    //     .or_else(|e| {
    //         error!("Error running cycles: {}", e);
    //         Ok::<(), Error>(())
    //     })
    //     .ok();
    // fuzzer.stop()?;

    Ok(())
}
