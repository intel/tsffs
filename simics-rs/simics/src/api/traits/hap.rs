// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Traits for classes

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use raw_cstr::AsRawCstr;

use crate::{
    sys::{SIM_hap_delete_callback_id, SIM_hap_delete_callback_obj_id},
    ConfObject, HapHandle, Result,
};

/// A SIMICS Hap and the type of callbacks associated with it
pub trait Hap {
    /// The type of the name of the HAP, must be convertible to raw C string to pass to
    /// the simulator
    type Name: AsRawCstr;
    /// The name of the HAP.
    const NAME: Self::Name;

    /// A callback for a hap can be deleted by its handle
    fn delete_callback_id(handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_id(Self::NAME.as_raw_cstr()?, handle) };
        Ok(())
    }

    /// A callback for a hap can be deleted by the object it is associated with
    fn delete_callback_obj_id(obj: *mut ConfObject, handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_obj_id(Self::NAME.as_raw_cstr()?, obj, handle) };
        Ok(())
    }
}
