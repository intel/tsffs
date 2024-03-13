// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Experimental snapshot APIs

#[cfg(all(
    simics_experimental_api_snapshots,
    not(simics_experimental_api_snapshots_v2),
    not(simics_stable_api_snapshots)
))]
// NOTE: This API changes to VT_take_snapshot in simics 6.0.180
use crate::sys::VT_save_snapshot;

#[cfg(all(simics_experimental_api_snapshots_v2, not(simics_stable_api_snapshots)))]
// NOTE: This API changed to VT_take_snapshot in simics 6.0.180 and to SIM_take_snapshot in simics 6.0.180
use crate::sys::{snapshot_error_t, VT_take_snapshot};

#[cfg(simics_stable_api_snapshots)]
use crate::sys::{
    SIM_delete_snapshot, SIM_list_snapshots, SIM_restore_snapshot, SIM_take_snapshot,
};

#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2
))]
use crate::sys::{VT_delete_snapshot, VT_list_snapshots, VT_restore_snapshot};

#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots,
))]
use crate::sys::{VT_snapshot_size_used, VT_snapshots_ignore_class};

#[cfg(simics_stable_api_snapshots)]
use crate::sys::snapshot_error_t;
use crate::{simics_exception, AttrValue, Result};
use raw_cstr::raw_cstr;

#[cfg(any(simics_experimental_api_snapshots_v2, simics_stable_api_snapshots))]
type SnapshotError = snapshot_error_t;

#[cfg(all(
    simics_experimental_api_snapshots,
    not(simics_experimental_api_snapshots_v2)
))]
#[simics_exception]
/// Save a snapshot with a name
pub fn save_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_save_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_experimental_api_snapshots_v2)]
#[cfg_attr(
    any(simics_experimental_api_snapshots, simics_stable_api_snapshots),
    deprecated = "Use `take_snapshot` instead"
)]
/// Save a snapshot with a name. API deprecated as of SIMICS 6.0.180
pub fn save_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_take_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_experimental_api_snapshots_v2)]
#[simics_exception]
/// Take a snapshot with a name
pub fn take_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_take_snapshot(raw_cstr(name)?) })
}

#[cfg(all(
    simics_experimental_api_snapshots,
    not(simics_experimental_api_snapshots_v2)
))]
#[simics_exception]
/// Restore a snapshot with a name
pub fn restore_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_restore_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_experimental_api_snapshots_v2)]
#[simics_exception]
/// Restore a snapshot with a name
pub fn restore_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_restore_snapshot(raw_cstr(name)?) })
}

#[cfg(all(
    simics_experimental_api_snapshots,
    not(simics_experimental_api_snapshots_v2)
))]
#[simics_exception]
/// Delete a snapshot with a name
pub fn delete_snapshot<S>(name: S) -> Result<bool>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_delete_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_experimental_api_snapshots_v2)]
#[simics_exception]
/// Delete a snapshot with a name
pub fn delete_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { VT_delete_snapshot(raw_cstr(name)?) })
}

#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
#[simics_exception]
/// Get the total size used by all snapshots
pub fn snapshot_size_used() -> AttrValue {
    unsafe { VT_snapshot_size_used() }.into()
}

#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2
))]
#[simics_exception]
/// Get the list of all snapshots
pub fn list_snapshots() -> AttrValue {
    unsafe { VT_list_snapshots() }.into()
}

#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
#[simics_exception]
/// Set snapshots to ignore a given class by name
pub fn snapshots_ignore_class<S>(class_name: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { VT_snapshots_ignore_class(raw_cstr(class_name)?) };
    Ok(())
}

#[deprecated = "Use `take_snapshot` instead`"]
#[cfg(simics_stable_api_snapshots)]
#[simics_exception]
/// Take a snapshot with a name
pub fn save_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_take_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_stable_api_snapshots)]
#[simics_exception]
/// Take a snapshot with a name
pub fn take_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_take_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_stable_api_snapshots)]
#[simics_exception]
/// Restore a snapshot with a name
pub fn restore_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_restore_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_stable_api_snapshots)]
#[simics_exception]
/// Delete a snapshot with a name
pub fn delete_snapshot<S>(name: S) -> Result<SnapshotError>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_delete_snapshot(raw_cstr(name)?) })
}

#[cfg(simics_stable_api_snapshots)]
#[simics_exception]
/// Get the list of all snapshots
pub fn list_snapshots() -> AttrValue {
    unsafe { SIM_list_snapshots() }.into()
}
