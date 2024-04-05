// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure, Result};
use ispm_wrapper::ispm::{self, GlobalOptions};
use std::{
    env::var,
    fs::read_dir,
    path::{Path, PathBuf},
};

/// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
/// exist, returns an error
pub fn subdir<P>(dir: P) -> Result<PathBuf>
where
    P: AsRef<Path>,
{
    let subdirs = read_dir(dir)?
        .filter_map(|p| p.ok())
        .map(|p| p.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    ensure!(
        subdirs.len() == 1,
        "Expected exactly 1 sub-directory, found {}",
        subdirs.len()
    );

    subdirs
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("No sub-directories found"))
}

/// Emit configuration directives used in the build process to conditionally enable
/// features that aren't compatible with all supported SIMICS versions, based on the
/// SIMICS version of the low level bindings. This is not needed for all consumers of the
/// API, but is useful for consumers which need to remain compatible with a wide range of
/// SIMICS base versions.
///
/// Changelog:
///
/// 6.0.163->6.0.164:
///     - Add telnet_connection_v2_interface_t
///     - Add vnc_server_v2_interface_t
/// 6.0.166->6.0.167:
///     - Add probe_notification_context_interface_t
/// 6.0.167->6.0.168:
///     - Add bool VT_attr_values_equal(attr_value_t a1, attr_value_t a2)
///     - Add interrupt_subscriber_interface_t and interrupt_source_t
/// 6.0.169->6.0.170:
///     - Add riscv_imsic_interface_t and riscv_imsic_file_it_t
/// 6.0.170->6.0.171:
///     - Add Sim_Global_Notify_Message variant to global_notifier_type_t
/// 6.0.172->6.0.173:
///     - Add riscv_signal_sgeip_interface_t
///     - Add Sim_Atom_Id_pcie_destination_segment variant to atom_id_t
///     - Add preset argument to:
///         void VT_load_target_preset_yml(const char *target, const char *ns, const char *preset, const char *preset_yml);
///         (was void VT_load_target_preset_yml(const char *target, const char *ns, const char *preset_yml);
///     - Add snapshot API v1
///         - void VT_save_snapshot(const char *name);
///         - bool VT_restore_snapshot(int index);
///         - bool VT_delete_snapshot(int index);
///         - attr_value_t VT_snapshot_size_used();
///         - attr_value_t VT_list_snapshots();
///         - void VT_snapshots_ignore_class(const char *class_name);
///     - Add vhdx_params_t
///     - Add vhdx_file_t *vhdx_creat(const char *fname, uint64 size, craff_error_t *ce, vhdx_params_t *params);
/// 6.0.173->6.0.174:
///     - Add return value to:
///         bool VT_save_snapshot(const char *name);
///         (was void VT_save_snapshot(const char *name);)
/// 6.0.174->6.0.175:
///     - Add Sim_Global_Notify_Snapshot_Will_Load and Sim_Global_Notify_Snapshot_Did_Load variants to global_notifier_type_t
///     - Add void *VT_save_and_release_python_lock();
///     - Add void VT_obtain_and_restore_python_lock(void *saved);
///     - Add return value to:
///         attr_value_t SIM_load_target(const char *target, const char *ns, attr_value_t presets, attr_value_t cmdline_args);
///         (was void SIM_load_target(const char *target, const char *ns, attr_value_t presets, attr_value_t cmdline_args);)
/// 6.0.176->6.0.177:
///     - Add pcie_adapter_compat_interface_t interface
///     - Add bool VT_is_loading_snapshot();
///     - Remove probe_notification_context_interface_t
/// 6.0.177->6.0.178:
///     - Add void VT_snapshots_skip_class_resotre(conf_class_t *cls);
///     - Add void VT_snapshots_skip_attr_restore(conf_class_t *cls, const char *attr_name);
///     - Add attr_value_t VT_dump_snapshot(const char *name);
/// 6.0.179->6.0.180:
///     - Add snapshot_error_t type
///     - Rename:
///         bool VT_is_restoring_snapshot();
///         (was bool VT_is_loading_snapshot();)
///     - Change return value of (and rename):
///         snapshot_error_t VT_take_snapshot(const char *name);
///         (was bool VT_save_snapshot(const char *name);)
///         snapshot_error_t VT_restore_snapshot(const char *name);
///         (was bool VT_restore_snapshot(int index);)
///         snapshot_error_t VT_delete_snapshot(const char *name);
///         (was bool VT_delete_snapshot(int index);)    
///     - Add attr_value_t VT_get_snapshot_info(const char *name);
/// 6.0.180->6.0.181:
///     - Change argument type:
///         void VT_load_target_preset_yml(const char *target, const char *ns, attr_value_t presets, const char *preset_yml);
///         (was void VT_load_target_preset_yml(const char *target, const char *ns, cosnt char *preset, const char *preset_yml);)
/// 6.0.183->6.0.184:
///     - Add pcie_hotplug_events_interface_t, pcie_hotplug_pd_t, pcie_hotplug_mrl_t
///     - Add probe_array_interface_t
///     - Remove probe_cache_interface_t
/// 6.0.184->6.0.185:
///     - Add pcie_link_training_interface_t, pcie_link_speed_t, pcie_link_width_t, pcie_link_negotiation_t
/// 6.0.188->6.0.89:
///     - Add Sim_Log_Warning variant to log_type_t
///     - Add void VT_log_warning(conf_object_t *dev, uint64 grp, const char *str, ...);
/// 6.X.X->7.0.0:
///     - Remove bool VT_is_reversing();
///     - Remove bool SIM_is_loading_micro_checkpoint(cont_object_t *obj);
///     - Remove Sim_Attr_Session and Sim_Attr_Doc variants from attr_type_t
///     - Remove i2c_device_state_t, i2c_device_flag_t, i2c_bus_interface_t, i2c_device_interface_t, i2c_status_t, i2c_link_interface_t, i2c_slave_interface_t, i2c_master_interface_t, i2c_bridge_interface_t
///     - Remove device_interrupt_t, device_interrupt_clear_t, interrupt_query_register_t, interrupt_query_enabled_t
///     - Remove map_func_t, operation_func_t
///     - Remove mil-std-1553 device
///     - Remove rapidio device
///     - Remove VT_register_py_interface
///     - Remove VT_get_py_interface
///     - Add:
///         - VT_get_py_popaque_conf_object
///         - VT_python_wrap_conf_class
///         - VT_get_conf_class
///         - VT_get_py_opaque_transaction
///         - VT_python_wrap_transaction
///         - VT_get_py_opaque_generic_transaction
///         - VT_python_wrap_generic_transaction
///         - VT_get_py_opaque_x86_transaction_upcast
///         - VT_python_wrap_x86_transaction_upcast
///         - VT_get_py_opaque_ppc_transaction_upcast
///         - VT_python_wrap_ppc_transaction_upcast
///         - VT_get_py_opaque_pci_transaction_upcast
///         - VT_python_wrap_pci_transaction_upcast
///         - VT_get_py_opaque_mips_transaction_upcast
///         - VT_python_wrap_mips_transaction_upcast
///         - VT_get_py_opaque_arm_transaction_upcast
///         - VT_python_wrap_arm_transaction_upcast
///         - VT_get_exception_type
///     - Remove breakpoint_query_interface_t
///     - Remove pool_protect_interface_t
///     - Remove gui_mode_t
///     - Remove cpu_variant_t
///     - Remove workspace, gui_mode, cpu_mode, license_file, expire_time, alt_settings_dir, allow_license_gui, eclipse_params from init_prefs_t
///     - Remove hap_flags_t
///     - Remove int VT_write_rev(const void *src, int length)
///     - Remove int pr_rev(const char *format, ...);
///     - Change:
///         void pr_err_vararg(const char *prefix, bool is_error, const char *format, va_list ap);
///         (was void pr_err_vararg(const char *str, va_list ap);)
///      - Add void pr_warn(const char *str, ...);
///         - Add void SIM_printf_error(const char *str, ...);
///         - Add void SIM_printf_warning(const char *str, ...);
///     - Remove:
///         - bool VT_revexec_available();
///         - bool VT_revexec_active();
///         - bool VT_in_the_past();
///         - revexec_pos_t
///         - VT_revexec_steps
///         - VT_revexec_cycles
///         - VT_get_rewind_overhead
///         - VT_reverse
///         - VT_reverse_cpu
///         - VT_skipto_step
///         - VT_skipto_cycle
///         - VT_skipto_bookmark
///         - VT_rewind
///         - micro_checkpoint_flags_t
///         - VT_save_micro_checkpoint
///         - VT_restore_micro_checkpoint
///         - VT_delete_micro_checkpoint
///         - VT_in_time_order
///         - time_ordered_handler_t
///         - VT_c_in_time_order
///         - VT_revexec_ignore_class
///         - VT_revexec_barrier
///     - Remove:
///         - slave_time_t
///         - slave_time_as_sec
///         - slave_time_from_sec
///         - slave_time_from_ps
///         - slave_time_as_ps_hi
///         - slave_time_as_ps_lo
///         - slave_time_from_ps_int128
///         - slave_time_eq
///         - slave_time_lt
///         - slave_time_gt
///         - slave_time_le
///         - slave_time_ge
///     - Rename:
///         - VT_is_restoring_snapshot
///           to SIM_is_restoring_snapshot
///         - VT_take_snapshot
///           to SIM_take_snapshot
///         - VT_restore_snapshot
///           to SIM_restore_snapshot
///         - VT_delete_snapshot
///           to SIM_delete_snapshot
///         - VT_list_snapshots
///           to SIM_list_snapshots
///         - VT_get_snapshot_info
///           to SIM_get_snapshot_info
///     - Remove:
///         - telnet_connection_interface_t
///         - vnc_server_interface_t
///         - link_endpoint_interface_t
///         - probe_array_interface_t
///         - recorder_interface_t
///         - slave_agent_interface_t
///         - slaver_agent_interface_t
///         - mm_malloc_low
/// 7.0.0->7.1.0:
///     - Add pcie_hotplug_pd_t, pcie_hotplug_mrl_t, pcie_hotplug_events_interface_t, pcie_link_training_interface_t, pcie_link_speed_t, pcie_
///     - Remove attribute_monitor_interface_t
///     - Add flags to save_flags_t
///     - Add void SIM_write_persistent_state(const char *file, conf_object_t *root, save_flags_t flags);
///     - Remove VT_set_frontend_server, VT_send_startup_complete_message, VT_remove_control, frontend_server_interface_t
///     - Add probe_array_interface_t
///     - Remove probe_cache_interface_t
/// 7.2.0->7.3.0:
///     - Remove pr_err_vararg
///     - Add void SIM_printf_error_vararg(const char *format, va_list ap);
///     - Add void SIM_printf_warning_vararg(const char *format, va_list ap);
pub fn emit_cfg_directives() -> Result<()> {
    // Set configurations to conditionally enable experimental features that aren't
    // compatible with all supported SIMICS versions, based on the SIMICS version of the
    // low level bindings.

    let simics_api_version = versions::Versioning::new(simics_api_sys::SIMICS_VERSION)
        .ok_or_else(|| anyhow!("Invalid version {}", simics_api_sys::SIMICS_VERSION))?;

    // Exports a configuration directive indicating which Simics version is *compiled* against.
    println!(
        "cargo:rustc-cfg=simics_version_{}",
        simics_api_version.to_string().replace('.', "_")
    );

    println!(
        "cargo:rustc-cfg=simics_version_{}",
        simics_api_version
            .to_string()
            .split('.')
            .next()
            .ok_or_else(|| anyhow!("No major version found"))?
    );

    Ok(())
}

pub fn emit_link_info() -> Result<()> {
    #[cfg(unix)]
    const HOST_DIRNAME: &str = "linux64";

    #[cfg(not(unix))]
    const HOST_DIRNAME: &'static str = "win64";

    let base_dir_path = if let Ok(simics_base) = var("SIMICS_BASE") {
        PathBuf::from(simics_base)
    } else {
        println!("cargo:warning=No SIMICS_BASE environment variable found, using ispm to find installed packages and using latest base version");

        let mut packages = ispm::packages::list(&GlobalOptions::default())?;

        packages.sort();

        let Some(installed) = packages.installed_packages.as_ref() else {
            anyhow::bail!("No SIMICS_BASE variable set and did not get any installed packages");
        };
        let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
            anyhow::bail!(
                "No SIMICS_BASE variable set and did not find a package with package number 1000"
            );
        };
        println!("cargo:warning=Using Simics base version {}", base.version);
        base.paths
            .first()
            .ok_or_else(|| anyhow!("No paths found for package with package number 1000"))?
            .clone()
    };

    #[cfg(unix)]
    {
        // Link `libsimics-common.so`, `libvtutils.so`, and `libpythonX.XX.so.X.X` if they exist
        let bin_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("bin")
            .canonicalize()?;
        let libsimics_common = bin_dir.join("libsimics-common.so").canonicalize()?;

        let libvtutils = bin_dir.join("libvtutils.so").canonicalize()?;

        let sys_lib_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("sys")
            .join("lib")
            .canonicalize()?;

        let libpython = sys_lib_dir.join(
            read_dir(&sys_lib_dir)?
                .filter_map(|p| p.ok())
                .filter(|p| p.path().is_file())
                .filter(|p| {
                    let path = p.path();

                    let Some(file_name) = path.file_name() else {
                        return false;
                    };

                    let Some(file_name) = file_name.to_str() else {
                        return false;
                    };

                    file_name.starts_with("libpython")
                        && file_name.contains(".so")
                        && file_name != "libpython3.so"
                })
                .map(|p| p.path())
                .next()
                .ok_or_else(|| {
                    anyhow!("No libpythonX.XX.so.X.X found in {}", sys_lib_dir.display())
                })?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libsimics_common.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = [
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
        ]
        .join(":");

        println!("cargo:rustc-env=LD_LIBRARY_PATH={}", ld_library_path);
    }

    #[cfg(windows)]
    {
        // Link `libsimics-common.so`, `libvtutils.so`, and `libpythonX.XX.so.X.X` if they exist
        let bin_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("bin")
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find bin dir {:?}: {}",
                    base_dir_path.join(HOST_DIRNAME).join("bin"),
                    e
                )
            })?;

        let libsimics_common = bin_dir
            .join("libsimics-common.dll")
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find libsimics-common {:?}: {}",
                    bin_dir.join("libsimics-common.dll"),
                    e
                )
            })?;

        let libvtutils = bin_dir.join("libvtutils.dll").canonicalize().map_err(|e| {
            anyhow!(
                "Could not find libvtutils {:?}: {}",
                bin_dir.join("libvtutils.dll"),
                e
            )
        })?;

        let python_include_dir = subdir(base_dir_path.join(HOST_DIRNAME).join("include"))?;
        // .ok_or_else(|| anyhow!("Did not get any subdirectory of {:?}", base_dir_path.join(HOST_DIRNAME).join("include")))?;

        let python_dir_name = python_include_dir
            .components()
            .last()
            .ok_or_else(|| {
                anyhow!(
                    "Did not get any last component of path {:?}",
                    python_include_dir
                )
            })?
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Could not convert python include dir name to string"))?
            .to_string();

        let sys_lib_dir = base_dir_path
            .join(HOST_DIRNAME)
            .join("lib")
            .join(python_dir_name)
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Could not find sys lib dir {:?}: {}",
                    base_dir_path.join(HOST_DIRNAME).join("sys").join("lib"),
                    e
                )
            })?;

        let libpython = sys_lib_dir.join(
            read_dir(&sys_lib_dir)?
                .filter_map(|p| p.ok())
                .filter(|p| p.path().is_file())
                .filter(|p| {
                    let path = p.path();

                    let Some(file_name) = path.file_name() else {
                        return false;
                    };

                    let Some(file_name) = file_name.to_str() else {
                        return false;
                    };

                    file_name.starts_with("python")
                        && file_name.ends_with(".dll")
                        && file_name != "python3.dll"
                })
                .map(|p| p.path())
                .next()
                .ok_or_else(|| anyhow!("No pythonX.XX.dll found in {}", sys_lib_dir.display()))?,
        );

        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libsimics_common
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libsimics_common.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libvtutils
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libvtutils.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-lib=dylib:+verbatim={}",
            libpython
                .file_name()
                .ok_or_else(|| anyhow!("No file name found for {}", libpython.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        println!(
            "cargo:rustc-link-search=native={}",
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?
        );
        let ld_library_path = vec![
            bin_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
            sys_lib_dir
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert path to string"))?,
        ]
        .join(":");
    }

    // NOTE: EVEN with all of the above, a binary built using `cargo build` will not
    // be able to find libsimics-common.so. Instead, when we build a binary that
    // transitively depends on this -sys crate, we compile it with `cargo rustc`,
    // passing the `-rpath` link argument like so. Note `--disable-new-dtags`,
    // otherwise `libsimics-common.so` cannot find `libpython3.9.so.1.0` because it
    // will be missing the recursive rpath lookup.

    // SIMICS_BASE=/home/rhart/simics-public/simics-6.0.174
    // PYTHON3_INCLUDE=-I/home/rhart/simics-public/simics-6.0.174/linux64/include/python3.9
    // INCLUDE_PATHS=/home/rhart/simics-public/simics-6.0.174/src/include
    // PYTHON3_LDFLAGS=/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/libpython3.so
    // LDFLAGS="-L/home/rhart/simics-public/simics-6.0.174/linux64/bin -z noexecstack -z relro -z now" LIBS="-lsimics-common -lvtutils"
    // cargo --features=auto,link --example simple-simics -- -C link-args="-Wl,--disable-new-dtags -Wl,-rpath,/home/rhart/simics-public/simics-6.0.174/linux64/bin;/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/"
    //
    // This command (the environment variables can be left out) can be
    // auto-generated in the SIMICS makefile build system.

    Ok(())
}
