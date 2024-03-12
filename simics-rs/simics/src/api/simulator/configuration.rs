// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Configuration APIs

use crate::{
    last_error, simics_exception,
    sys::{
        pre_conf_object_set_t, save_flags_t, SIM_add_configuration, SIM_current_checkpoint_dir,
        SIM_read_configuration, SIM_set_configuration, SIM_write_configuration_to_file,
    },
    AttrValue, Error, Result,
};
use raw_cstr::raw_cstr;
use std::{
    ffi::CStr,
    path::{Path, PathBuf},
};

/// Alias for `pre_conf_object_set_t`
pub type PreConfObjectSet = pre_conf_object_set_t;
/// Alias for `save_flags_t`
pub type SaveFlags = save_flags_t;

#[simics_exception]
/// Read a configuration file into the simulator state
///
/// # Context
///
/// Global Context
pub fn read_configuration<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe { SIM_read_configuration(raw_cstr(file.as_ref().to_str().ok_or(Error::ToString)?)?) };
    Ok(())
}

#[simics_exception]
/// Set the current configuration from a value
/// Note: It is recommended that the add_configuration function is used instead of
/// set_configuration.  This function is an alternative to reading the configuration
/// from a file. A configuration is an attr_value_t which should have the following
/// structure.
///
/// `(("name", "class",  ("attr_name", attr_val) ... ), ... )`
///   
/// That is a list of object specifiers containing name, class, and a list of attribute
/// specifiers. An attribute specifier is a list of length 2 containing the attribute
/// name and its value. set_configuration allows an easy way of parameterizing the
/// configuration, especially if called from Python.
///
/// The argument value may be modified, but the caller is still responsible for freeing
/// it. Neither point applies when the function is called from Python.
///
/// EXAMPLE
///
/// ```python,ignore
/// The following is a Python example:
///
///   from configuration import OBJ
///   from simics import SIM_set_configuration
///
///   SIM_set_configuration([
///    ["cpu0", "x86",
///     ["queue", OBJ("cpu0")],
///     ["freq_mhz", 20],
///     ["physical_memory", OBJ("phys_mem0")]],
///
///    ["phys_mem0", "memory-space",
///     ["map",  [[0xa0000,    OBJ("vga0"),    1, 0, 0x20000],
///               [0x00000,    OBJ("mem0"),    0, 0x00000, 0xA0000],
///               [0xc0000,    OBJ("mem0"),    0, 0xc0000, 0x8000],
///               [0xc8000,    OBJ("setmem0"), 0, 0, 0x28000],
///               [0xf0000,    OBJ("mem0"),    0, 0xf0000, 0x10000],
///               [0x100000,   OBJ("mem0"),    0, 0x100000, 0x3ff00000],
///               [0xfee00000, OBJ("apic0"),   0, 0, 0x4000]]]],
///       ... ])
/// ```
///
/// # Context
///
/// Global Context
pub fn set_configuration(conf: AttrValue) {
    unsafe { SIM_set_configuration(conf.into()) }
}

#[simics_exception]
/// Add a configuration
///
/// This function creates objects from the parse objects in set and adds the initialized
/// objects to the current configuration (creating one if necessary). When called from
/// Python (which is the intended usage), the configuration set is a sequence (list or
/// tuple) of pre_conf_object Python objects, or a dictionary of the form {name :
/// pre_conf_object}.
///
/// The file argument is the name of the file that a configuration was read from, and
/// should be set to None/NULL if not used.
///
/// The following examples are written in Python. As they do not map any devices in
/// phys_mem, they will not work as stand-alone simulations.
///
/// Example when set is a sequence:
///
/// ```python,ignore
///     clock = pre_conf_object('timer', 'clock')
///     clock.freq_mhz = 20
///     space = pre_conf_object('phys_mem', 'memory-space')
///     space.queue = clock
///
///     SIM_add_configuration([clock, space], None)
/// ```
/// Example when set is a dictionary:
/// ```python,ignore
///     objects = {}
///     objects['clock'] = pre_conf_object('timer', 'clock')
///     objects['clock'].freq_mhz = 20
///     objects['space'] = pre_conf_object('phys_mem', 'memory-space')
///     objects['space'].queue = objects['clock']
///
///     SIM_add_configuration(objects, None)
/// ```
///
/// # Context
///
/// Global Context
pub fn add_configuration<P>(object_list: *mut PreConfObjectSet, file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_add_configuration(
            object_list,
            raw_cstr(file.as_ref().to_str().ok_or(Error::ToString)?)?,
        )
    };
    Ok(())
}

#[simics_exception]
/// Get the current checkpoint (bundle) directory if called during loading of a checkpoint.
/// May be absolute or relative.
///
/// # Context
///
/// Global Context
pub fn current_checkpoint_dir() -> Result<PathBuf> {
    let res = unsafe { SIM_current_checkpoint_dir() };

    if res.is_null() {
        Err(Error::CurrentCheckpointDir {
            message: last_error(),
        })
    } else {
        let mut dir = unsafe { CStr::from_ptr(res) }.to_str()?;

        if dir.is_empty() {
            dir = ".";
        }

        Ok(PathBuf::from(dir))
    }
}

#[simics_exception]
/// Save the current configuration to a file
///
/// Saves all objects to filename. Objects whose class_kind_t is equal to
/// Sim_Class_Kind_Session or Sim_Class_Kind_Pseudo are not saved. This also holds for
/// attributes (in all objects) of types Sim_Attr_Session and Sim_Attr_Pseudo.
///
/// The flags argument should be 0.
///
/// # Context
///
/// Global Context
pub fn write_configuration_to_file<P>(file: P, flags: SaveFlags) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_write_configuration_to_file(
            raw_cstr(file.as_ref().to_str().ok_or(Error::ToString)?)?,
            flags,
        )
    };
    Ok(())
}
