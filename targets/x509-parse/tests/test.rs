use anyhow::Result;
use confuse_module::CRATE_NAME as CONFUSE_MODULE_CRATE_NAME;
use confuse_simics_manifest::PackageNumber;
use confuse_simics_module::find_module;
use confuse_simics_project::{
    bool_param, file_param, int_param, simics_app, simics_path, str_param, SimicsApp,
    SimicsAppParam, SimicsAppParamType, SimicsProject,
};
use indoc::{formatdoc, indoc};
use x509_parse::X509_PARSE_EFI_MODULE;

#[test]
pub fn test_load() -> Result<()> {
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

    const CONFUSE_START_SIGNAL: u32 = 0x4343;
    const CONFUSE_STOP_SIGNAL: u32 = 0x4242;

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

        if SIM_get_batch_mode():
            SIM_log_info(
                1,
                conf.sim,
                0,
                'Batch mode detected. Disconnecting console from VGA'
            )
            conf.board.mb.gpu.vga.console=None

        SIM_run_command('bp.hap.run-until name = Core_Magic_Instruction index = {}')
        SIM_run_command('enable-unsupported-feature internals')
        SIM_run_command('save-snapshot name = origin')

        cmd_output = io.StringIO()

        with contextlib.redirect_stdout(cmd_output):
            SIM_run_command('list-snapshots')

        res = cmd_output.getvalue()

        ckpt_id = -1

        for line in res.split('\n'):
            line = line.split()
            if len(line) > 2 and line[1]=='origin':
                ckpt_id = int(line[0])

        if ckpt_id != 0:
            print("Error! Microcheckpoint id incorrect")
        else:
            print("Took checkpoint :)")
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
            ! Starts a virtual machine that boots the provided kernel, with the provided rootFS, initrd and commandline.

            params from "{}"
            # Do not expose all advanced options to the end user
            except use_acpi, use_vmp, lan_bios_image, spi_flash_image,
                    enable_efi, vga_bios_image, disk0_image
            default num_cores = 1
            default real_time_mode = FALSE
            default show_con0 = TRUE
            default create_disk1 = NIL
            default disk0_size = 200Mi
            default enable_break_on_reboot = FALSE
            default system_info = "QSP x86 with externally provided Kernel/RootFs/InitRd"
            default bios_image = "{}"
            
            group "Disks"
            param disk0_image : file("*") or nil = "{}"
            ! Disk image for disk0. Will be used as boot medium.

            group "MSR"
            param tsc_factor : int = 20
            ! TSC factor field for platform info MSR.

            group "System"
            param auto_start_uefi_shell : bool = TRUE
            ! Automatically enter BIOS setup and start UEFI shell
            param tmp_dir : string or nil = NIL
            ! Directory on the host where to place tmp files used to start the system.
            param startup_nsh : file("*") or nil = "{}"
            ! NSH script that controls things. 
            param uefi_app : file("*")
            ! UEFI app you wanna start. 

            result system : string
            result eth_link : string or nil
            result service_node : string or nil
        }}

        run-command-file {}

        @import os
        @simenv.startup_nsh_nodir = os.path.basename(simenv.startup_nsh)
        @simenv.uefi_app_nodir = os.path.basename(simenv.uefi_app)

        # The below branch will (when enabled) enter the BIOS menu by pressing ESC
        # after 10 seconds, then go to the third entry on the top level (by pressin DOWN twice).
        # The assumption is that this is the boot device selection (which is true for the QSP BIOS)
        # Then there is one press of UP, to select the last element in the list, which is assumed
        # to be the UEFI shell (which again is true for the QSP BIOS). Then the shell is started.
        if $auto_start_uefi_shell {{
            script-branch "UEFI Shell Enter Branch" {{
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
                
                $con.input "FS0:\n"
                bp.time.wait-for seconds = 10

                $con.input ("set -v UEFI_APP_ON_HOST \" " + $uefi_app + "\"\n")
                bp.time.wait-for seconds = .5

                $con.input ("set -v UEFI_APP_NODIR \" " + $uefi_app_nodir + "\"\n")
                bp.time.wait-for seconds = .5

            
                local $manager = (start-agent-manager)

                $con.input ("SimicsAgent.efi --download " + (lookup-file $startup_nsh) + "\n")
                bp.time.wait-for seconds = .5
                
                $con.input ("" + $startup_nsh_nodir + "\n")

            }}
        }}
    "#,
        &simics_path!("targets/qsp-x86/qsp-hdd-boot.simics"),
        &simics_path!("targets/qsp-x86/images/SIMICSX58IA32X64_1_1_0_r.fd"),
        &simics_path!(BOOT_DISK_PATH),
        &simics_path!(STARTUP_NSH_PATH),
        "targets/qsp-x86/qsp-hdd-boot.simics"
    };

    let confuse_module = find_module(CONFUSE_MODULE_CRATE_NAME)?;

    let mut simics_project = SimicsProject::try_new()?
        .try_with_package(PackageNumber::QuickStartPlatform)?
        .try_with_file_contents(&X509_PARSE_EFI_MODULE, UEFI_APP_PATH)?
        .try_with_file_contents(&app.to_string().as_bytes(), APP_YML_PATH)?
        .try_with_file_contents(app_script.as_bytes(), APP_SCRIPT_PATH)?
        .try_with_file_contents(boot_disk, BOOT_DISK_PATH)?
        .try_with_file_contents(run_uefi_app_nsh_script, STARTUP_NSH_PATH)?
        .try_with_file_contents(run_uefi_app_simics_script.as_bytes(), STARTUP_SIMICS_PATH)?
        .try_with_module(CONFUSE_MODULE_CRATE_NAME, &confuse_module)?;

    println!("Project: {}", simics_project.base_path.display());

    Ok(())
}
