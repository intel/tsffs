// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{last_error, ConfObject, GenericTransaction};
use anyhow::{bail, ensure, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{hap_handle_t, SIM_hap_add_callback};
use std::{
    ffi::{c_char, c_void},
    mem::transmute,
    ptr::null_mut,
};

pub type HapHandle = hap_handle_t;

/// Set of HAPs
pub enum Hap {
    // Base HAPs from API reference manual part 12
    Arinc429Word,
    CliCommandAdded,
    ComponentChange,
    ComponentHierarchyChange,
    ConsoleBreakString,
    CoreAddressNotMapped,
    CoreAsynchronousTrap,
    CoreAtExit,
    CoreBackToFront,
    CoreBreakpointChange,
    CoreBreakpointMemop,
    CoreCleanAtExit,
    CoreConfClassRegister,
    CoreConfClassUnregister,
    CoreConfClockChangeCell,
    CoreConfObjectChangeClock,
    CoreConfObjectCreate,
    CoreConfObjectCreated,
    CoreConfObjectDelete,
    CoreConfObjectPreDelete,
    CoreConfObjectRename,
    CoreConfObjectsCreated,
    CoreConfObjectsDeleted,
    CoreConfigurationLoaded,
    CoreContextActivate,
    CoreContextChange,
    CoreContextDeactivate,
    CoreContextUpdated,
    CoreContinuation,
    CoreControlRegisterRead,
    CoreControlRegisterWrite,
    CoreDeviceAccessMemop,
    CoreDisableBreakpoints,
    CoreDiscardFuture,
    CoreDstcFlushCounter,
    CoreException,
    CoreExceptionReturn,
    CoreExternalInterrupt,
    CoreFrequencyChanged,
    CoreGlobalMessage,
    CoreHapCallbackInstalled,
    CoreHapCallbackRemoved,
    CoreHapTypeAdded,
    CoreImageActivity,
    CoreInitialConfiguration,
    CoreLogGroupsChange,
    CoreLogLevelChange,
    CoreLogMessage,
    CoreLogMessageExtended,
    CoreLogMessageFiltered,
    CoreMagicInstruction,
    CoreMemorySpaceMapChanged,
    CoreModeChange,
    CoreModuleLoaded,
    CoreMulticoreAccelerationChanged,
    CoreMultithreadingChanged,
    CoreNotImplemented,
    CorePreferencesChanged,
    CoreProcessorScheduleChanged,
    CoreProjectChanged,
    CoreRecentFilesChanged,
    CoreRexecActive,
    CoreSimulationModeChange,
    CoreSimulationStopped,
    CoreSkiptoProgress,
    CoreSyncInstruction,
    CoreTimeTransition,
    CoreTimingModelChange,
    CoreUserCommentsChanged,
    CoreWriteConfiguration,
    EthInjectorPcapEof,
    FirewireReset,
    FirewireTransfer,
    GfxBreak,
    GfxBreakString,
    GraphicsConsoleNewTitle,
    GraphicsConsoleShowHide,
    InternalBookmarkListChanged,
    InternalBreakIo,
    InternalDeviceRegAccess,
    InternalMicroCheckpointLoaded,
    InternalSbWait,
    InternalTimeDirectionChanged,
    InternalTimeQuantumChanged,
    RealtimeEnabled,
    RecStateChanged,
    RexecLimitExceeded,
    RtcNvramUpdate,
    ScsiDiskCommand,
    SnNaptEnabled,
    TextConsoleNewTitle,
    TextConsoleShowHide,
    TlbFillData,
    TlbFillInstruction,
    TlbInvalidateData,
    TlbInvalidateInstruction,
    TlbMissData,
    TlbMissInstruction,
    TlbReplaceData,
    TlbReplaceInstruction,
    UiRecordStateChanged,
    UiRunStateChanged,
    VgaBreakString,
    VgaRefreshTriggered,
    XtermBreakString,

    // X86 QSP HAPs
    CoreInterruptStatus,
    CoreModeSwitch,
    CorePseudoException,
    X86DescriptorChange,
    X86EnterSmm,
    X86LeaveSmm,
    X86MisplacedRex,
    X86ProcessorReset,
    X86Sysenter,
    X86Sysexit,
    X86TripleFault,
    X86VmcsRead,
    X86VmcsWrite,
    X86VmxModeChange,

    // ARM HAPs
    ArmInstructionModeChange,
    ArmV8InterProcessing,
}

impl Hap {
    const HAP_ARINC429_WORD: &'static str = "Arinc429_Word";
    const HAP_CLI_COMMAND_ADDED: &'static str = "CLI_Command_Added";
    const HAP_COMPONENT_CHANGE: &'static str = "Component_Change";
    const HAP_COMPONENT_HIERARCHY_CHANGE: &'static str = "Component_Hierarchy_Change";
    const HAP_CONSOLE_BREAK_STRING: &'static str = "Console_Break_String";
    const HAP_CORE_ADDRESS_NOT_MAPPED: &'static str = "Core_Address_Not_Mapped";
    const HAP_CORE_ASYNCHRONOUS_TRAP: &'static str = "Core_Asynchronous_Trap";
    const HAP_CORE_AT_EXIT: &'static str = "Core_At_Exit";
    const HAP_CORE_BACK_TO_FRONT: &'static str = "Core_Back_To_Front";
    const HAP_CORE_BREAKPOINT_CHANGE: &'static str = "Core_Breakpoint_Change";
    const HAP_CORE_BREAKPOINT_MEMOP: &'static str = "Core_Breakpoint_Memop";
    const HAP_CORE_CLEAN_AT_EXIT: &'static str = "Core_Clean_At_Exit";
    const HAP_CORE_CONF_CLASS_REGISTER: &'static str = "Core_Conf_Class_Register";
    const HAP_CORE_CONF_CLASS_UNREGISTER: &'static str = "Core_Conf_Class_Unregister";
    const HAP_CORE_CONF_CLOCK_CHANGE_CELL: &'static str = "Core_Conf_Clock_Change_Cell";
    const HAP_CORE_CONF_OBJECT_CHANGE_CLOCK: &'static str = "Core_Conf_Object_Change_Clock";
    const HAP_CORE_CONF_OBJECT_CREATE: &'static str = "Core_Conf_Object_Create";
    const HAP_CORE_CONF_OBJECT_CREATED: &'static str = "Core_Conf_Object_Created";
    const HAP_CORE_CONF_OBJECT_DELETE: &'static str = "Core_Conf_Object_Delete";
    const HAP_CORE_CONF_OBJECT_PRE_DELETE: &'static str = "Core_Conf_Object_Pre_Delete";
    const HAP_CORE_CONF_OBJECT_RENAME: &'static str = "Core_Conf_Object_Rename";
    const HAP_CORE_CONF_OBJECTS_CREATED: &'static str = "Core_Conf_Objects_Created";
    const HAP_CORE_CONF_OBJECTS_DELETED: &'static str = "Core_Conf_Objects_Deleted";
    const HAP_CORE_CONFIGURATION_LOADED: &'static str = "Core_Configuration_Loaded";
    const HAP_CORE_CONTEXT_ACTIVATE: &'static str = "Core_Context_Activate";
    const HAP_CORE_CONTEXT_CHANGE: &'static str = "Core_Context_Change";
    const HAP_CORE_CONTEXT_DEACTIVATE: &'static str = "Core_Context_Deactivate";
    const HAP_CORE_CONTEXT_UPDATED: &'static str = "Core_Context_Updated";
    const HAP_CORE_CONTINUATION: &'static str = "Core_Continuation";
    const HAP_CORE_CONTROL_REGISTER_READ: &'static str = "Core_Control_Register_Read";
    const HAP_CORE_CONTROL_REGISTER_WRITE: &'static str = "Core_Control_Register_Write";
    const HAP_CORE_DEVICE_ACCESS_MEMOP: &'static str = "Core_Device_Access_Memop";
    const HAP_CORE_DISABLE_BREAKPOINTS: &'static str = "Core_Disable_Breakpoints";
    const HAP_CORE_DISCARD_FUTURE: &'static str = "Core_Discard_Future";
    const HAP_CORE_DSTC_FLUSH_COUNTER: &'static str = "Core_DSTC_Flush_Counter";
    const HAP_CORE_EXCEPTION: &'static str = "Core_Exception";
    const HAP_CORE_EXCEPTION_RETURN: &'static str = "Core_Exception_Return";
    const HAP_CORE_EXTERNAL_INTERRUPT: &'static str = "Core_External_Interrupt";
    const HAP_CORE_FREQUENCY_CHANGED: &'static str = "Core_Frequency_Changed";
    const HAP_CORE_GLOBAL_MESSAGE: &'static str = "Core_Global_Message";
    const HAP_CORE_HAP_CALLBACK_INSTALLED: &'static str = "Core_Hap_Callback_Installed";
    const HAP_CORE_HAP_CALLBACK_REMOVED: &'static str = "Core_Hap_Callback_Removed";
    const HAP_CORE_HAP_TYPE_ADDED: &'static str = "Core_Hap_Type_Added";
    const HAP_CORE_IMAGE_ACTIVITY: &'static str = "Core_Image_Activity";
    const HAP_CORE_INITIAL_CONFIGURATION: &'static str = "Core_Initial_Configuration";
    const HAP_CORE_LOG_GROUPS_CHANGE: &'static str = "Core_Log_Groups_Change";
    const HAP_CORE_LOG_LEVEL_CHANGE: &'static str = "Core_Log_Level_Change";
    const HAP_CORE_LOG_MESSAGE: &'static str = "Core_Log_Message";
    const HAP_CORE_LOG_MESSAGE_EXTENDED: &'static str = "Core_Log_Message_Extended";
    const HAP_CORE_LOG_MESSAGE_FILTERED: &'static str = "Core_Log_Message_Filtered";
    const HAP_CORE_MAGIC_INSTRUCTION: &'static str = "Core_Magic_Instruction";
    const HAP_CORE_MEMORY_SPACE_MAP_CHANGED: &'static str = "Core_Memory_Space_Map_Changed";
    const HAP_CORE_MODE_CHANGE: &'static str = "Core_Mode_Change";
    const HAP_CORE_MODULE_LOADED: &'static str = "Core_Module_Loaded";
    const HAP_CORE_MULTICORE_ACCELERATION_CHANGED: &'static str =
        "Core_Multicore_Acceleration_Changed";
    const HAP_CORE_MULTITHREADING_CHANGED: &'static str = "Core_Multithreading_Changed";
    const HAP_CORE_NOT_IMPLEMENTED: &'static str = "Core_Not_Implemented";
    const HAP_CORE_PREFERENCES_CHANGED: &'static str = "Core_Preferences_Changed";
    const HAP_CORE_PROCESSOR_SCHEDULE_CHANGED: &'static str = "Core_Processor_Schedule_Changed";
    const HAP_CORE_PROJECT_CHANGED: &'static str = "Core_Project_Changed";
    const HAP_CORE_RECENT_FILES_CHANGED: &'static str = "Core_Recent_Files_Changed";
    const HAP_CORE_REXEC_ACTIVE: &'static str = "Core_Rexec_Active";
    const HAP_CORE_SIMULATION_MODE_CHANGE: &'static str = "Core_Simulation_Mode_Change";
    const HAP_CORE_SIMULATION_STOPPED: &'static str = "Core_Simulation_Stopped";
    const HAP_CORE_SKIPTO_PROGRESS: &'static str = "Core_Skipto_Progress";
    const HAP_CORE_SYNC_INSTRUCTION: &'static str = "Core_Sync_Instruction";
    const HAP_CORE_TIME_TRANSITION: &'static str = "Core_Time_Transition";
    const HAP_CORE_TIMING_MODEL_CHANGE: &'static str = "Core_Timing_Model_Change";
    const HAP_CORE_USER_COMMENTS_CHANGED: &'static str = "Core_User_Comments_Changed";
    const HAP_CORE_WRITE_CONFIGURATION: &'static str = "Core_Write_Configuration";
    const HAP_ETH_INJECTOR_PCAP_EOF: &'static str = "Eth_Injector_Pcap_Eof";
    const HAP_FIREWIRE_RESET: &'static str = "Firewire_Reset";
    const HAP_FIREWIRE_TRANSFER: &'static str = "Firewire_Transfer";
    const HAP_GFX_BREAK: &'static str = "Gfx_Break";
    const HAP_GFX_BREAK_STRING: &'static str = "Gfx_Break_String";
    const HAP_GRAPHICS_CONSOLE_NEW_TITLE: &'static str = "Graphics_Console_New_Title";
    const HAP_GRAPHICS_CONSOLE_SHOW_HIDE: &'static str = "Graphics_Console_Show_Hide";
    const HAP_INTERNAL_BOOKMARK_LIST_CHANGED: &'static str = "Internal_Bookmark_List_Changed";
    const HAP_INTERNAL_BREAK_IO: &'static str = "Internal_Break_IO";
    const HAP_INTERNAL_DEVICE_REG_ACCESS: &'static str = "Internal_Device_Reg_Access";
    const HAP_INTERNAL_MICRO_CHECKPOINT_LOADED: &'static str = "Internal_Micro_Checkpoint_Loaded";
    const HAP_INTERNAL_SB_WAIT: &'static str = "Internal_SB_Wait";
    const HAP_INTERNAL_TIME_DIRECTION_CHANGED: &'static str = "Internal_Time_Direction_Changed";
    const HAP_INTERNAL_TIME_QUANTUM_CHANGED: &'static str = "Internal_Time_Quantum_Changed";
    const HAP_REALTIME_ENABLED: &'static str = "Realtime_Enabled";
    const HAP_REC_STATE_CHANGED: &'static str = "REC_State_Changed";
    const HAP_REXEC_LIMIT_EXCEEDED: &'static str = "Rexec_Limit_Exceeded";
    const HAP_RTC_NVRAM_UPDATE: &'static str = "RTC_Nvram_Update";
    const HAP_SCSI_DISK_COMMAND: &'static str = "SCSI_Disk_Command";
    const HAP_SN_NAPT_ENABLED: &'static str = "SN_NAPT_Enabled";
    const HAP_TEXT_CONSOLE_NEW_TITLE: &'static str = "Text_Console_New_Title";
    const HAP_TEXT_CONSOLE_SHOW_HIDE: &'static str = "Text_Console_Show_Hide";
    const HAP_TLB_FILL_DATA: &'static str = "TLB_Fill_Data";
    const HAP_TLB_FILL_INSTRUCTION: &'static str = "TLB_Fill_Instruction";
    const HAP_TLB_INVALIDATE_DATA: &'static str = "TLB_Invalidate_Data";
    const HAP_TLB_INVALIDATE_INSTRUCTION: &'static str = "TLB_Invalidate_Instruction";
    const HAP_TLB_MISS_DATA: &'static str = "TLB_Miss_Data";
    const HAP_TLB_MISS_INSTRUCTION: &'static str = "TLB_Miss_Instruction";
    const HAP_TLB_REPLACE_DATA: &'static str = "TLB_Replace_Data";
    const HAP_TLB_REPLACE_INSTRUCTION: &'static str = "TLB_Replace_Instruction";
    const HAP_UI_RECORD_STATE_CHANGED: &'static str = "UI_Record_State_Changed";
    const HAP_UI_RUN_STATE_CHANGED: &'static str = "UI_Run_State_Changed";
    const HAP_VGA_BREAK_STRING: &'static str = "Vga_Break_String";
    const HAP_VGA_REFRESH_TRIGGERED: &'static str = "Vga_Refresh_Triggered";
    const HAP_XTERM_BREAK_STRING: &'static str = "Xterm_Break_String";

    // X86 QSP HAPs
    const HAP_CORE_INTERRUPT_STATUS: &'static str = "Core_Interrupt_Status";
    const HAP_CORE_MODE_SWITCH: &'static str = "Core_Mode_Switch";
    const HAP_CORE_PSEUDO_EXCEPTION: &'static str = "Core_Pseudo_Exception";
    const HAP_X86_DESCRIPTOR_CHANGE: &'static str = "X86_Descriptor_Change";
    const HAP_X86_ENTER_SMM: &'static str = "X86_Enter_SMM";
    const HAP_X86_LEAVE_SMM: &'static str = "X86_Leave_SMM";
    const HAP_X86_MISPLACED_REX: &'static str = "X86_Misplaced_Rex";
    const HAP_X86_PROCESSOR_RESET: &'static str = "X86_Processor_Reset";
    const HAP_X86_SYSENTER: &'static str = "X86_Sysenter";
    const HAP_X86_SYSEXIT: &'static str = "X86_Sysexit";
    const HAP_X86_TRIPLE_FAULT: &'static str = "X86_Triple_Fault";
    const HAP_X86_VMCS_READ: &'static str = "X86_Vmcs_Read";
    const HAP_X86_VMCS_WRITE: &'static str = "X86_Vmcs_Write";
    const HAP_X86_VMX_MODE_CHANGE: &'static str = "X86_Vmx_Mode_Change";

    // ARM HAPs
    const HAP_ARM_INSTRUCTION_MODE_CHANGE: &'static str = "Arm_Instruction_Mode_Change";
    const HAP_ARM_V8_INTER_PROCESSING: &'static str = "Arm_V8_Inter_Processing";
}

impl ToString for Hap {
    /// Convert a HAP enum to the name of the HAP
    fn to_string(&self) -> String {
        match *self {
            Hap::Arinc429Word => Hap::HAP_ARINC429_WORD.to_string(),
            Hap::CliCommandAdded => Hap::HAP_CLI_COMMAND_ADDED.to_string(),
            Hap::ComponentChange => Hap::HAP_COMPONENT_CHANGE.to_string(),
            Hap::ComponentHierarchyChange => Hap::HAP_COMPONENT_HIERARCHY_CHANGE.to_string(),
            Hap::ConsoleBreakString => Hap::HAP_CONSOLE_BREAK_STRING.to_string(),
            Hap::CoreAddressNotMapped => Hap::HAP_CORE_ADDRESS_NOT_MAPPED.to_string(),
            Hap::CoreAsynchronousTrap => Hap::HAP_CORE_ASYNCHRONOUS_TRAP.to_string(),
            Hap::CoreAtExit => Hap::HAP_CORE_AT_EXIT.to_string(),
            Hap::CoreBackToFront => Hap::HAP_CORE_BACK_TO_FRONT.to_string(),
            Hap::CoreBreakpointChange => Hap::HAP_CORE_BREAKPOINT_CHANGE.to_string(),
            Hap::CoreBreakpointMemop => Hap::HAP_CORE_BREAKPOINT_MEMOP.to_string(),
            Hap::CoreCleanAtExit => Hap::HAP_CORE_CLEAN_AT_EXIT.to_string(),
            Hap::CoreConfClassRegister => Hap::HAP_CORE_CONF_CLASS_REGISTER.to_string(),
            Hap::CoreConfClassUnregister => Hap::HAP_CORE_CONF_CLASS_UNREGISTER.to_string(),
            Hap::CoreConfClockChangeCell => Hap::HAP_CORE_CONF_CLOCK_CHANGE_CELL.to_string(),
            Hap::CoreConfObjectChangeClock => Hap::HAP_CORE_CONF_OBJECT_CHANGE_CLOCK.to_string(),
            Hap::CoreConfObjectCreate => Hap::HAP_CORE_CONF_OBJECT_CREATE.to_string(),
            Hap::CoreConfObjectCreated => Hap::HAP_CORE_CONF_OBJECT_CREATED.to_string(),
            Hap::CoreConfObjectDelete => Hap::HAP_CORE_CONF_OBJECT_DELETE.to_string(),
            Hap::CoreConfObjectPreDelete => Hap::HAP_CORE_CONF_OBJECT_PRE_DELETE.to_string(),
            Hap::CoreConfObjectRename => Hap::HAP_CORE_CONF_OBJECT_RENAME.to_string(),
            Hap::CoreConfObjectsCreated => Hap::HAP_CORE_CONF_OBJECTS_CREATED.to_string(),
            Hap::CoreConfObjectsDeleted => Hap::HAP_CORE_CONF_OBJECTS_DELETED.to_string(),
            Hap::CoreConfigurationLoaded => Hap::HAP_CORE_CONFIGURATION_LOADED.to_string(),
            Hap::CoreContextActivate => Hap::HAP_CORE_CONTEXT_ACTIVATE.to_string(),
            Hap::CoreContextChange => Hap::HAP_CORE_CONTEXT_CHANGE.to_string(),
            Hap::CoreContextDeactivate => Hap::HAP_CORE_CONTEXT_DEACTIVATE.to_string(),
            Hap::CoreContextUpdated => Hap::HAP_CORE_CONTEXT_UPDATED.to_string(),
            Hap::CoreContinuation => Hap::HAP_CORE_CONTINUATION.to_string(),
            Hap::CoreControlRegisterRead => Hap::HAP_CORE_CONTROL_REGISTER_READ.to_string(),
            Hap::CoreControlRegisterWrite => Hap::HAP_CORE_CONTROL_REGISTER_WRITE.to_string(),
            Hap::CoreDeviceAccessMemop => Hap::HAP_CORE_DEVICE_ACCESS_MEMOP.to_string(),
            Hap::CoreDisableBreakpoints => Hap::HAP_CORE_DISABLE_BREAKPOINTS.to_string(),
            Hap::CoreDiscardFuture => Hap::HAP_CORE_DISCARD_FUTURE.to_string(),
            Hap::CoreDstcFlushCounter => Hap::HAP_CORE_DSTC_FLUSH_COUNTER.to_string(),
            Hap::CoreException => Hap::HAP_CORE_EXCEPTION.to_string(),
            Hap::CoreExceptionReturn => Hap::HAP_CORE_EXCEPTION_RETURN.to_string(),
            Hap::CoreExternalInterrupt => Hap::HAP_CORE_EXTERNAL_INTERRUPT.to_string(),
            Hap::CoreFrequencyChanged => Hap::HAP_CORE_FREQUENCY_CHANGED.to_string(),
            Hap::CoreGlobalMessage => Hap::HAP_CORE_GLOBAL_MESSAGE.to_string(),
            Hap::CoreHapCallbackInstalled => Hap::HAP_CORE_HAP_CALLBACK_INSTALLED.to_string(),
            Hap::CoreHapCallbackRemoved => Hap::HAP_CORE_HAP_CALLBACK_REMOVED.to_string(),
            Hap::CoreHapTypeAdded => Hap::HAP_CORE_HAP_TYPE_ADDED.to_string(),
            Hap::CoreImageActivity => Hap::HAP_CORE_IMAGE_ACTIVITY.to_string(),
            Hap::CoreInitialConfiguration => Hap::HAP_CORE_INITIAL_CONFIGURATION.to_string(),
            Hap::CoreLogGroupsChange => Hap::HAP_CORE_LOG_GROUPS_CHANGE.to_string(),
            Hap::CoreLogLevelChange => Hap::HAP_CORE_LOG_LEVEL_CHANGE.to_string(),
            Hap::CoreLogMessage => Hap::HAP_CORE_LOG_MESSAGE.to_string(),
            Hap::CoreLogMessageExtended => Hap::HAP_CORE_LOG_MESSAGE_EXTENDED.to_string(),
            Hap::CoreLogMessageFiltered => Hap::HAP_CORE_LOG_MESSAGE_FILTERED.to_string(),
            Hap::CoreMagicInstruction => Hap::HAP_CORE_MAGIC_INSTRUCTION.to_string(),
            Hap::CoreMemorySpaceMapChanged => Hap::HAP_CORE_MEMORY_SPACE_MAP_CHANGED.to_string(),
            Hap::CoreModeChange => Hap::HAP_CORE_MODE_CHANGE.to_string(),
            Hap::CoreModuleLoaded => Hap::HAP_CORE_MODULE_LOADED.to_string(),
            Hap::CoreMulticoreAccelerationChanged => {
                Hap::HAP_CORE_MULTICORE_ACCELERATION_CHANGED.to_string()
            }
            Hap::CoreMultithreadingChanged => Hap::HAP_CORE_MULTITHREADING_CHANGED.to_string(),
            Hap::CoreNotImplemented => Hap::HAP_CORE_NOT_IMPLEMENTED.to_string(),
            Hap::CorePreferencesChanged => Hap::HAP_CORE_PREFERENCES_CHANGED.to_string(),
            Hap::CoreProcessorScheduleChanged => {
                Hap::HAP_CORE_PROCESSOR_SCHEDULE_CHANGED.to_string()
            }
            Hap::CoreProjectChanged => Hap::HAP_CORE_PROJECT_CHANGED.to_string(),
            Hap::CoreRecentFilesChanged => Hap::HAP_CORE_RECENT_FILES_CHANGED.to_string(),
            Hap::CoreRexecActive => Hap::HAP_CORE_REXEC_ACTIVE.to_string(),
            Hap::CoreSimulationModeChange => Hap::HAP_CORE_SIMULATION_MODE_CHANGE.to_string(),
            Hap::CoreSimulationStopped => Hap::HAP_CORE_SIMULATION_STOPPED.to_string(),
            Hap::CoreSkiptoProgress => Hap::HAP_CORE_SKIPTO_PROGRESS.to_string(),
            Hap::CoreSyncInstruction => Hap::HAP_CORE_SYNC_INSTRUCTION.to_string(),
            Hap::CoreTimeTransition => Hap::HAP_CORE_TIME_TRANSITION.to_string(),
            Hap::CoreTimingModelChange => Hap::HAP_CORE_TIMING_MODEL_CHANGE.to_string(),
            Hap::CoreUserCommentsChanged => Hap::HAP_CORE_USER_COMMENTS_CHANGED.to_string(),
            Hap::CoreWriteConfiguration => Hap::HAP_CORE_WRITE_CONFIGURATION.to_string(),
            Hap::EthInjectorPcapEof => Hap::HAP_ETH_INJECTOR_PCAP_EOF.to_string(),
            Hap::FirewireReset => Hap::HAP_FIREWIRE_RESET.to_string(),
            Hap::FirewireTransfer => Hap::HAP_FIREWIRE_TRANSFER.to_string(),
            Hap::GfxBreak => Hap::HAP_GFX_BREAK.to_string(),
            Hap::GfxBreakString => Hap::HAP_GFX_BREAK_STRING.to_string(),
            Hap::GraphicsConsoleNewTitle => Hap::HAP_GRAPHICS_CONSOLE_NEW_TITLE.to_string(),
            Hap::GraphicsConsoleShowHide => Hap::HAP_GRAPHICS_CONSOLE_SHOW_HIDE.to_string(),
            Hap::InternalBookmarkListChanged => Hap::HAP_INTERNAL_BOOKMARK_LIST_CHANGED.to_string(),
            Hap::InternalBreakIo => Hap::HAP_INTERNAL_BREAK_IO.to_string(),
            Hap::InternalDeviceRegAccess => Hap::HAP_INTERNAL_DEVICE_REG_ACCESS.to_string(),
            Hap::InternalMicroCheckpointLoaded => {
                Hap::HAP_INTERNAL_MICRO_CHECKPOINT_LOADED.to_string()
            }
            Hap::InternalSbWait => Hap::HAP_INTERNAL_SB_WAIT.to_string(),
            Hap::InternalTimeDirectionChanged => {
                Hap::HAP_INTERNAL_TIME_DIRECTION_CHANGED.to_string()
            }
            Hap::InternalTimeQuantumChanged => Hap::HAP_INTERNAL_TIME_QUANTUM_CHANGED.to_string(),
            Hap::RealtimeEnabled => Hap::HAP_REALTIME_ENABLED.to_string(),
            Hap::RecStateChanged => Hap::HAP_REC_STATE_CHANGED.to_string(),
            Hap::RexecLimitExceeded => Hap::HAP_REXEC_LIMIT_EXCEEDED.to_string(),
            Hap::RtcNvramUpdate => Hap::HAP_RTC_NVRAM_UPDATE.to_string(),
            Hap::ScsiDiskCommand => Hap::HAP_SCSI_DISK_COMMAND.to_string(),
            Hap::SnNaptEnabled => Hap::HAP_SN_NAPT_ENABLED.to_string(),
            Hap::TextConsoleNewTitle => Hap::HAP_TEXT_CONSOLE_NEW_TITLE.to_string(),
            Hap::TextConsoleShowHide => Hap::HAP_TEXT_CONSOLE_SHOW_HIDE.to_string(),
            Hap::TlbFillData => Hap::HAP_TLB_FILL_DATA.to_string(),
            Hap::TlbFillInstruction => Hap::HAP_TLB_FILL_INSTRUCTION.to_string(),
            Hap::TlbInvalidateData => Hap::HAP_TLB_INVALIDATE_DATA.to_string(),
            Hap::TlbInvalidateInstruction => Hap::HAP_TLB_INVALIDATE_INSTRUCTION.to_string(),
            Hap::TlbMissData => Hap::HAP_TLB_MISS_DATA.to_string(),
            Hap::TlbMissInstruction => Hap::HAP_TLB_MISS_INSTRUCTION.to_string(),
            Hap::TlbReplaceData => Hap::HAP_TLB_REPLACE_DATA.to_string(),
            Hap::TlbReplaceInstruction => Hap::HAP_TLB_REPLACE_INSTRUCTION.to_string(),
            Hap::UiRecordStateChanged => Hap::HAP_UI_RECORD_STATE_CHANGED.to_string(),
            Hap::UiRunStateChanged => Hap::HAP_UI_RUN_STATE_CHANGED.to_string(),
            Hap::VgaBreakString => Hap::HAP_VGA_BREAK_STRING.to_string(),
            Hap::VgaRefreshTriggered => Hap::HAP_VGA_REFRESH_TRIGGERED.to_string(),
            Hap::XtermBreakString => Hap::HAP_XTERM_BREAK_STRING.to_string(),
            Hap::CoreInterruptStatus => Hap::HAP_CORE_INTERRUPT_STATUS.to_string(),
            Hap::CoreModeSwitch => Hap::HAP_CORE_MODE_SWITCH.to_string(),
            Hap::CorePseudoException => Hap::HAP_CORE_PSEUDO_EXCEPTION.to_string(),
            Hap::X86DescriptorChange => Hap::HAP_X86_DESCRIPTOR_CHANGE.to_string(),
            Hap::X86EnterSmm => Hap::HAP_X86_ENTER_SMM.to_string(),
            Hap::X86LeaveSmm => Hap::HAP_X86_LEAVE_SMM.to_string(),
            Hap::X86MisplacedRex => Hap::HAP_X86_MISPLACED_REX.to_string(),
            Hap::X86ProcessorReset => Hap::HAP_X86_PROCESSOR_RESET.to_string(),
            Hap::X86Sysenter => Hap::HAP_X86_SYSENTER.to_string(),
            Hap::X86Sysexit => Hap::HAP_X86_SYSEXIT.to_string(),
            Hap::X86TripleFault => Hap::HAP_X86_TRIPLE_FAULT.to_string(),
            Hap::X86VmcsRead => Hap::HAP_X86_VMCS_READ.to_string(),
            Hap::X86VmcsWrite => Hap::HAP_X86_VMCS_WRITE.to_string(),
            Hap::X86VmxModeChange => Hap::HAP_X86_VMX_MODE_CHANGE.to_string(),
            Hap::ArmInstructionModeChange => Hap::HAP_ARM_INSTRUCTION_MODE_CHANGE.to_string(),
            Hap::ArmV8InterProcessing => Hap::HAP_ARM_V8_INTER_PROCESSING.to_string(),
        }
    }
}

// all HAPs are called with at least two parameters: callback_data: *mut c_void and
// trigger_obj: *mut ConfObject

pub type Arinc429WordCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type ArmInstructionModeChangeCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type ArmV8InterProcessingCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CliCommandAddedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    command_name: *mut c_char,
);
pub type ComponentChangeCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type ComponentHierarchyChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    tope_level_component: *mut ConfObject,
);
pub type ConsoleBreakStringCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    break_string: *mut c_char,
);
pub type CoreAddressNotMappedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    physical_address: i64,
    access_type: i64,
);
pub type CoreAsynchronousTrapCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    trap_number: i64,
);
pub type CoreAtExitCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreBackToFrontCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreBreakpointChangeCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreBreakpointMemopCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    breakpoint_number: i64,
    memory_operation: *mut GenericTransaction,
);
pub type CoreCleanAtExitCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfClassRegisterCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    class_name: *mut c_char,
);
pub type CoreConfClassUnregisterCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    class_name: *mut c_char,
);
pub type CoreConfClockChangeCellCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    old_cell: *mut ConfObject,
    new_cell: *mut ConfObject,
);

pub type CoreConfObjectChangeClockCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfObjectCreateCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfObjectCreatedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfObjectDeleteCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    object_name: *mut c_char,
);
pub type CoreConfObjectPreDeleteCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfObjectRenameCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    old_name: *mut c_char,
);
pub type CoreConfObjectsCreatedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfObjectsDeletedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreConfigurationLoadedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreContextActivateCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    other_ctx: *mut ConfObject,
    cpu: *mut ConfObject,
);
pub type CoreContextChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    cpu: *mut ConfObject,
);
pub type CoreContextDeactivateCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    other_ctx: *mut ConfObject,
    cpu: *mut ConfObject,
);
pub type CoreContextUpdatedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreContinuationCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreControlRegisterReadCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    register_number: i64,
);
pub type CoreControlRegisterWriteCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    register_number: i64,
    register_value: i64,
);
pub type CoreDeviceAccessMemopCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    memop: *mut GenericTransaction,
);

pub type CoreDisableBreakpointsCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, reenable: i32);
pub type CoreDiscardFutureCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, *mut ConfObject);
pub type CoreDstcFlushCounterCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    typ: i64,
    virtual_address: i64,
    physical_address: i64,
    counter: i64,
);
pub type CoreExceptionCallback =
    unsafe extern "C" fn(data: *mut c_void, trigger_obj: *mut ConfObject, exception_number: i64);
pub type CoreExceptionReturnCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    exception_number: i64,
);
pub type CoreExternalInterruptCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, source_mid: i64);
pub type CoreFrequencyChangedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    old_freq: i64,
    new_freq: i64,
);
pub type CoreGlobalMessageCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    message: *mut c_char,
);
pub type CoreHapCallbackInstalledCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    hap_number: i64,
    range_low: i64,
    range_high: i64,
);
pub type CoreHapCallbackRemovedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    hap_number: i64,
    range_low: i64,
    range_high: i64,
);
pub type CoreHapTypeAddedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    hap_name: *mut c_char,
);
pub type CoreImageActivityCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    typ: i32,
    onoff: i32,
);
pub type CoreInitialConfigurationCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreInterruptStatusCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, status: i64);
pub type CoreLogGroupsChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    log_group_mask: i32,
);
pub type CoreLogLevelChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    new_log_level: i32,
);
pub type CoreLogMessageCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    typ: i32,
    message: *mut c_char,
);
pub type CoreLogMessageExtendedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    typ: i32,
    message: *mut c_char,
    level: i32,
    group: i64,
);
pub type CoreLogMessageFilteredCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    typ: i32,
    message: *mut c_char,
    level: i32,
    group: i64,
);
pub type CoreMagicInstructionCallback =
    unsafe extern "C" fn(*mut c_void, *mut ConfObject, parameter: i64);
pub type CoreMemorySpaceMapChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreModeChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    old_mode: i64,
    new_mode: i64,
);
pub type CoreModeSwitchCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, mode: i64);
pub type CoreModuleLoadedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    module_name: *mut c_char,
);
pub type CoreMulticoreAccelerationChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, onoff: i32);
pub type CoreMultithreadingChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, onoff: i32);
pub type CoreNotImplementedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    line: i32,
    file: *mut c_char,
    rcsid: *mut c_char,
    message: *mut c_char,
    data: i64,
);
pub type CorePreferencesChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreProcessorScheduleChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreProjectChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CorePseudoExceptionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    exception_number: i64,
);
pub type CoreRecentFilesChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreRexecActiveCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    active_flag: i32,
);
pub type CoreSimulationModeChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    simulation_mode: i32,
);

/// exception is *always* SimExc::NoException, error_string is always NULL
pub type CoreSimulationStoppedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    exception: i64,
    error_string: *mut c_char,
);

pub type CoreSkiptoProgressCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, progress: i32);
pub type CoreSyncInstructionCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, typ: i64);
pub type CoreTimeTransitionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    in_the_past: i32,
);
pub type CoreTimingModelChangeCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreUserCommentsChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type CoreWriteConfigurationCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    file_name: *mut c_char,
);
pub type EthInjectorPcapEofCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    pcap_file: *mut c_char,
    num_injected: i32,
    pcap_num_pkgs: i32,
    auto_restart: i32,
);
pub type FirewireResetCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type FirewireTransferCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type GfxBreakCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, gfx_break: i64);
pub type GfxBreakStringCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, break_id: i64);
pub type GraphicsConsoleNewTitleCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    title: *mut c_char,
);
pub type GraphicsConsoleShowHideCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, state: i32);
pub type InternalBookmarkListChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type InternalBreakIoCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, break_id: i32);
pub type InternalDeviceRegAccessCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    memop: *mut GenericTransaction,
    port: *mut c_char,
    idx: i32,
    func: i32,
    offset: i64,
);
pub type InternalMicroCheckpointLoadedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type InternalSbWaitCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type InternalTimeDirectionChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, rev: i32);
pub type InternalTimeQuantumChangedCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type RealtimeEnabledCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, enabled: i32);
pub type RecStateChangedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    recording: i32,
    playback: i32,
);
pub type RexecLimitExceededCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, limit_type: i32);
pub type RtcNvramUpdateCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    index: i64,
    old_value: i64,
    new_value: i64,
);
pub type ScsiDiskCommandCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    command_number: i64,
    start: i64,
    len: i64,
);
pub type SnNaptEnabledCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, enabled: i32);
pub type TextConsoleNewTitleCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    new_title: *mut c_char,
);
pub type TextConsoleShowHideCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, is_shown: i32);
pub type TlbFillDataCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type TlbFillInstructionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type TlbInvalidateDataCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type TlbInvalidateInstructionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type TlbMissDataCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear_address: i64,
);
pub type TlbMissInstructionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear_address: i64,
);
pub type TlbReplaceDataCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type TlbReplaceInstructionCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    linear: i64,
    physical: i64,
    page_size: i64,
);
pub type UiRecordStateChangedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    record: i32,
    playback: i32,
);
pub type UiRunStateChangedCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    state: *mut c_char,
);
pub type VgaBreakStringCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    string: *mut c_char,
);
pub type VgaRefreshTriggeredCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type X86DescriptorChangeCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    // Segment number: ES=0, CS=1, SS=2, DS=3, FS=4, and GS=5
    segment_number: i64,
);
pub type X86EnterSmmCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, phase: i32);
pub type X86LeaveSmmCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, phase: i32);
pub type X86MisplacedRexCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type X86ProcessorResetCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, hard_reset: i32);
pub type X86SysenterCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, kind: i32);
pub type X86SysexitCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, kind: i32);
pub type X86TripleFaultCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject);
pub type X86VmcsReadCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    field_index: i64,
);
pub type X86VmcsWriteCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    field_index: i64,
    value: i64,
);
pub type X86VmxModeChangeCallback =
    unsafe extern "C" fn(callback_data: *mut c_void, trigger_obj: *mut ConfObject, mode: i64);
pub type XtermBreakStringCallback = unsafe extern "C" fn(
    callback_data: *mut c_void,
    trigger_obj: *mut ConfObject,
    break_string: *mut c_char,
);

/// Types for HAP callbacks
pub enum HapCallback {
    // Base HAPs from API reference manual part 12
    Arinc429Word(Arinc429WordCallback),
    CliCommandAdded(CliCommandAddedCallback),
    ComponentChange(ComponentChangeCallback),
    ComponentHierarchyChange(ComponentHierarchyChangeCallback),
    ConsoleBreakString(ConsoleBreakStringCallback),
    CoreAddressNotMapped(CoreAddressNotMappedCallback),
    CoreAsynchronousTrap(CoreAsynchronousTrapCallback),
    CoreAtExit(CoreAtExitCallback),
    CoreBackToFront(CoreBackToFrontCallback),
    CoreBreakpointChange(CoreBreakpointChangeCallback),
    CoreBreakpointMemop(CoreBreakpointMemopCallback),
    CoreCleanAtExit(CoreCleanAtExitCallback),
    CoreConfClassRegister(CoreConfClassRegisterCallback),
    CoreConfClassUnregister(CoreConfClassUnregisterCallback),
    CoreConfClockChangeCell(CoreConfClockChangeCellCallback),
    CoreConfObjectChangeClock(CoreConfObjectChangeClockCallback),
    CoreConfObjectCreate(CoreConfObjectCreateCallback),
    CoreConfObjectCreated(CoreConfObjectCreatedCallback),
    CoreConfObjectDelete(CoreConfObjectDeleteCallback),
    CoreConfObjectPreDelete(CoreConfObjectPreDeleteCallback),
    CoreConfObjectRename(CoreConfObjectRenameCallback),
    CoreConfObjectsCreated(CoreConfObjectsCreatedCallback),
    CoreConfObjectsDeleted(CoreConfObjectsDeletedCallback),
    CoreConfigurationLoaded(CoreConfigurationLoadedCallback),
    CoreContextActivate(CoreContextActivateCallback),
    CoreContextChange(CoreContextChangeCallback),
    CoreContextDeactivate(CoreContextDeactivateCallback),
    CoreContextUpdated(CoreContextUpdatedCallback),
    CoreContinuation(CoreContinuationCallback),
    CoreControlRegisterRead(CoreControlRegisterReadCallback),
    CoreControlRegisterWrite(CoreControlRegisterWriteCallback),
    CoreDeviceAccessMemop(CoreDeviceAccessMemopCallback),
    CoreDisableBreakpoints(CoreDisableBreakpointsCallback),
    CoreDiscardFuture(CoreDiscardFutureCallback),
    CoreDstcFlushCounter(CoreDstcFlushCounterCallback),
    CoreException(CoreExceptionCallback),
    CoreExceptionReturn(CoreExceptionReturnCallback),
    CoreExternalInterrupt(CoreExternalInterruptCallback),
    CoreFrequencyChanged(CoreFrequencyChangedCallback),
    CoreGlobalMessage(CoreGlobalMessageCallback),
    CoreHapCallbackInstalled(CoreHapCallbackInstalledCallback),
    CoreHapCallbackRemoved(CoreHapCallbackRemovedCallback),
    CoreHapTypeAdded(CoreHapTypeAddedCallback),
    CoreImageActivity(CoreImageActivityCallback),
    CoreInitialConfiguration(CoreInitialConfigurationCallback),
    CoreLogGroupsChange(CoreLogGroupsChangeCallback),
    CoreLogLevelChange(CoreLogLevelChangeCallback),
    CoreLogMessage(CoreLogMessageCallback),
    CoreLogMessageExtended(CoreLogMessageExtendedCallback),
    CoreLogMessageFiltered(CoreLogMessageFilteredCallback),
    CoreMagicInstruction(CoreMagicInstructionCallback),
    CoreMemorySpaceMapChanged(CoreMemorySpaceMapChangedCallback),
    CoreModeChange(CoreModeChangeCallback),
    CoreModuleLoaded(CoreModuleLoadedCallback),
    CoreMulticoreAccelerationChanged(CoreMulticoreAccelerationChangedCallback),
    CoreMultithreadingChanged(CoreMultithreadingChangedCallback),
    CoreNotImplemented(CoreNotImplementedCallback),
    CorePreferencesChanged(CorePreferencesChangedCallback),
    CoreProcessorScheduleChanged(CoreProcessorScheduleChangedCallback),
    CoreProjectChanged(CoreProjectChangedCallback),
    CoreRecentFilesChanged(CoreRecentFilesChangedCallback),
    CoreRexecActive(CoreRexecActiveCallback),
    CoreSimulationModeChange(CoreSimulationModeChangeCallback),
    CoreSimulationStopped(CoreSimulationStoppedCallback),
    CoreSkiptoProgress(CoreSkiptoProgressCallback),
    CoreSyncInstruction(CoreSyncInstructionCallback),
    CoreTimeTransition(CoreTimeTransitionCallback),
    CoreTimingModelChange(CoreTimingModelChangeCallback),
    CoreUserCommentsChanged(CoreUserCommentsChangedCallback),
    CoreWriteConfiguration(CoreWriteConfigurationCallback),
    EthInjectorPcapEof(EthInjectorPcapEofCallback),
    FirewireReset(FirewireResetCallback),
    FirewireTransfer(FirewireTransferCallback),
    GfxBreak(GfxBreakCallback),
    GfxBreakString(GfxBreakStringCallback),
    GraphicsConsoleNewTitle(GraphicsConsoleNewTitleCallback),
    GraphicsConsoleShowHide(GraphicsConsoleShowHideCallback),
    InternalBookmarkListChanged(InternalBookmarkListChangedCallback),
    InternalBreakIo(InternalBreakIoCallback),
    InternalDeviceRegAccess(InternalDeviceRegAccessCallback),
    InternalMicroCheckpointLoaded(InternalMicroCheckpointLoadedCallback),
    InternalSbWait(InternalSbWaitCallback),
    InternalTimeDirectionChanged(InternalTimeDirectionChangedCallback),
    InternalTimeQuantumChanged(InternalTimeQuantumChangedCallback),
    RealtimeEnabled(RealtimeEnabledCallback),
    RecStateChanged(RecStateChangedCallback),
    RexecLimitExceeded(RexecLimitExceededCallback),
    RtcNvramUpdate(RtcNvramUpdateCallback),
    ScsiDiskCommand(ScsiDiskCommandCallback),
    SnNaptEnabled(SnNaptEnabledCallback),
    TextConsoleNewTitle(TextConsoleNewTitleCallback),
    TextConsoleShowHide(TextConsoleShowHideCallback),
    TlbFillData(TlbFillDataCallback),
    TlbFillInstruction(TlbFillInstructionCallback),
    TlbInvalidateData(TlbInvalidateDataCallback),
    TlbInvalidateInstruction(TlbInvalidateInstructionCallback),
    TlbMissData(TlbMissDataCallback),
    TlbMissInstruction(TlbMissInstructionCallback),
    TlbReplaceData(TlbReplaceDataCallback),
    TlbReplaceInstruction(TlbReplaceInstructionCallback),
    UiRecordStateChanged(UiRecordStateChangedCallback),
    UiRunStateChanged(UiRunStateChangedCallback),
    VgaBreakString(VgaBreakStringCallback),
    VgaRefreshTriggered(VgaRefreshTriggeredCallback),
    XtermBreakString(XtermBreakStringCallback),

    // X86 QSP HAPs
    CoreInterruptStatus(CoreInterruptStatusCallback),
    CoreModeSwitch(CoreModeSwitchCallback),
    CorePseudoException(CorePseudoExceptionCallback),
    X86DescriptorChange(X86DescriptorChangeCallback),
    X86EnterSmm(X86EnterSmmCallback),
    X86LeaveSmm(X86LeaveSmmCallback),
    X86MisplacedRex(X86MisplacedRexCallback),
    X86ProcessorReset(X86ProcessorResetCallback),
    X86Sysenter(X86SysenterCallback),
    X86Sysexit(X86SysexitCallback),
    X86TripleFault(X86TripleFaultCallback),
    X86VmcsRead(X86VmcsReadCallback),
    X86VmcsWrite(X86VmcsWriteCallback),
    X86VmxModeChange(X86VmxModeChangeCallback),

    // ARM HAPs
    ArmInstructionModeChange(ArmInstructionModeChangeCallback),
    ArmV8InterProcessing(ArmV8InterProcessingCallback),
}

impl HapCallback {
    /// Coerce any HAP callback type to a CFFI compatible function pointer that takes no arguments
    /// and returns no data, required by the SIMICS API
    pub fn as_fn(&self) -> extern "C" fn() {
        unsafe {
            match *self {
                HapCallback::Arinc429Word(func) => transmute(func),
                HapCallback::CliCommandAdded(func) => transmute(func),
                HapCallback::ComponentChange(func) => transmute(func),
                HapCallback::ComponentHierarchyChange(func) => transmute(func),
                HapCallback::ConsoleBreakString(func) => transmute(func),
                HapCallback::CoreAddressNotMapped(func) => transmute(func),
                HapCallback::CoreAsynchronousTrap(func) => transmute(func),
                HapCallback::CoreAtExit(func) => transmute(func),
                HapCallback::CoreBackToFront(func) => transmute(func),
                HapCallback::CoreBreakpointChange(func) => transmute(func),
                HapCallback::CoreBreakpointMemop(func) => transmute(func),
                HapCallback::CoreCleanAtExit(func) => transmute(func),
                HapCallback::CoreConfClassRegister(func) => transmute(func),
                HapCallback::CoreConfClassUnregister(func) => transmute(func),
                HapCallback::CoreConfClockChangeCell(func) => transmute(func),
                HapCallback::CoreConfObjectChangeClock(func) => transmute(func),
                HapCallback::CoreConfObjectCreate(func) => transmute(func),
                HapCallback::CoreConfObjectCreated(func) => transmute(func),
                HapCallback::CoreConfObjectDelete(func) => transmute(func),
                HapCallback::CoreConfObjectPreDelete(func) => transmute(func),
                HapCallback::CoreConfObjectRename(func) => transmute(func),
                HapCallback::CoreConfObjectsCreated(func) => transmute(func),
                HapCallback::CoreConfObjectsDeleted(func) => transmute(func),
                HapCallback::CoreConfigurationLoaded(func) => transmute(func),
                HapCallback::CoreContextActivate(func) => transmute(func),
                HapCallback::CoreContextChange(func) => transmute(func),
                HapCallback::CoreContextDeactivate(func) => transmute(func),
                HapCallback::CoreContextUpdated(func) => transmute(func),
                HapCallback::CoreContinuation(func) => transmute(func),
                HapCallback::CoreControlRegisterRead(func) => transmute(func),
                HapCallback::CoreControlRegisterWrite(func) => transmute(func),
                HapCallback::CoreDeviceAccessMemop(func) => transmute(func),
                HapCallback::CoreDisableBreakpoints(func) => transmute(func),
                HapCallback::CoreDiscardFuture(func) => transmute(func),
                HapCallback::CoreDstcFlushCounter(func) => transmute(func),
                HapCallback::CoreException(func) => transmute(func),
                HapCallback::CoreExceptionReturn(func) => transmute(func),
                HapCallback::CoreExternalInterrupt(func) => transmute(func),
                HapCallback::CoreFrequencyChanged(func) => transmute(func),
                HapCallback::CoreGlobalMessage(func) => transmute(func),
                HapCallback::CoreHapCallbackInstalled(func) => transmute(func),
                HapCallback::CoreHapCallbackRemoved(func) => transmute(func),
                HapCallback::CoreHapTypeAdded(func) => transmute(func),
                HapCallback::CoreImageActivity(func) => transmute(func),
                HapCallback::CoreInitialConfiguration(func) => transmute(func),
                HapCallback::CoreLogGroupsChange(func) => transmute(func),
                HapCallback::CoreLogLevelChange(func) => transmute(func),
                HapCallback::CoreLogMessage(func) => transmute(func),
                HapCallback::CoreLogMessageExtended(func) => transmute(func),
                HapCallback::CoreLogMessageFiltered(func) => transmute(func),
                HapCallback::CoreMagicInstruction(func) => transmute(func),
                HapCallback::CoreMemorySpaceMapChanged(func) => transmute(func),
                HapCallback::CoreModeChange(func) => transmute(func),
                HapCallback::CoreModuleLoaded(func) => transmute(func),
                HapCallback::CoreMulticoreAccelerationChanged(func) => transmute(func),
                HapCallback::CoreMultithreadingChanged(func) => transmute(func),
                HapCallback::CoreNotImplemented(func) => transmute(func),
                HapCallback::CorePreferencesChanged(func) => transmute(func),
                HapCallback::CoreProcessorScheduleChanged(func) => transmute(func),
                HapCallback::CoreProjectChanged(func) => transmute(func),
                HapCallback::CoreRecentFilesChanged(func) => transmute(func),
                HapCallback::CoreRexecActive(func) => transmute(func),
                HapCallback::CoreSimulationModeChange(func) => transmute(func),
                HapCallback::CoreSimulationStopped(func) => transmute(func),
                HapCallback::CoreSkiptoProgress(func) => transmute(func),
                HapCallback::CoreSyncInstruction(func) => transmute(func),
                HapCallback::CoreTimeTransition(func) => transmute(func),
                HapCallback::CoreTimingModelChange(func) => transmute(func),
                HapCallback::CoreUserCommentsChanged(func) => transmute(func),
                HapCallback::CoreWriteConfiguration(func) => transmute(func),
                HapCallback::EthInjectorPcapEof(func) => transmute(func),
                HapCallback::FirewireReset(func) => transmute(func),
                HapCallback::FirewireTransfer(func) => transmute(func),
                HapCallback::GfxBreak(func) => transmute(func),
                HapCallback::GfxBreakString(func) => transmute(func),
                HapCallback::GraphicsConsoleNewTitle(func) => transmute(func),
                HapCallback::GraphicsConsoleShowHide(func) => transmute(func),
                HapCallback::InternalBookmarkListChanged(func) => transmute(func),
                HapCallback::InternalBreakIo(func) => transmute(func),
                HapCallback::InternalDeviceRegAccess(func) => transmute(func),
                HapCallback::InternalMicroCheckpointLoaded(func) => transmute(func),
                HapCallback::InternalSbWait(func) => transmute(func),
                HapCallback::InternalTimeDirectionChanged(func) => transmute(func),
                HapCallback::InternalTimeQuantumChanged(func) => transmute(func),
                HapCallback::RealtimeEnabled(func) => transmute(func),
                HapCallback::RecStateChanged(func) => transmute(func),
                HapCallback::RexecLimitExceeded(func) => transmute(func),
                HapCallback::RtcNvramUpdate(func) => transmute(func),
                HapCallback::ScsiDiskCommand(func) => transmute(func),
                HapCallback::SnNaptEnabled(func) => transmute(func),
                HapCallback::TextConsoleNewTitle(func) => transmute(func),
                HapCallback::TextConsoleShowHide(func) => transmute(func),
                HapCallback::TlbFillData(func) => transmute(func),
                HapCallback::TlbFillInstruction(func) => transmute(func),
                HapCallback::TlbInvalidateData(func) => transmute(func),
                HapCallback::TlbInvalidateInstruction(func) => transmute(func),
                HapCallback::TlbMissData(func) => transmute(func),
                HapCallback::TlbMissInstruction(func) => transmute(func),
                HapCallback::TlbReplaceData(func) => transmute(func),
                HapCallback::TlbReplaceInstruction(func) => transmute(func),
                HapCallback::UiRecordStateChanged(func) => transmute(func),
                HapCallback::UiRunStateChanged(func) => transmute(func),
                HapCallback::VgaBreakString(func) => transmute(func),
                HapCallback::VgaRefreshTriggered(func) => transmute(func),
                HapCallback::XtermBreakString(func) => transmute(func),
                HapCallback::CoreInterruptStatus(func) => transmute(func),
                HapCallback::CoreModeSwitch(func) => transmute(func),
                HapCallback::CorePseudoException(func) => transmute(func),
                HapCallback::X86DescriptorChange(func) => transmute(func),
                HapCallback::X86EnterSmm(func) => transmute(func),
                HapCallback::X86LeaveSmm(func) => transmute(func),
                HapCallback::X86MisplacedRex(func) => transmute(func),
                HapCallback::X86ProcessorReset(func) => transmute(func),
                HapCallback::X86Sysenter(func) => transmute(func),
                HapCallback::X86Sysexit(func) => transmute(func),
                HapCallback::X86TripleFault(func) => transmute(func),
                HapCallback::X86VmcsRead(func) => transmute(func),
                HapCallback::X86VmcsWrite(func) => transmute(func),
                HapCallback::X86VmxModeChange(func) => transmute(func),
                HapCallback::ArmInstructionModeChange(func) => transmute(func),
                HapCallback::ArmV8InterProcessing(func) => transmute(func),
            }
        }
    }

    /// Check if a HAP callback is the correct callback type for a HAP
    pub fn is_callback_for(&self, hap: &Hap) -> bool {
        match *self {
            HapCallback::Arinc429Word(_) => matches!(hap, Hap::Arinc429Word),
            HapCallback::CliCommandAdded(_) => matches!(hap, Hap::CliCommandAdded),
            HapCallback::ComponentChange(_) => matches!(hap, Hap::ComponentChange),
            HapCallback::ComponentHierarchyChange(_) => {
                matches!(hap, Hap::ComponentHierarchyChange)
            }
            HapCallback::ConsoleBreakString(_) => matches!(hap, Hap::ConsoleBreakString),
            HapCallback::CoreAddressNotMapped(_) => matches!(hap, Hap::CoreAddressNotMapped),
            HapCallback::CoreAsynchronousTrap(_) => matches!(hap, Hap::CoreAsynchronousTrap),
            HapCallback::CoreAtExit(_) => matches!(hap, Hap::CoreAtExit),
            HapCallback::CoreBackToFront(_) => matches!(hap, Hap::CoreBackToFront),
            HapCallback::CoreBreakpointChange(_) => matches!(hap, Hap::CoreBreakpointChange),
            HapCallback::CoreBreakpointMemop(_) => matches!(hap, Hap::CoreBreakpointMemop),
            HapCallback::CoreCleanAtExit(_) => matches!(hap, Hap::CoreCleanAtExit),
            HapCallback::CoreConfClassRegister(_) => matches!(hap, Hap::CoreConfClassRegister),
            HapCallback::CoreConfClassUnregister(_) => matches!(hap, Hap::CoreConfClassUnregister),
            HapCallback::CoreConfClockChangeCell(_) => matches!(hap, Hap::CoreConfClockChangeCell),
            HapCallback::CoreConfObjectChangeClock(_) => {
                matches!(hap, Hap::CoreConfObjectChangeClock)
            }
            HapCallback::CoreConfObjectCreate(_) => matches!(hap, Hap::CoreConfObjectCreate),
            HapCallback::CoreConfObjectCreated(_) => matches!(hap, Hap::CoreConfObjectCreated),
            HapCallback::CoreConfObjectDelete(_) => matches!(hap, Hap::CoreConfObjectDelete),
            HapCallback::CoreConfObjectPreDelete(_) => matches!(hap, Hap::CoreConfObjectPreDelete),
            HapCallback::CoreConfObjectRename(_) => matches!(hap, Hap::CoreConfObjectRename),
            HapCallback::CoreConfObjectsCreated(_) => matches!(hap, Hap::CoreConfObjectsCreated),
            HapCallback::CoreConfObjectsDeleted(_) => matches!(hap, Hap::CoreConfObjectsDeleted),
            HapCallback::CoreConfigurationLoaded(_) => matches!(hap, Hap::CoreConfigurationLoaded),
            HapCallback::CoreContextActivate(_) => matches!(hap, Hap::CoreContextActivate),
            HapCallback::CoreContextChange(_) => matches!(hap, Hap::CoreContextChange),
            HapCallback::CoreContextDeactivate(_) => matches!(hap, Hap::CoreContextDeactivate),
            HapCallback::CoreContextUpdated(_) => matches!(hap, Hap::CoreContextUpdated),
            HapCallback::CoreContinuation(_) => matches!(hap, Hap::CoreContinuation),
            HapCallback::CoreControlRegisterRead(_) => matches!(hap, Hap::CoreControlRegisterRead),
            HapCallback::CoreControlRegisterWrite(_) => {
                matches!(hap, Hap::CoreControlRegisterWrite)
            }
            HapCallback::CoreDeviceAccessMemop(_) => matches!(hap, Hap::CoreDeviceAccessMemop),
            HapCallback::CoreDisableBreakpoints(_) => matches!(hap, Hap::CoreDisableBreakpoints),
            HapCallback::CoreDiscardFuture(_) => matches!(hap, Hap::CoreDiscardFuture),
            HapCallback::CoreDstcFlushCounter(_) => matches!(hap, Hap::CoreDstcFlushCounter),
            HapCallback::CoreException(_) => matches!(hap, Hap::CoreException),
            HapCallback::CoreExceptionReturn(_) => matches!(hap, Hap::CoreExceptionReturn),
            HapCallback::CoreExternalInterrupt(_) => matches!(hap, Hap::CoreExternalInterrupt),
            HapCallback::CoreFrequencyChanged(_) => matches!(hap, Hap::CoreFrequencyChanged),
            HapCallback::CoreGlobalMessage(_) => matches!(hap, Hap::CoreGlobalMessage),
            HapCallback::CoreHapCallbackInstalled(_) => {
                matches!(hap, Hap::CoreHapCallbackInstalled)
            }
            HapCallback::CoreHapCallbackRemoved(_) => matches!(hap, Hap::CoreHapCallbackRemoved),
            HapCallback::CoreHapTypeAdded(_) => matches!(hap, Hap::CoreHapTypeAdded),
            HapCallback::CoreImageActivity(_) => matches!(hap, Hap::CoreImageActivity),
            HapCallback::CoreInitialConfiguration(_) => {
                matches!(hap, Hap::CoreInitialConfiguration)
            }
            HapCallback::CoreLogGroupsChange(_) => matches!(hap, Hap::CoreLogGroupsChange),
            HapCallback::CoreLogLevelChange(_) => matches!(hap, Hap::CoreLogLevelChange),
            HapCallback::CoreLogMessage(_) => matches!(hap, Hap::CoreLogMessage),
            HapCallback::CoreLogMessageExtended(_) => matches!(hap, Hap::CoreLogMessageExtended),
            HapCallback::CoreLogMessageFiltered(_) => matches!(hap, Hap::CoreLogMessageFiltered),
            HapCallback::CoreMagicInstruction(_) => matches!(hap, Hap::CoreMagicInstruction),
            HapCallback::CoreMemorySpaceMapChanged(_) => {
                matches!(hap, Hap::CoreMemorySpaceMapChanged)
            }
            HapCallback::CoreModeChange(_) => matches!(hap, Hap::CoreModeChange),
            HapCallback::CoreModuleLoaded(_) => matches!(hap, Hap::CoreModuleLoaded),
            HapCallback::CoreMulticoreAccelerationChanged(_) => {
                matches!(hap, Hap::CoreMulticoreAccelerationChanged)
            }
            HapCallback::CoreMultithreadingChanged(_) => {
                matches!(hap, Hap::CoreMultithreadingChanged)
            }
            HapCallback::CoreNotImplemented(_) => matches!(hap, Hap::CoreNotImplemented),
            HapCallback::CorePreferencesChanged(_) => matches!(hap, Hap::CorePreferencesChanged),
            HapCallback::CoreProcessorScheduleChanged(_) => {
                matches!(hap, Hap::CoreProcessorScheduleChanged)
            }
            HapCallback::CoreProjectChanged(_) => matches!(hap, Hap::CoreProjectChanged),
            HapCallback::CoreRecentFilesChanged(_) => matches!(hap, Hap::CoreRecentFilesChanged),
            HapCallback::CoreRexecActive(_) => matches!(hap, Hap::CoreRexecActive),
            HapCallback::CoreSimulationModeChange(_) => {
                matches!(hap, Hap::CoreSimulationModeChange)
            }
            HapCallback::CoreSimulationStopped(_) => matches!(hap, Hap::CoreSimulationStopped),
            HapCallback::CoreSkiptoProgress(_) => matches!(hap, Hap::CoreSkiptoProgress),
            HapCallback::CoreSyncInstruction(_) => matches!(hap, Hap::CoreSyncInstruction),
            HapCallback::CoreTimeTransition(_) => matches!(hap, Hap::CoreTimeTransition),
            HapCallback::CoreTimingModelChange(_) => matches!(hap, Hap::CoreTimingModelChange),
            HapCallback::CoreUserCommentsChanged(_) => matches!(hap, Hap::CoreUserCommentsChanged),
            HapCallback::CoreWriteConfiguration(_) => matches!(hap, Hap::CoreWriteConfiguration),
            HapCallback::EthInjectorPcapEof(_) => matches!(hap, Hap::EthInjectorPcapEof),
            HapCallback::FirewireReset(_) => matches!(hap, Hap::FirewireReset),
            HapCallback::FirewireTransfer(_) => matches!(hap, Hap::FirewireTransfer),
            HapCallback::GfxBreak(_) => matches!(hap, Hap::GfxBreak),
            HapCallback::GfxBreakString(_) => matches!(hap, Hap::GfxBreakString),
            HapCallback::GraphicsConsoleNewTitle(_) => matches!(hap, Hap::GraphicsConsoleNewTitle),
            HapCallback::GraphicsConsoleShowHide(_) => matches!(hap, Hap::GraphicsConsoleShowHide),
            HapCallback::InternalBookmarkListChanged(_) => {
                matches!(hap, Hap::InternalBookmarkListChanged)
            }
            HapCallback::InternalBreakIo(_) => matches!(hap, Hap::InternalBreakIo),
            HapCallback::InternalDeviceRegAccess(_) => matches!(hap, Hap::InternalDeviceRegAccess),
            HapCallback::InternalMicroCheckpointLoaded(_) => {
                matches!(hap, Hap::InternalMicroCheckpointLoaded)
            }
            HapCallback::InternalSbWait(_) => matches!(hap, Hap::InternalSbWait),
            HapCallback::InternalTimeDirectionChanged(_) => {
                matches!(hap, Hap::InternalTimeDirectionChanged)
            }
            HapCallback::InternalTimeQuantumChanged(_) => {
                matches!(hap, Hap::InternalTimeQuantumChanged)
            }
            HapCallback::RealtimeEnabled(_) => matches!(hap, Hap::RealtimeEnabled),
            HapCallback::RecStateChanged(_) => matches!(hap, Hap::RecStateChanged),
            HapCallback::RexecLimitExceeded(_) => matches!(hap, Hap::RexecLimitExceeded),
            HapCallback::RtcNvramUpdate(_) => matches!(hap, Hap::RtcNvramUpdate),
            HapCallback::ScsiDiskCommand(_) => matches!(hap, Hap::ScsiDiskCommand),
            HapCallback::SnNaptEnabled(_) => matches!(hap, Hap::SnNaptEnabled),
            HapCallback::TextConsoleNewTitle(_) => matches!(hap, Hap::TextConsoleNewTitle),
            HapCallback::TextConsoleShowHide(_) => matches!(hap, Hap::TextConsoleShowHide),
            HapCallback::TlbFillData(_) => matches!(hap, Hap::TlbFillData),
            HapCallback::TlbFillInstruction(_) => matches!(hap, Hap::TlbFillInstruction),
            HapCallback::TlbInvalidateData(_) => matches!(hap, Hap::TlbInvalidateData),
            HapCallback::TlbInvalidateInstruction(_) => {
                matches!(hap, Hap::TlbInvalidateInstruction)
            }
            HapCallback::TlbMissData(_) => matches!(hap, Hap::TlbMissData),
            HapCallback::TlbMissInstruction(_) => matches!(hap, Hap::TlbMissInstruction),
            HapCallback::TlbReplaceData(_) => matches!(hap, Hap::TlbReplaceData),
            HapCallback::TlbReplaceInstruction(_) => matches!(hap, Hap::TlbReplaceInstruction),
            HapCallback::UiRecordStateChanged(_) => matches!(hap, Hap::UiRecordStateChanged),
            HapCallback::UiRunStateChanged(_) => matches!(hap, Hap::UiRunStateChanged),
            HapCallback::VgaBreakString(_) => matches!(hap, Hap::VgaBreakString),
            HapCallback::VgaRefreshTriggered(_) => matches!(hap, Hap::VgaRefreshTriggered),
            HapCallback::XtermBreakString(_) => matches!(hap, Hap::XtermBreakString),
            HapCallback::CoreInterruptStatus(_) => matches!(hap, Hap::CoreInterruptStatus),
            HapCallback::CoreModeSwitch(_) => matches!(hap, Hap::CoreModeSwitch),
            HapCallback::CorePseudoException(_) => matches!(hap, Hap::CorePseudoException),
            HapCallback::X86DescriptorChange(_) => matches!(hap, Hap::X86DescriptorChange),
            HapCallback::X86EnterSmm(_) => matches!(hap, Hap::X86EnterSmm),
            HapCallback::X86LeaveSmm(_) => matches!(hap, Hap::X86LeaveSmm),
            HapCallback::X86MisplacedRex(_) => matches!(hap, Hap::X86MisplacedRex),
            HapCallback::X86ProcessorReset(_) => matches!(hap, Hap::X86ProcessorReset),
            HapCallback::X86Sysenter(_) => matches!(hap, Hap::X86Sysenter),
            HapCallback::X86Sysexit(_) => matches!(hap, Hap::X86Sysexit),
            HapCallback::X86TripleFault(_) => matches!(hap, Hap::X86TripleFault),
            HapCallback::X86VmcsRead(_) => matches!(hap, Hap::X86VmcsRead),
            HapCallback::X86VmcsWrite(_) => matches!(hap, Hap::X86VmcsWrite),
            HapCallback::X86VmxModeChange(_) => matches!(hap, Hap::X86VmxModeChange),
            HapCallback::ArmInstructionModeChange(_) => {
                matches!(hap, Hap::ArmInstructionModeChange)
            }
            HapCallback::ArmV8InterProcessing(_) => matches!(hap, Hap::ArmV8InterProcessing),
        }
    }
}

/// Add a callback on a particular HAP occurrence, with some user data (which should generally be
/// a raw pointer to the module object you are running)
pub fn hap_add_callback<D>(hap: Hap, func: HapCallback, data: Option<D>) -> Result<HapHandle>
where
    D: Into<*mut c_void>,
{
    ensure!(
        func.is_callback_for(&hap),
        "Callback and Hap types must match!"
    );

    let data = match data {
        Some(data) => data.into(),
        None => null_mut(),
    };

    let handle =
        unsafe { SIM_hap_add_callback(raw_cstr(hap.to_string())?, Some(func.as_fn()), data) };

    if handle == -1 {
        bail!(
            "Error adding {} callback: {}",
            hap.to_string(),
            last_error()
        );
    } else {
        Ok(handle)
    }
}
