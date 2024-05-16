# Build Directives

The Simics bindings have some version-dependence on the underlying C API. The
`emit_build_directives` function from the `simics-build-utils` crate emits configuration
directives used in the build process to conditionally enable features that aren't
compatible with all supported SIMICS versions, based on the SIMICS version of the low
level bindings. This is not needed for all consumers of the API, but is useful for
consumers which need to remain compatible with a wide range of SIMICS base versions.

If your consumer of the Simics API needs to conditionally enable/disable its own
functionality depending on the version compiled against, you can call
`emit_build_directives` in your `build.rs` script and use the directives like:

```rust
#[cfg(any(simics_version_6_0_191, simics_version_7))]
```

## Changelog

The following is an abbreviated changelog for reference purposes.

6.0.163->6.0.164:
    - Add:
      -  `telnet_connection_v2_interface_t`
      - `vnc_server_v2_interface_t`

6.0.166->6.0.167:
    - Add `probe_notification_context_interface_t`

6.0.167->6.0.168:
    - Add:
      - `bool VT_attr_values_equal(attr_value_t a1, attr_value_t a2)`
      - `interrupt_subscriber_interface_t`
      - `interrupt_source_t`

6.0.169->6.0.170:
    - Add:
      - `riscv_imsic_interface_t`
      - `riscv_imsic_file_it_t`

6.0.170->6.0.171:
    - Add:
      - `Sim_Global_Notify_Message` variant to `global_notifier_type_t`

6.0.172->6.0.173:
    - Add:
      - `riscv_signal_sgeip_interface_t`
      - `Sim_Atom_Id_pcie_destination_segment` variant to `atom_id_t`
        - `void VT_save_snapshot(const char *name);`
        - `bool VT_restore_snapshot(int index);`
        - `bool VT_delete_snapshot(int index);`
        - `attr_value_t VT_snapshot_size_used();`
        - `attr_value_t VT_list_snapshots();`
        - `void VT_snapshots_ignore_class(const char *class_name);`
        - `vhdx_params_t`
        - `vhdx_file_t *vhdx_creat(const char *fname, uint64 size, craff_error_t *ce,
        vhdx_params_t *params);`
    - Change:
        - `void VT_load_target_preset_yml(const char *target, const char *ns, const char
        *preset, const char *preset_yml);` (was `void VT_load_target_preset_yml(const
        char *target, const char *ns, const char *preset_yml);`

6.0.173->6.0.174:
    - Change:
      - `bool VT_save_snapshot(const char *name);` (was `void VT_save_snapshot(const
      char *name);`)

6.0.174->6.0.175:
    - Add:
        - `Sim_Global_Notify_Snapshot_Will_Load` and
        `Sim_Global_Notify_Snapshot_Did_Load` variants to `global_notifier_type_t`
        - `void *VT_save_and_release_python_lock();`
        - `void VT_obtain_and_restore_python_lock(void *saved);`
    - Change:
      - `attr_value_t SIM_load_target(const char *target, const char *ns, attr_value_t
      presets, attr_value_t cmdline_args);` (was `void SIM_load_target(const char
      *target, const char *ns, attr_value_t presets, attr_value_t cmdline_args);`)

6.0.176->6.0.177:
    - Add:
        - `pcie_adapter_compat_interface_t`
        - `bool VT_is_loading_snapshot();`
    - Remove:
      - `probe_notification_context_interface_t`

6.0.177->6.0.178:
    - Add:
      - `void VT_snapshots_skip_class_resotre(conf_class_t *cls);`
      - `void VT_snapshots_skip_attr_restore(conf_class_t *cls, const char *attr_name);`
      - `attr_value_t VT_dump_snapshot(const char *name);`

6.0.179->6.0.180:
    - Add:
      - `snapshot_error_t`
      - `attr_value_t VT_get_snapshot_info(const char *name);`
    - Change:
        - `bool VT_is_restoring_snapshot();` (was `bool VT_is_loading_snapshot();`)
        - `snapshot_error_t VT_take_snapshot(const char *name);` (was `bool
        VT_save_snapshot(const char *name);`)
        - `snapshot_error_t VT_restore_snapshot(const char *name);` (was `bool
        VT_restore_snapshot(int index);`)
        - `snapshot_error_t VT_delete_snapshot(const char *name);` (was `bool
        VT_delete_snapshot(int index);`)

6.0.180->6.0.181:
    - Change:
      - `void VT_load_target_preset_yml(const char *target, const char *ns, attr_value_t
      presets, const char *preset_yml);` (was `void VT_load_target_preset_yml(const char
      *target, const char *ns, cosnt char *preset, const char *preset_yml);`)

6.0.183->6.0.184:
    - Add
      - `pcie_hotplug_events_interface_t`
      - `pcie_hotplug_pd_t`
      - `pcie_hotplug_mrl_t`
      - `probe_array_interface_t`
    - Remove:
      - `probe_cache_interface_t`

6.0.184->6.0.185:
    - Add:
      - `pcie_link_training_interface_t`
      - `pcie_link_speed_t`
      - `pcie_link_width_t`
      - `pcie_link_negotiation_t`

6.0.188->6.0.89:
    - Add:
      - `Sim_Log_Warning` variant to `log_type_t`
      - `void VT_log_warning(conf_object_t *dev, uint64 grp, const char *str, ...);`

6.X.X->7.0.0:
    - Remove:
        - `bool VT_is_reversing();`
        - `bool SIM_is_loading_micro_checkpoint(cont_object_t *obj);`
        - `Sim_Attr_Session and Sim_Attr_Doc variants from attr_type_t`
        - `i2c_device_state_t`
        - `i2c_device_flag_t`
        - `i2c_bus_interface_t`
        - `i2c_device_interface_t`
        - `i2c_status_t`
        - `i2c_link_interface_t`
        - `i2c_slave_interface_t`
        - `i2c_master_interface_t`
        - `i2c_bridge_interface_t`
        - `device_interrupt_t`
        - `device_interrupt_clear_t`
        - `interrupt_query_register_t`
        - `interrupt_query_enabled_t`
        - `map_func_t`
        - `operation_func_t`
        - `mil-std-1553` device
        - `rapidio` device
        - `VT_register_py_interface`
        - `VT_get_py_interface`
        - `breakpoint_query_interface_t`
        - `pool_protect_interface_t`
        - `gui_mode_t`
        - `cpu_variant_t`
        - `workspace`, `gui_mode`, `cpu_mode`, `license_file`, `expire_time`,
        `alt_settings_dir`, `allow_license_gui`, `eclipse_params` from `init_prefs_t`
        - `hap_flags_t`
        - `int VT_write_rev(const void *src, int length)`
        - `int pr_rev(const char *format, ...);`
        - `bool VT_revexec_available();`
        - `bool VT_revexec_active();`
        - `bool VT_in_the_past();`
        - `revexec_pos_t`
        - `VT_revexec_steps`
        - `VT_revexec_cycles`
        - `VT_get_rewind_overhead`
        - `VT_reverse`
        - `VT_reverse_cpu`
        - `VT_skipto_step`
        - `VT_skipto_cycle`
        - `VT_skipto_bookmark`
        - `VT_rewind`
        - `micro_checkpoint_flags_t`
        - `VT_save_micro_checkpoint`
        - `VT_restore_micro_checkpoint`
        - `VT_delete_micro_checkpoint`
        - `VT_in_time_order`
        - `time_ordered_handler_t`
        - `VT_c_in_time_order`
        - `VT_revexec_ignore_class`
        - `VT_revexec_barrier`
        - `slave_time_t`
        - `slave_time_as_sec`
        - `slave_time_from_sec`
        - `slave_time_from_ps`
        - `slave_time_as_ps_hi`
        - `slave_time_as_ps_lo`
        - `slave_time_from_ps_int128`
        - `slave_time_eq`
        - `slave_time_lt`
        - `slave_time_gt`
        - `slave_time_le`
        - `slave_time_ge`
        - `telnet_connection_interface_t`
        - `vnc_server_interface_t`
        - `link_endpoint_interface_t`
        - `probe_array_interface_t`
        - `recorder_interface_t`
        - `slave_agent_interface_t`
        - `slaver_agent_interface_t`
        - `mm_malloc_low`
    - Add:
        - `VT_get_py_popaque_conf_object`
        - `VT_python_wrap_conf_class`
        - `VT_get_conf_class`
        - `VT_get_py_opaque_transaction`
        - `VT_python_wrap_transaction`
        - `VT_get_py_opaque_generic_transaction`
        - `VT_python_wrap_generic_transaction`
        - `VT_get_py_opaque_x86_transaction_upcast`
        - `VT_python_wrap_x86_transaction_upcast`
        - `VT_get_py_opaque_ppc_transaction_upcast`
        - `VT_python_wrap_ppc_transaction_upcast`
        - `VT_get_py_opaque_pci_transaction_upcast`
        - `VT_python_wrap_pci_transaction_upcast`
        - `VT_get_py_opaque_mips_transaction_upcast`
        - `VT_python_wrap_mips_transaction_upcast`
        - `VT_get_py_opaque_arm_transaction_upcast`
        - `VT_python_wrap_arm_transaction_upcast`
        - `VT_get_exception_type`
        - `void pr_warn(const char *str, ...);`
        - `Add void SIM_printf_error(const char *str, ...);`
        - `Add void SIM_printf_warning(const char *str, ...);`
    - Change:
        `void pr_err_vararg(const char *prefix, bool is_error, const char *format,
        va_list ap);` (was `void pr_err_vararg(const char *str, va_list ap);`)
    - Rename:
        - `VT_is_restoring_snapshot` to `SIM_is_restoring_snapshot`
        - `VT_take_snapshot` to `SIM_take_snapshot`
        - `VT_restore_snapshot` to `SIM_restore_snapshot`
        - `VT_delete_snapshot` to `SIM_delete_snapshot`
        - `VT_list_snapshots` to `SIM_list_snapshots`
        - `VT_get_snapshot_info` to `SIM_get_snapshot_info`
7.0.0->7.1.0:
    - Add:
      - `pcie_hotplug_pd_t`
      - `pcie_hotplug_mrl_t`
      - `pcie_hotplug_events_interface_t`
      - `pcie_link_training_interface_t`
      - `pcie_link_speed_t`
      - `flags` to `save_flags_t`
      - `probe_array_interface_t`
      - `void SIM_write_persistent_state(const char *file, conf_object_t *root,
      save_flags_t flags);`
      - `VT_set_frontend_server`
      - `VT_send_startup_complete_message`
      - `VT_remove_control`
      - `frontend_server_interface_t`
    - Remove:
      -  `attribute_monitor_interface_t`
      - `probe_cache_interface_t`

7.2.0->7.3.0:
    - Add:
      - `void SIM_printf_error_vararg(const char *format, va_list ap);`
      - `void SIM_printf_warning_vararg(const char *format, va_list ap);`
    - Remove:
      - `pr_err_vararg`