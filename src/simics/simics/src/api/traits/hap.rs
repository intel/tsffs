// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use raw_cstr::AsRawCstr;

use crate::{
    api::{
        sys::{SIM_hap_delete_callback_id, SIM_hap_delete_callback_obj_id},
        ConfObject, HapHandle,
    },
    Error, Result,
};

/// A SIMICS Hap and the type of callbacks associated with it
pub trait Hap {
    type Handler;
    type Name: AsRawCstr;
    type Callback;
    const NAME: Self::Name;
    const HANDLER: Self::Handler;

    fn add_callback(_callback: Self::Callback) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    /// Callback that only reacts to HAPs for one index. Only for HAPs that have an associated
    /// index.
    fn add_callback_index(_callback: Self::Callback, _index: i64) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    /// Callback that only reacts to HAPs for a range of indices. Only for HAPs that have an associated
    /// index.
    fn add_callback_range(_callback: Self::Callback, _start: i64, _end: i64) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    /// Callback that only reacts to HAPs of a type triggered by `obj` and no other objects
    fn add_callback_object(_callback: Self::Callback, _obj: *mut ConfObject) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    fn add_callback_object_index(
        _callback: Self::Callback,
        _obj: *mut ConfObject,
        _index: i64,
    ) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    fn add_callback_object_range(
        _callback: Self::Callback,
        _obj: *mut ConfObject,
        _start: i64,
        _end: i64,
    ) -> Result<HapHandle> {
        Err(Error::HapRegistrationType)
    }

    // TODO: I don't think these are a good idea from callback pointers...let's just support
    // handles

    // fn delete_callback(_callback: Self::Callback) -> Result<()> {
    //     Err(Error::HapDeleteType)
    // }

    // fn delete_callback_obj(_callback: Self::Callback, _obj: *mut ConfObject) -> Result<()> {
    //     Err(Error::HapDeleteType)
    // }

    fn delete_callback_id(handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_id(Self::NAME.as_raw_cstr()?, handle) };
        Ok(())
    }

    fn delete_callback_obj_id(obj: *mut ConfObject, handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_obj_id(Self::NAME.as_raw_cstr()?, obj, handle) };
        Ok(())
    }
}
