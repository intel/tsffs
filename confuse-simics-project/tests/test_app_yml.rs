use confuse_simics_project::{
    bool_param, file_param, int_param, simics_app, simics_path, str_param, SimicsApp,
    SimicsAppParam, SimicsAppParamType,
};

const TEST_APP_YML: &[u8] = include_bytes!("rsrc/qsp-x86-uefi-app.yml");

#[test]
fn test_app_create() {
    let app = simics_app! {
        "QSP With UEFI App (Fuzzing)",
        &simics_path!("qsp-x86-uefi-app.py"),
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
        file_param!(disk0_image: { default: "%simics%/targets/qsp-x86-fuzzing/images/minimal_boot_disk.craff" }),
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
        file_param!(bios_image: { default: "%simics%/targets/qsp-x86/images/SIMICSX58IA32X64_1_1_0_r.fd" }),
        bool_param!(create_usb_tablet: { default: false }),
        str_param!(machine_name: { default: "board" }),
        int_param!(memory_megs: { default: 8192 }),
        str_param!(rtc_time: { default: "2021-06-10 10:41:54" }),
        file_param!(startup_nsh: { default: "%simics%/targets/qsp-x86-fuzzing/images/run_uefi_app.nsh" }),
        file_param!(uefi_app: { default: "HelloFuzzing.efi" }),
        str_param!(eth_link: { output: true }),
        str_param!(service_node: { output: true }),
        str_param!(system: { output: true }),
    };

    assert_eq!(
        app.to_string(),
        String::from_utf8(TEST_APP_YML.to_vec()).expect("Error")
    );
}
