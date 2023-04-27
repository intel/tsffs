use crate::{last_error, ConfObject, GenericTransaction};
use anyhow::{bail, Result};
use raw_cstr::raw_cstr;
use simics_api_sys::{hap_handle_t, SIM_hap_add_callback};
use std::{
    ffi::{c_char, c_void},
    mem::transmute,
    ptr::null_mut,
};

pub type HapHandle = hap_handle_t;

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
    const HAP_ARINC429_WORD: &str = "Arinc429_Word";
    const HAP_CLI_COMMAND_ADDED: &str = "CLI_Command_Added";
    const HAP_COMPONENT_CHANGE: &str = "Component_Change";
    const HAP_COMPONENT_HIERARCHY_CHANGE: &str = "Component_Hierarchy_Change";
    const HAP_CONSOLE_BREAK_STRING: &str = "Console_Break_String";
    const HAP_CORE_ADDRESS_NOT_MAPPED: &str = "Core_Address_Not_Mapped";
    const HAP_CORE_ASYNCHRONOUS_TRAP: &str = "Core_Asynchronous_Trap";
    const HAP_CORE_AT_EXIT: &str = "Core_At_Exit";
    const HAP_CORE_BACK_TO_FRONT: &str = "Core_Back_To_Front";
    const HAP_CORE_BREAKPOINT_CHANGE: &str = "Core_Breakpoint_Change";
    const HAP_CORE_BREAKPOINT_MEMOP: &str = "Core_Breakpoint_Memop";
    const HAP_CORE_CLEAN_AT_EXIT: &str = "Core_Clean_At_Exit";
    const HAP_CORE_CONF_CLASS_REGISTER: &str = "Core_Conf_Class_Register";
    const HAP_CORE_CONF_CLASS_UNREGISTER: &str = "Core_Conf_Class_Unregister";
    const HAP_CORE_CONF_CLOCK_CHANGE_CELL: &str = "Core_Conf_Clock_Change_Cell";
    const HAP_CORE_CONF_OBJECT_CHANGE_CLOCK: &str = "Core_Conf_Object_Change_Clock";
    const HAP_CORE_CONF_OBJECT_CREATE: &str = "Core_Conf_Object_Create";
    const HAP_CORE_CONF_OBJECT_CREATED: &str = "Core_Conf_Object_Created";
    const HAP_CORE_CONF_OBJECT_DELETE: &str = "Core_Conf_Object_Delete";
    const HAP_CORE_CONF_OBJECT_PRE_DELETE: &str = "Core_Conf_Object_Pre_Delete";
    const HAP_CORE_CONF_OBJECT_RENAME: &str = "Core_Conf_Object_Rename";
    const HAP_CORE_CONF_OBJECTS_CREATED: &str = "Core_Conf_Objects_Created";
    const HAP_CORE_CONF_OBJECTS_DELETED: &str = "Core_Conf_Objects_Deleted";
    const HAP_CORE_CONFIGURATION_LOADED: &str = "Core_Configuration_Loaded";
    const HAP_CORE_CONTEXT_ACTIVATE: &str = "Core_Context_Activate";
    const HAP_CORE_CONTEXT_CHANGE: &str = "Core_Context_Change";
    const HAP_CORE_CONTEXT_DEACTIVATE: &str = "Core_Context_Deactivate";
    const HAP_CORE_CONTEXT_UPDATED: &str = "Core_Context_Updated";
    const HAP_CORE_CONTINUATION: &str = "Core_Continuation";
    const HAP_CORE_CONTROL_REGISTER_READ: &str = "Core_Control_Register_Read";
    const HAP_CORE_CONTROL_REGISTER_WRITE: &str = "Core_Control_Register_Write";
    const HAP_CORE_DEVICE_ACCESS_MEMOP: &str = "Core_Device_Access_Memop";
    const HAP_CORE_DISABLE_BREAKPOINTS: &str = "Core_Disable_Breakpoints";
    const HAP_CORE_DISCARD_FUTURE: &str = "Core_Discard_Future";
    const HAP_CORE_DSTC_FLUSH_COUNTER: &str = "Core_DSTC_Flush_Counter";
    const HAP_CORE_EXCEPTION: &str = "Core_Exception";
    const HAP_CORE_EXCEPTION_RETURN: &str = "Core_Exception_Return";
    const HAP_CORE_EXTERNAL_INTERRUPT: &str = "Core_External_Interrupt";
    const HAP_CORE_FREQUENCY_CHANGED: &str = "Core_Frequency_Changed";
    const HAP_CORE_GLOBAL_MESSAGE: &str = "Core_Global_Message";
    const HAP_CORE_HAP_CALLBACK_INSTALLED: &str = "Core_Hap_Callback_Installed";
    const HAP_CORE_HAP_CALLBACK_REMOVED: &str = "Core_Hap_Callback_Removed";
    const HAP_CORE_HAP_TYPE_ADDED: &str = "Core_Hap_Type_Added";
    const HAP_CORE_IMAGE_ACTIVITY: &str = "Core_Image_Activity";
    const HAP_CORE_INITIAL_CONFIGURATION: &str = "Core_Initial_Configuration";
    const HAP_CORE_LOG_GROUPS_CHANGE: &str = "Core_Log_Groups_Change";
    const HAP_CORE_LOG_LEVEL_CHANGE: &str = "Core_Log_Level_Change";
    const HAP_CORE_LOG_MESSAGE: &str = "Core_Log_Message";
    const HAP_CORE_LOG_MESSAGE_EXTENDED: &str = "Core_Log_Message_Extended";
    const HAP_CORE_LOG_MESSAGE_FILTERED: &str = "Core_Log_Message_Filtered";
    const HAP_CORE_MAGIC_INSTRUCTION: &str = "Core_Magic_Instruction";
    const HAP_CORE_MEMORY_SPACE_MAP_CHANGED: &str = "Core_Memory_Space_Map_Changed";
    const HAP_CORE_MODE_CHANGE: &str = "Core_Mode_Change";
    const HAP_CORE_MODULE_LOADED: &str = "Core_Module_Loaded";
    const HAP_CORE_MULTICORE_ACCELERATION_CHANGED: &str = "Core_Multicore_Acceleration_Changed";
    const HAP_CORE_MULTITHREADING_CHANGED: &str = "Core_Multithreading_Changed";
    const HAP_CORE_NOT_IMPLEMENTED: &str = "Core_Not_Implemented";
    const HAP_CORE_PREFERENCES_CHANGED: &str = "Core_Preferences_Changed";
    const HAP_CORE_PROCESSOR_SCHEDULE_CHANGED: &str = "Core_Processor_Schedule_Changed";
    const HAP_CORE_PROJECT_CHANGED: &str = "Core_Project_Changed";
    const HAP_CORE_RECENT_FILES_CHANGED: &str = "Core_Recent_Files_Changed";
    const HAP_CORE_REXEC_ACTIVE: &str = "Core_Rexec_Active";
    const HAP_CORE_SIMULATION_MODE_CHANGE: &str = "Core_Simulation_Mode_Change";
    const HAP_CORE_SIMULATION_STOPPED: &str = "Core_Simulation_Stopped";
    const HAP_CORE_SKIPTO_PROGRESS: &str = "Core_Skipto_Progress";
    const HAP_CORE_SYNC_INSTRUCTION: &str = "Core_Sync_Instruction";
    const HAP_CORE_TIME_TRANSITION: &str = "Core_Time_Transition";
    const HAP_CORE_TIMING_MODEL_CHANGE: &str = "Core_Timing_Model_Change";
    const HAP_CORE_USER_COMMENTS_CHANGED: &str = "Core_User_Comments_Changed";
    const HAP_CORE_WRITE_CONFIGURATION: &str = "Core_Write_Configuration";
    const HAP_ETH_INJECTOR_PCAP_EOF: &str = "Eth_Injector_Pcap_Eof";
    const HAP_FIREWIRE_RESET: &str = "Firewire_Reset";
    const HAP_FIREWIRE_TRANSFER: &str = "Firewire_Transfer";
    const HAP_GFX_BREAK: &str = "Gfx_Break";
    const HAP_GFX_BREAK_STRING: &str = "Gfx_Break_String";
    const HAP_GRAPHICS_CONSOLE_NEW_TITLE: &str = "Graphics_Console_New_Title";
    const HAP_GRAPHICS_CONSOLE_SHOW_HIDE: &str = "Graphics_Console_Show_Hide";
    const HAP_INTERNAL_BOOKMARK_LIST_CHANGED: &str = "Internal_Bookmark_List_Changed";
    const HAP_INTERNAL_BREAK_IO: &str = "Internal_Break_IO";
    const HAP_INTERNAL_DEVICE_REG_ACCESS: &str = "Internal_Device_Reg_Access";
    const HAP_INTERNAL_MICRO_CHECKPOINT_LOADED: &str = "Internal_Micro_Checkpoint_Loaded";
    const HAP_INTERNAL_SB_WAIT: &str = "Internal_SB_Wait";
    const HAP_INTERNAL_TIME_DIRECTION_CHANGED: &str = "Internal_Time_Direction_Changed";
    const HAP_INTERNAL_TIME_QUANTUM_CHANGED: &str = "Internal_Time_Quantum_Changed";
    const HAP_REALTIME_ENABLED: &str = "Realtime_Enabled";
    const HAP_REC_STATE_CHANGED: &str = "REC_State_Changed";
    const HAP_REXEC_LIMIT_EXCEEDED: &str = "Rexec_Limit_Exceeded";
    const HAP_RTC_NVRAM_UPDATE: &str = "RTC_Nvram_Update";
    const HAP_SCSI_DISK_COMMAND: &str = "SCSI_Disk_Command";
    const HAP_SN_NAPT_ENABLED: &str = "SN_NAPT_Enabled";
    const HAP_TEXT_CONSOLE_NEW_TITLE: &str = "Text_Console_New_Title";
    const HAP_TEXT_CONSOLE_SHOW_HIDE: &str = "Text_Console_Show_Hide";
    const HAP_TLB_FILL_DATA: &str = "TLB_Fill_Data";
    const HAP_TLB_FILL_INSTRUCTION: &str = "TLB_Fill_Instruction";
    const HAP_TLB_INVALIDATE_DATA: &str = "TLB_Invalidate_Data";
    const HAP_TLB_INVALIDATE_INSTRUCTION: &str = "TLB_Invalidate_Instruction";
    const HAP_TLB_MISS_DATA: &str = "TLB_Miss_Data";
    const HAP_TLB_MISS_INSTRUCTION: &str = "TLB_Miss_Instruction";
    const HAP_TLB_REPLACE_DATA: &str = "TLB_Replace_Data";
    const HAP_TLB_REPLACE_INSTRUCTION: &str = "TLB_Replace_Instruction";
    const HAP_UI_RECORD_STATE_CHANGED: &str = "UI_Record_State_Changed";
    const HAP_UI_RUN_STATE_CHANGED: &str = "UI_Run_State_Changed";
    const HAP_VGA_BREAK_STRING: &str = "Vga_Break_String";
    const HAP_VGA_REFRESH_TRIGGERED: &str = "Vga_Refresh_Triggered";
    const HAP_XTERM_BREAK_STRING: &str = "Xterm_Break_String";

    // X86 QSP HAPs
    const HAP_CORE_INTERRUPT_STATUS: &str = "Core_Interrupt_Status";
    const HAP_CORE_MODE_SWITCH: &str = "Core_Mode_Switch";
    const HAP_CORE_PSEUDO_EXCEPTION: &str = "Core_Pseudo_Exception";
    const HAP_X86_DESCRIPTOR_CHANGE: &str = "X86_Descriptor_Change";
    const HAP_X86_ENTER_SMM: &str = "X86_Enter_SMM";
    const HAP_X86_LEAVE_SMM: &str = "X86_Leave_SMM";
    const HAP_X86_MISPLACED_REX: &str = "X86_Misplaced_Rex";
    const HAP_X86_PROCESSOR_RESET: &str = "X86_Processor_Reset";
    const HAP_X86_SYSENTER: &str = "X86_Sysenter";
    const HAP_X86_SYSEXIT: &str = "X86_Sysexit";
    const HAP_X86_TRIPLE_FAULT: &str = "X86_Triple_Fault";
    const HAP_X86_VMCS_READ: &str = "X86_Vmcs_Read";
    const HAP_X86_VMCS_WRITE: &str = "X86_Vmcs_Write";
    const HAP_X86_VMX_MODE_CHANGE: &str = "X86_Vmx_Mode_Change";

    // ARM HAPs
    const HAP_ARM_INSTRUCTION_MODE_CHANGE: &str = "Arm_Instruction_Mode_Change";
    const HAP_ARM_V8_INTER_PROCESSING: &str = "Arm_V8_Inter_Processing";
}

impl ToString for Hap {
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
    unsafe extern "C" fn(*mut c_void, *const ConfObject, parameter: i64);
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
pub type CoreSimulationStoppedCallback =
    unsafe extern "C" fn(*mut c_void, *const ConfObject, exception: i64, error_string: *mut c_char);

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

pub struct ObjHapFunc {
    /// Function stored as integer
    func: usize,
}

impl ObjHapFunc {
    unsafe fn as_func(&self) -> extern "C" fn() {
        transmute(self.func)
    }
}

impl From<CoreExceptionCallback> for ObjHapFunc {
    fn from(value: CoreExceptionCallback) -> Self {
        Self {
            func: value as usize,
        }
    }
}

impl From<X86TripleFaultCallback> for ObjHapFunc {
    fn from(value: X86TripleFaultCallback) -> Self {
        Self {
            func: value as usize,
        }
    }
}

pub fn hap_add_callback<D>(hap: Hap, func: ObjHapFunc, data: Option<D>) -> Result<HapHandle>
where
    D: Into<*mut c_void>,
{
    let data = match data {
        Some(data) => data.into(),
        None => null_mut(),
    };

    let handle =
        unsafe { SIM_hap_add_callback(raw_cstr(hap.to_string())?, Some(func.as_func()), data) };

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
