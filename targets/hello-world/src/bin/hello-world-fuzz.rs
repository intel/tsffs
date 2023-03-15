use anyhow::{bail, Result};
use chrono::Local;
use confuse_fuzz::{
    message::{FuzzerEvent, SimicsEvent, StopType},
    Fault, InitInfo,
};
use confuse_module::interface::{
    BOOTSTRAP_SOCKNAME as CONFUSE_MODULE_BOOTSTRAP_SOCKNAME,
    CRATE_NAME as CONFUSE_MODULE_CRATE_NAME,
};
use confuse_simics_manifest::PackageNumber;
use confuse_simics_module::find_module;
use confuse_simics_project::{
    bool_param, file_param, int_param, simics_app, simics_path, str_param, SimicsApp,
    SimicsAppParam, SimicsAppParamType, SimicsProject,
};
use hello_world::HELLO_WORLD_EFI_MODULE;
use indoc::{formatdoc, indoc};
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use libafl::prelude::{tui::TuiMonitor, *};
use log::{debug, error, info, warn, LevelFilter};
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    init_config,
};
use std::{
    io::{BufRead, BufReader},
    process::Stdio,
    thread::spawn,
};
use tempfile::Builder as NamedTempFileBuilder;

fn main() -> Result<()> {
    let logfile = NamedTempFileBuilder::new()
        .prefix("hello-world")
        .suffix(".log")
        .rand_bytes(4)
        .tempfile()?;
    let logfile_path = logfile.path().to_path_buf();
    let appender = FileAppender::builder()
        // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(logfile_path)
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(appender)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Trace),
        )
        .unwrap();
    let _handle = init_config(config)?;

    // init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    // Paths of
    const APP_SCRIPT_PATH: &str = "scripts/app.py";
    const APP_YML_PATH: &str = "scripts/app.yml";
    const BOOT_DISK_PATH: &str = "targets/hello-world/minimal_boot_disk.craff";
    const STARTUP_NSH_PATH: &str = "targets/hello-world/run_uefi_app.nsh";
    const STARTUP_SIMICS_PATH: &str = "targets/hello-world/run-uefi-app.simics";
    const UEFI_APP_PATH: &str = "HelloWorld.efi";

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

    const CONFUSE_START_SIGNAL: u32 = 0x4343;

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
        # SIM_run_command('bp.hap.run-until name = Core_Magic_Instruction index = {}')
        # SIM_run_command('enable-unsupported-feature internals')
        # SIM_run_command('save-snapshot name = origin')
    "#,
            // simics.SIM_lookup_file("%simics%/targets/qsp-x86-fuzzing/run-uefi-app.simics"),
          &simics_path!(STARTUP_SIMICS_PATH),
          CONFUSE_START_SIGNAL,
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

    let confuse_module = find_module(CONFUSE_MODULE_CRATE_NAME)?;

    // let input = repeat(b'A').take(4096).collect::<Vec<_>>();
    let simics_project = SimicsProject::try_new()?
        .try_with_package(PackageNumber::QuickStartPlatform)?
        .try_with_file_contents(HELLO_WORLD_EFI_MODULE, UEFI_APP_PATH)?
        .try_with_file_contents(app.to_string().as_bytes(), APP_YML_PATH)?
        .try_with_file_contents(app_script.as_bytes(), APP_SCRIPT_PATH)?
        .try_with_file_contents(boot_disk, BOOT_DISK_PATH)?
        .try_with_file_contents(run_uefi_app_nsh_script, STARTUP_NSH_PATH)?
        .try_with_file_contents(run_uefi_app_simics_script.as_bytes(), STARTUP_SIMICS_PATH)?
        // .try_with_file_contents(&input, "corpus/input")?
        .try_with_module(CONFUSE_MODULE_CRATE_NAME, confuse_module)?;

    info!("Project: {}", simics_project.base_path.display());

    let (bootstrap, bootstrap_name) = IpcOneShotServer::new()?;

    let mut simics_process = simics_project
        .command()
        .args(simics_project.module_load_args())
        .arg(APP_YML_PATH)
        .arg("-batch-mode")
        .arg("-e")
        .arg("@SIM_main_loop()")
        .current_dir(&simics_project.base_path)
        .env(CONFUSE_MODULE_BOOTSTRAP_SOCKNAME, bootstrap_name)
        .env("RUST_LOG", "trace")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = simics_process.stdout.take().expect("Could not get stdout");
    let stderr = simics_process.stderr.take().expect("Could not get stderr");

    let simics_output_reader = spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).expect("Could not read line");
            info!("SIMICS: {}", line.trim());
        }
    });

    let simics_err_reader = spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).expect("Could not read line");
            warn!("SIMICS: {}", line.trim());
        }
    });

    let (_, (tx, rx)): (_, (IpcSender<FuzzerEvent>, IpcReceiver<SimicsEvent>)) =
        bootstrap.accept()?;

    info!("Sending initialize");

    let mut info = InitInfo::default();
    info.add_fault(Fault::Triple);
    info.add_fault(Fault::InvalidOpcode);
    // Hello World stalls for 10 seconds on 'B', we'll treat a timeout as slightly shorter than
    // that
    info.set_timeout_seconds(6);

    tx.send(FuzzerEvent::Initialize(info))?;

    info!("Receiving ipc shm");

    let mut shm = match rx.recv()? {
        SimicsEvent::SharedMem(shm) => shm,
        _ => bail!("Unexpected message received"),
    };

    let mut writer = shm.writer()?;

    info!("Got writer");

    info!("Sending initial reset signal");

    tx.send(FuzzerEvent::Reset)?;

    let coverage_observer =
        unsafe { StdMapObserver::from_mut_ptr("map", writer.as_mut_ptr(), writer.len()) };

    let mut coverage_feedback = MaxMapFeedback::new(&coverage_observer);

    // let mut objectives: Vec<bool> = Vec::new();
    // let objectives_observer = unsafe { ListObserver::new("objectives", &mut objectives) };
    // let mut objectives_feedback = ListFeedback::with_observer(&objectives_observer);

    let mut objective = CrashFeedback::new();

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        InMemoryCorpus::new(),
        OnDiskCorpus::new(simics_project.base_path.join("crashes"))?,
        &mut coverage_feedback,
        &mut objective,
    )?;

    let mon = TuiMonitor::new("Test fuzzer for hello world".to_string(), true);
    let mut mgr = SimpleEventManager::new(mon);
    let scheduler = QueueScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, coverage_feedback, objective);

    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.as_slice();
        let run_input = buf.to_vec();
        let mut exit_kind = ExitKind::Ok;
        // We expect we'll get a simics ready message:

        info!("Running with input '{:?}'", run_input);
        match rx.recv().expect("Failed to receive message") {
            SimicsEvent::Ready => {
                debug!("Received ready signal");
            }
            _ => {
                error!("Received unexpected event");
            }
        }

        info!("Sending run signal");
        tx.send(FuzzerEvent::Run(run_input))
            .expect("Failed to send message");

        match rx.recv().expect("Failed to receive message") {
            SimicsEvent::Stopped(stop_type) => match stop_type {
                StopType::Crash => {
                    error!("Target crashed, yeehaw!");
                    exit_kind = ExitKind::Crash;
                }
                StopType::Normal => {
                    info!("Target stopped normally ;_;");

                    exit_kind = ExitKind::Ok;
                }
                StopType::TimeOut => {
                    warn!("Target timed out, yeehaw(???)");
                    exit_kind = ExitKind::Timeout;
                }
            },
            _ => {
                error!("Received unexpected event");
            }
        }

        // We'd read the state of the vm here, including caught exceptions and branch trace
        // Now we send the reset signal
        debug!("Sending reset signal");

        tx.send(FuzzerEvent::Reset).expect("Failed to send message");

        debug!("Harness done");

        exit_kind
    };

    info!("Creating executor");

    let mut executor = InProcessExecutor::new(
        &mut harness,
        tuple_list!(coverage_observer),
        &mut fuzzer,
        &mut state,
        &mut mgr,
    )?;

    info!("Generating initial inputs");

    let mut generator = RandBytesGenerator::new(32);

    state.generate_initial_inputs_forced(
        &mut fuzzer,
        &mut executor,
        &mut generator,
        &mut mgr,
        8,
    )?;

    info!("Creating mutator");

    let mutator = StdScheduledMutator::new(havoc_mutations());

    let mut stages = tuple_list!(StdMutationalStage::new(mutator));

    info!("Starting fuzz loop");

    fuzzer.fuzz_loop_for(&mut stages, &mut executor, &mut state, &mut mgr, 100)?;

    info!("Done.");

    // We expect we'll get a simics ready message:
    match rx.recv()? {
        SimicsEvent::Ready => {
            info!("Received ready signal");
        }
        _ => {
            error!("Received unexpected event");
        }
    }

    tx.send(FuzzerEvent::Stop)?;

    simics_output_reader
        .join()
        .expect("Could not join output thread");
    simics_err_reader
        .join()
        .expect("Could not join output thread");
    simics_process.wait()?;

    Ok(())
}
