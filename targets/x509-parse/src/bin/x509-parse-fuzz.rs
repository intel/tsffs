use std::path::PathBuf;

use anyhow::{Error, Result};
use clap::Parser;
use confuse_fuzz::fuzzer::Fuzzer;
use confuse_module::messages::{Fault, InitInfo};
use confuse_simics_manifest::PublicPackageNumber;
use confuse_simics_project::{
    bool_param, file_param, int_param, simics_app, simics_path, str_param, SimicsApp,
    SimicsAppParam, SimicsAppParamType, SimicsProject,
};
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

    let app = simics_app! {
        "QSP With UEFI App (Fuzzing)",
        &simics_path!(APP_SCRIPT_PATH),
        int_param!(apic_freq_mhz: { default: 133 }),
        int_param!(cpi: { default: 1 }),
        str_param!(cpu_comp_class: { default: "x86QSP1" }),
        int_param!(freq_mhz: { default: 2000 }),
        int_param!(num_cores: { default: 1 }),
        int_param!(num_cores_small: { default: 0 }),
        int_param!(num_cpus: { default: 1 }),
        int_param!(num_threads: { default: 1 }),
        int_param!(num_threads_small: { default: 0 }),
        bool_param!(hide_consoles: { default: false }),
        str_param!(serial_console_bg_color: { default: "black" }),
        str_param!(serial_console_fg_color: { default: "white" }),
        bool_param!(show_con0: { default: true }),
        bool_param!(show_con1: { default: false }),
        bool_param!(show_gfx_con: { default: true }),
        str_param!(create_cdrom: { default: "sata" }),
        str_param!(create_disk0: { default: "sata" }),
        file_param!(disk0_image: { default: &simics_path!(BOOT_DISK_PATH) }),
        int_param!(disk0_size: { default: 209715200 }),
        int_param!(tsc_factor: { default: 20 }),
        str_param!(connect_real_network: { default: "napt" }),
        bool_param!(create_network: { default: true }),
        bool_param!(create_service_node: { default: true }),
        str_param!(dhcp_domain_name: { default: "network.sim" }),
        int_param!(dhcp_pool_size: { default: 100 }),
        int_param!(eth_connector_vlan_id: { default: -1 }),
        bool_param!(eth_vlan_enable: { default: false }),
        str_param!(ip_address: { default: "auto" }),
        str_param!(mac_address: { default: "auto" }),
        str_param!(service_node_ip_address: { default: "10.10.0.1" }),
        bool_param!(service_node_setup: { default: true }),
        int_param!(service_node_vlan_id: { default: -1 }),
        bool_param!(create_osa: { default: true }),
        bool_param!(create_tracker: { default: false}),
        bool_param!(enable_break_on_reboot: { default: false }),
        bool_param!(enable_system_clock: { default: false }),
        bool_param!(real_time_mode: { default: false }),
        str_param!(system_clock_class: { default: "clock" }),
        str_param!(system_info: { default: "QSP x86 with externally provided Kernel/RootFs/" }),
        bool_param!(auto_start_uefi_shell: { default: true }),
        file_param!(bios_image: { default: &simics_path!("targets/qsp-x86/images/SIMICSX58IA32X64_1_1_0_r.fd") }),
        bool_param!(create_usb_tablet: { default: false }),
        str_param!(machine_name: { default: "board" }),
        int_param!(memory_megs: { default: 8192 }),
        str_param!(rtc_time: { default: "2021-06-10 10:41:54" }),
        file_param!(startup_nsh: { default: &simics_path!(STARTUP_NSH_PATH) }),
        file_param!(uefi_app: { default: &simics_path!(UEFI_APP_PATH) }),
        str_param!(eth_link: { output: true }),
        str_param!(service_node: { output: true }),
        str_param!(system: { output: true }),
    };

    let app_script = formatdoc! {r#"
        from sim_params import params
        import simics
        import commands
        import io, contextlib

        args = [
            [name, commands.param_val_to_str(value)] for (name, value) in params.items()
        ]

        simics.SIM_run_command_file_params(
            simics.SIM_lookup_file("{}"),
            True, args
        )

        SIM_create_object('confuse_module', 'confuse_module', [])
        conf.confuse_module.processor = SIM_get_object(simenv.system).mb.cpu0.core[0][0]


        if SIM_get_batch_mode():
            SIM_log_info(
                1,
                conf.sim,
                0,
                'Batch mode detected. Disconnecting console from VGA'
            )
            conf.board.mb.gpu.vga.console=None

        conf.confuse_module.signal = 1
    "#,
            // simics.SIM_lookup_file("%simics%/targets/qsp-x86-fuzzing/run-uefi-app.simics"),
          &simics_path!(STARTUP_SIMICS_PATH),
    };

    let boot_disk = include_bytes!("resource/test_load/minimal_boot_disk.craff");

    let run_uefi_app_nsh_script = indoc! {br#"
        #Get kernel image
        SimicsAgent.efi --download %UEFI_APP_ON_HOST%

        %UEFI_APP_NODIR%
    "#};

    let run_uefi_app_simics_script = formatdoc! {r#"
        decl {{

            # We import most parameters from the QSP-X86 boot script
            params from "{}"

            group "MSR"

            # Set the TSC factor field for platform info MSR.
            param tsc_factor : int = 20

            group "System"

            # Automatically enter BIOS setup and start UEFI shell using the script
            # branch below
            param auto_start_uefi_shell : bool = TRUE

            # NSH script that controls things. 
            param startup_nsh : file("*") or nil = "{}"

            # UEFI app you wanna start. 
            param uefi_app : file("*")

            result system : string
            result eth_link : string or nil
            result service_node : string or nil
        }}

        echo "Loaded simics declaration"

        echo "Running command file"

        run-command-file {}

        @import os
        @simenv.startup_nsh_nodir = os.path.basename(simenv.startup_nsh)
        echo "Set startup nsh"
        @simenv.uefi_app_nodir = os.path.basename(simenv.uefi_app)
        echo "Set startup uefi app"

        # The below branch will (when enabled) enter the BIOS menu by pressing ESC
        # after 10 seconds, then go to the third entry on the top level (by pressin DOWN twice).
        # The assumption is that this is the boot device selection (which is true for the QSP BIOS)
        # Then there is one press of UP, to select the last element in the list, which is assumed
        # to be the UEFI shell (which again is true for the QSP BIOS). Then the shell is started.

        # Confuse note: this is actually needed to boot the uefi image!

        if $auto_start_uefi_shell {{
            script-branch "UEFI Shell Enter Branch" {{
                echo "Doing UEFI button combination"
                local $kbd = $system.mb.sb.kbd
                local $con = $system.console.con
                local $sercon = $system.serconsole.con
                bp.time.wait-for seconds = 10
                $kbd.key-press ESC
                bp.time.wait-for seconds = 3
                foreach $i in (range 2) {{
                    $kbd.key-press KP_DOWN
                    bp.time.wait-for seconds = .5
                }}
                $kbd.key-press ENTER
                bp.time.wait-for seconds = .5
                $kbd.key-press KP_UP
                bp.time.wait-for seconds = .5
                $kbd.key-press ENTER
                bp.time.wait-for seconds = .5
                
                #stop countdown
                $kbd.key-press ENTER         
                bp.time.wait-for seconds = .5
                
                echo "Running command: FS0:\n"

                $con.input "FS0:\n"
                bp.time.wait-for seconds = 10

                echo "Running command: " + "set -v UEFI_APP_ON_HOST \"" + $uefi_app + "\"\n"
                $con.input ("set -v UEFI_APP_ON_HOST \" " + $uefi_app + "\"\n")
                bp.time.wait-for seconds = .5

                echo "Running command: " + "set -v UEFI_APP_NODIR \"" + $uefi_app_nodir + "\"\n"
                $con.input ("set -v UEFI_APP_NODIR \" " + $uefi_app_nodir + "\"\n")
                bp.time.wait-for seconds = .5

            
                local $manager = (start-agent-manager)

                echo "Running command: " + "SimicsAgent.efi --download \"" + (lookup-file $startup_nsh) + "\"\n"
                $con.input ("SimicsAgent.efi --download " + (lookup-file $startup_nsh) + "\n")
                bp.time.wait-for seconds = .5
                
                echo "Running command: " + "\"" + $startup_nsh_nodir + "\"\n"
                $con.input ("" + $startup_nsh_nodir + "\n")

            }}
        }}
    "#,
        &simics_path!("targets/qsp-x86/qsp-hdd-boot.simics"),
        // &simics_path!("targets/qsp-x86/images/SIMICSX58IA32X64_1_1_0_r.fd"),
        // &simics_path!(BOOT_DISK_PATH),
        &simics_path!(STARTUP_NSH_PATH),
        "targets/qsp-x86/qsp-hdd-boot.simics"
    };

    let simics_project = SimicsProject::try_new_latest()?
        .try_with_package_latest(PublicPackageNumber::QspX86)?
        .try_with_file_contents(X509_PARSE_EFI_MODULE, UEFI_APP_PATH)?
        .try_with_file_contents(app.to_string().as_bytes(), APP_YML_PATH)?
        .try_with_file_contents(app_script.as_bytes(), APP_SCRIPT_PATH)?
        .try_with_file_contents(boot_disk, BOOT_DISK_PATH)?
        .try_with_file_contents(run_uefi_app_nsh_script, STARTUP_NSH_PATH)?
        .try_with_file_contents(run_uefi_app_simics_script.as_bytes(), STARTUP_SIMICS_PATH)?;

    let mut init_info = InitInfo::default();

    init_info.add_faults([
        Fault::Triple,
        Fault::InvalidOpcode,
        Fault::Double,
        Fault::GeneralProtection,
        Fault::Page,
    ]);

    init_info.set_timeout_seconds(3);

    let mut fuzzer = Fuzzer::try_new(
        args.input,
        init_info,
        APP_YML_PATH,
        simics_project,
        args.log_level,
    )?;

    // Workaround to make sure we always stop even if we get a failure
    fuzzer
        .run_cycles(args.cycles)
        .or_else(|e| {
            error!("Error running cycles: {}", e);
            Ok::<(), Error>(())
        })
        .ok();
    fuzzer.stop()?;

    Ok(())
}
