// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{
        last_error,
        sys::{
            pre_conf_object_set_t, save_flags_t, SIM_add_configuration, SIM_current_checkpoint_dir,
            SIM_read_configuration, SIM_set_configuration, SIM_write_configuration_to_file,
        },
        AttrValue,
    },
    Error, Result,
};
use anyhow::anyhow;
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;
use std::{
    ffi::CStr,
    path::{Path, PathBuf},
};

pub type PreConfObjectSet = pre_conf_object_set_t;
pub type SaveFlags = save_flags_t;

#[simics_exception]
pub fn read_configuration<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_read_configuration(raw_cstr(file.as_ref().to_str().ok_or_else(|| {
            anyhow!(
                "Could not convert file path {} to string",
                file.as_ref().display()
            )
        })?)?)
    };
    Ok(())
}

#[simics_exception]
pub fn set_configuration(conf: AttrValue) {
    unsafe { SIM_set_configuration(conf) }
}

#[simics_exception]
pub fn add_configuration<P>(object_list: *mut PreConfObjectSet, file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_add_configuration(
            object_list,
            raw_cstr(file.as_ref().to_str().ok_or_else(|| {
                anyhow!(
                    "Could not convert file path {} to string",
                    file.as_ref().display()
                )
            })?)?,
        )
    };
    Ok(())
}

#[simics_exception]
/// Get the current checkpoint (bundle) directory if called during loading of a checkpoint.
/// May be absolute or relative.
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
pub fn write_configuration_to_file<P>(file: P, flags: SaveFlags) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_write_configuration_to_file(
            raw_cstr(file.as_ref().to_str().ok_or_else(|| {
                anyhow!(
                    "Could not convert file path {} to string",
                    file.as_ref().display()
                )
            })?)?,
            flags,
        )
    };
    Ok(())
}
