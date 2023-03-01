use anyhow::{bail, Result};
use confuse_simics_project::{SimicsApp, SimicsAppParam, SimicsAppParamType};
use serde_yaml::{from_reader, to_string};

const TEST_APP_YML: &[u8] = include_bytes!("rsrc/qsp-x86-uefi-app.yml");

#[test]
fn test_parse_app_yml() -> Result<()> {
    let app: SimicsApp = from_reader(TEST_APP_YML)?;
    assert_eq!(
        app.description, "QSP with UEFI App (Fuzzing)",
        "Incorrect description."
    );
    match app.params.get("disk0_size") {
        Some(p) => match p.param {
            SimicsAppParamType::Int(pt) => {
                assert!(pt.is_some() && pt.unwrap() == 209715200);
            }
            _ => bail!("Incorrect param type"),
        },
        None => bail!("Failed to get disk0_size param"),
    }

    Ok(())
}

#[test]
fn test_generate_app_yml() -> Result<()> {
    let app = SimicsApp::new("QSP with UEFI App", "%script/app.py")
        .param("apic_freq_mhz", SimicsAppParam::default().int(133))
        .param("cpi", SimicsAppParam::default().int(1))
        .param("cpu_comp_class", SimicsAppParam::default())
        .param("apic_freq_mhz", SimicsAppParam::default().int(133))
        .param("cpi", SimicsAppParam::default().int(1))
        .param("cpu_comp_class", SimicsAppParam::default().str("x86QSP1"))
        .param("freq_mhz", SimicsAppParam::default().int(2000))
        .param("num_cores", SimicsAppParam::default().int(1))
        .param("num_cores_small", SimicsAppParam::default().int(0))
        .param("num_cpus", SimicsAppParam::default().int(1))
        .param("num_threads", SimicsAppParam::default().int(1))
        .param("num_threads_small", SimicsAppParam::default().int(0))
        .param("hide_consoles", SimicsAppParam::default().bool(false))
        .param(
            "serial_console_bg_color",
            SimicsAppParam::default().str("black"),
        )
        .param(
            "serial_console_fg_color",
            SimicsAppParam::default().str("white"),
        )
        .param("show_con0", SimicsAppParam::default().bool(true))
        .param("show_con1", SimicsAppParam::default().bool(false))
        .param("show_gfx_con", SimicsAppParam::default().bool(true))
        .param("create_cdrom", SimicsAppParam::default().str("sata"))
        .param("create_disk0", SimicsAppParam::default().str("sata"))
        .param(
            "disk0_image",
            SimicsAppParam::default()
                .file("%simics%/targets/qsp-x86-fuzzing/images/minimal_boot_disk.craff"),
        )
        .param("disk0_size", SimicsAppParam::default().int(209715200))
        .param("tsc_factor", SimicsAppParam::default().int(20))
        .param(
            "connect_real_network",
            SimicsAppParam::default().str("napt"),
        )
        .param("create_network", SimicsAppParam::default().bool(true))
        .param("create_service_node", SimicsAppParam::default().bool(true))
        .param(
            "dhcp_domain_name",
            SimicsAppParam::default().str("network.sim"),
        )
        .param("dhcp_pool_size", SimicsAppParam::default().int(100))
        .param("eth_connector_vlan_id", SimicsAppParam::default().int(-1))
        .param("eth_vlan_enable", SimicsAppParam::default().bool(false))
        .param("ip_address", SimicsAppParam::default().str("auto"))
        .param("mac_address", SimicsAppParam::default().str("auto"))
        .param(
            "service_node_ip_address",
            SimicsAppParam::default().str("10.10.0.1"),
        )
        .param("service_node_setup", SimicsAppParam::default().bool(true))
        .param("service_node_vlan_id", SimicsAppParam::default().int(-1))
        .param("create_osa", SimicsAppParam::default().bool(true))
        .param("create_tracker", SimicsAppParam::default().bool(false))
        .param(
            "enable_break_on_reboot",
            SimicsAppParam::default().bool(false),
        )
        .param("enable_system_clock", SimicsAppParam::default().bool(false))
        .param("real_time_mode", SimicsAppParam::default().bool(false))
        .param("system_clock_class", SimicsAppParam::default().str("clock"))
        .param(
            "system_info",
            SimicsAppParam::default().str("QSP x86 with externally provided Kernel/RootFs/InitRd"),
        )
        .param(
            "auto_start_uefi_shell",
            SimicsAppParam::default().bool(true),
        )
        .param(
            "bios_image",
            SimicsAppParam::default()
                .file("%simics%/targets/qsp-x86/images/SIMICSX58IA32X64_1_1_0_r.fd"),
        )
        .param("create_usb_tablet", SimicsAppParam::default().bool(false))
        .param("machine_name", SimicsAppParam::default().str("board"))
        .param("memory_megs", SimicsAppParam::default().int(8192))
        .param(
            "rtc_time",
            SimicsAppParam::default().str("2021-06-10 10:41:54"),
        )
        .param(
            "startup_nsh",
            SimicsAppParam::default()
                .file("%simics%/targets/qsp-x86-fuzzing/images/run_uefi_app.nsh"),
        )
        .param(
            "uefi_app",
            SimicsAppParam::default().file("HelloFuzzing.efi"),
        )
        .param("eth_link", SimicsAppParam::new_str().output(true))
        .param("service_node", SimicsAppParam::new_str().output(true))
        .param("system", SimicsAppParam::new_str().output(true));

    let _ = to_string(&app)?;

    Ok(())
}
