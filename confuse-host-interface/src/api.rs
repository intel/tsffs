use libc::{c_char, c_int, c_void, pid_t, size_t};
use std::ptr::null_mut;

pub type SimicsHandle = pid_t;

#[no_mangle]
pub extern "C" fn confuse_init(
    /// Path to the root of the simics project
    simics_project_path: *const c_char,
    /// Path to simics app YAML file
    simics_project_config_path: *const c_char,
    /// OUT: Pointer to PID to set to the PID of running SIMICS instance
    simics_pid: *mut SimicsHandle,
) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn confuse_reset(simics_pid: SimicsHandle) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn confuse_run(simics_pid: SimicsHandle) -> c_int {
    0
}

#[no_mangle]
/// Create Direct I/O shared memory for SIMICS to communicate the branch trace map back
/// to the fuzzer
pub extern "C" fn confuse_create_dio_shared_mem(size: size_t) -> *mut c_void {}
