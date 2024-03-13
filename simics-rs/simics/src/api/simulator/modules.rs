// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Module handling

use crate::{
    simics_exception,
    sys::{
        SIM_add_module_dir, SIM_get_all_failed_modules, SIM_get_all_modules, SIM_load_module,
        SIM_module_list_refresh,
    },
    AttrValue, Result,
};
use raw_cstr::raw_cstr;
use std::path::Path;

#[simics_exception]
/// The list returned contains information about all modules that can be loaded into Simics. Each list entry is another list with module specific information. The layout of this sub-list is described below. The list may grow in future Simics version, but the currently defined fields will not change.
/// name - Module name (string).
/// path - File system path to the module (string).
/// loaded - Flag indicating that the module is already loaded (boolean).
/// version - Oldest Simics ABI version that the module was built for (integer).
/// user version - User version of the module (string).
/// build-id - Simics build-id that indicates in which Simics build this module was created (integer).
/// build-date - Build date of the module, in seconds (integer).
/// classes - Classes this module claims to implement.
/// thread-safe - If the module is thread-safe.
/// components - Components this module claims to implement.
/// user path - Module was loaded from path provided by user.
pub fn get_all_modules() -> AttrValue {
    unsafe { SIM_get_all_modules() }.into()
}

#[simics_exception]
/// Get the list of modules that failed to initialize
pub fn get_all_failed_modules() -> AttrValue {
    unsafe { SIM_get_all_failed_modules() }.into()
}

#[simics_exception]
/// Add a directory to the simulator to search for modules when running `load_module("name")`.
pub fn add_module_dir<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe { SIM_add_module_dir(raw_cstr(path.as_ref().to_string_lossy())?) }
    Ok(())
}

#[simics_exception]
/// Refresh the module list
pub fn module_list_refresh() {
    unsafe { SIM_module_list_refresh() };
}

#[simics_exception]
/// Load a module into the simulator
pub fn load_module<S>(module: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_load_module(raw_cstr(module)?) }
    Ok(())
}
