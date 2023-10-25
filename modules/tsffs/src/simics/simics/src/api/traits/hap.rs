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
    type Name: AsRawCstr;
    const NAME: Self::Name;

    // fn add_callback<F>(_callback: F) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
    // }

    // /// Callback that only reacts to HAPs for one index. Only for HAPs that have an associated
    // /// index.
    // fn add_callback_index<F>(_callback: F, _index: i64) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
    // }

    // /// Callback that only reacts to HAPs for a range of indices. Only for HAPs that have an associated
    // /// index.
    // fn add_callback_range<F>(_callback: F, _start: i64, _end: i64) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
    // }

    // /// Callback that only reacts to HAPs of a type triggered by `obj` and no other objects
    // fn add_callback_object<F>(_callback: F, _obj: *mut ConfObject) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
    // }

    // fn add_callback_object_index<F>(
    //     _callback: F,
    //     _obj: *mut ConfObject,
    //     _index: i64,
    // ) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
    // }

    // fn add_callback_object_range<F>(
    //     _callback: F,
    //     _obj: *mut ConfObject,
    //     _start: i64,
    //     _end: i64,
    // ) -> Result<HapHandle> {
    //     Err(Error::HapRegistrationType)
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
