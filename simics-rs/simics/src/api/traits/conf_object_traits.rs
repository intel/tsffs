// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Conversions to and from `ConfObject` pointers

use crate::ConfObject;

/// Convert from a reference to a raw pointer
pub trait AsConfObject {
    /// Convert a reference to this object to a raw `ConfObject` pointer
    fn as_conf_object(&self) -> *const ConfObject {
        self as *const _ as *const ConfObject
    }

    /// Convert a mutable reference to this object to a raw `ConfObject` pointer
    fn as_conf_object_mut(&mut self) -> *mut ConfObject {
        self as *mut _ as *mut ConfObject
    }
}

/// Convert from a raw pointer to a reference
pub trait FromConfObject
where
    Self: Sized,
{
    /// Get a reference to this object from a raw `ConfObject` pointer
    ///
    /// # Safety
    ///
    /// This function dereferences a raw pointer. It must be called with a valid pointer which
    /// has a sufficient lifetime.
    unsafe fn from_conf_object<'a>(obj: *const ConfObject) -> &'a Self {
        &*(obj as *const Self)
    }

    /// Get a mutable reference to this object from a raw `ConfObject` pointer
    ///
    /// # Safety
    ///
    /// This function dereferences a raw pointer. It must be called with a valid pointer which
    /// has a sufficient lifetime.
    unsafe fn from_conf_object_mut<'a>(obj: *mut ConfObject) -> &'a mut Self {
        &mut *(obj as *mut Self)
    }
}
