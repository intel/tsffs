// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Experimental snapshot APIs

use crate::{
    api::{
        sys::{
            VT_delete_snapshot, VT_list_snapshots, VT_restore_snapshot, VT_save_snapshot,
            VT_snapshot_size_used, VT_snapshots_ignore_class,
        },
        AttrValue,
    },
    Result,
};
use raw_cstr::raw_cstr;
use simics_macro::simics_exception;

#[simics_exception]
/// Save a snapshot with a name
pub fn save_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_save_snapshot(raw_cstr(name)?) })
}

#[simics_exception]
/// Restore a snapshot with a name
pub fn restore_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_restore_snapshot(raw_cstr(name)?) })
}

#[simics_exception]
/// Delete a snapshot with a name
pub fn delete_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_delete_snapshot(raw_cstr(name)?) })
}

#[simics_exception]
/// Get the total size used by all snapshots
pub fn snapshot_size_used() -> AttrValue {
    unsafe { VT_snapshot_size_used() }.into()
}

#[simics_exception]
/// Get the list of all snapshots
pub fn list_snapshots() -> AttrValue {
    unsafe { VT_list_snapshots() }.into()
}

#[simics_exception]
/// Set snapshots to ignore a given class by name
pub fn snapshots_ignore_class<S>(class_name: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { VT_snapshots_ignore_class(raw_cstr(class_name)?) };
    Ok(())
}
