// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{
        attr_boolean, attr_integer, attr_is_list, attr_list_item, attr_list_size, attr_string,
        sys::{
            SIM_add_module_dir, SIM_get_all_failed_modules, SIM_get_all_modules, SIM_load_module,
            SIM_module_list_refresh,
        },
        AttrKind, AttrValue,
    },
    Error, Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::path::Path;

pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub loaded: bool,
    pub version: i32,
    pub user_version: String,
    pub build_id: i32,
    pub build_date: i32,
    // TODO: Unknown data type
    // classes: AttrValue?,
    pub thread_safe: bool,
    // TODO: Unknown data type
    // components: AttrValue?,
    pub user_path: bool,
}

impl TryFrom<AttrValue> for ModuleInfo {
    type Error = Error;
    fn try_from(value: AttrValue) -> Result<Self> {
        if attr_is_list(value) {
            Ok(Self {
                name: attr_string(attr_list_item(value, 0)?)?,
                path: attr_string(attr_list_item(value, 1)?)?,
                loaded: attr_boolean(attr_list_item(value, 2)?)?,
                version: attr_integer(attr_list_item(value, 3)?)?.try_into()?,
                user_version: attr_string(attr_list_item(value, 4)?)?,
                build_id: attr_integer(attr_list_item(value, 5)?)?.try_into()?,
                build_date: attr_integer(attr_list_item(value, 6)?)?.try_into()?,
                thread_safe: attr_boolean(attr_list_item(value, 8)?)?,
                user_path: attr_boolean(attr_list_item(value, 10)?)?,
            })
        } else {
            Err(Error::AttrValueType {
                actual: value.private_kind,
                expected: AttrKind::Sim_Val_List,
            })
        }
    }
}

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
pub fn get_all_modules() -> Result<Vec<ModuleInfo>> {
    let modules = unsafe { SIM_get_all_modules() };
    let mut module_infos = Vec::new();

    for i in 0..attr_list_size(modules)? {
        let module_info = attr_list_item(modules, i)?;
        module_infos.push(ModuleInfo::try_from(module_info)?);
    }

    Ok(module_infos)
}

#[simics_exception]
/// Get the list of modules that failed to initialize
pub fn get_all_failed_modules() -> AttrValue {
    unsafe { SIM_get_all_failed_modules() }
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
