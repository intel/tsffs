// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Path handling

use crate::{
    simics_exception,
    sys::{SIM_add_directory, SIM_clear_directories, SIM_get_directories, SIM_lookup_file},
    AttrValue, Error, Result,
};
use raw_cstr::raw_cstr;
use std::{
    ffi::CStr,
    path::{Path, PathBuf},
};

#[simics_exception]
/// - If file exists and is an absolute path, it is converted to host native form and
/// returned.  
/// - If file starts with %simics%, the rest of the path is looked up first in the
/// current Simics project, and then in all configured Simics packages. If a match is
/// found, the native form of the file found will be returned.  
/// - If file exists in or relative to the current directory, it is returned without
/// using the Simics search path. This is more or less equivalent of always having "."
/// first in the search path.
/// - For each directory in Simics search path: The directory and the file is
/// concatenated and converted to host native format. Each such file is looked up first
/// in the current Simics project, and then in all Simics packages. If a match is found,
/// the native form of the file found will be returned.
///
/// # Examples
///
/// ```rust,ignore
/// use simics::api::lookup_file;
///
/// lookup_file("%simics%/target/Software.efi")?;
/// ```
pub fn lookup_file<S>(file: S) -> Result<PathBuf>
where
    S: AsRef<str>,
{
    let res = unsafe { SIM_lookup_file(raw_cstr(file.as_ref())?) };

    if res.is_null() {
        Err(Error::FileLookup {
            file: file.as_ref().to_string(),
        })
    } else {
        Ok(PathBuf::from(unsafe { CStr::from_ptr(res) }.to_str()?))
    }
}

#[simics_exception]
/// Add a directory to the SIMICS search path
pub fn add_directory<P>(directory: P, prepend: bool) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_add_directory(
            raw_cstr(directory.as_ref().to_str().ok_or(Error::ToString)?)?,
            prepend,
        );
    }

    Ok(())
}

#[simics_exception]
/// Clear extra search directories
pub fn clear_directories() {
    unsafe { SIM_clear_directories() };
}

#[simics_exception]
/// Get the list of extra search directories
pub fn get_directories() -> AttrValue {
    unsafe { SIM_get_directories() }.into()
}
