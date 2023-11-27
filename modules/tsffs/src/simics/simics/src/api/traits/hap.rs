// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use raw_cstr::AsRawCstr;

use crate::{
    api::{
        sys::{SIM_hap_delete_callback_id, SIM_hap_delete_callback_obj_id},
        ConfObject, HapHandle,
    },
    Result,
};

/// A SIMICS Hap and the type of callbacks associated with it
pub trait Hap {
    type Name: AsRawCstr;
    const NAME: Self::Name;

    fn delete_callback_id(handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_id(Self::NAME.as_raw_cstr()?, handle) };
        Ok(())
    }

    fn delete_callback_obj_id(obj: *mut ConfObject, handle: HapHandle) -> Result<()> {
        unsafe { SIM_hap_delete_callback_obj_id(Self::NAME.as_raw_cstr()?, obj, handle) };
        Ok(())
    }
}
