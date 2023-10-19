// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Safe wrappers for attr_value_t operations
//!
//! `attr_value_t` instances are basically Python objects as tagged unions (like an `enum`), these
//! functions convert the objects back and forth between anonymous `attr_value_t` and actual data
//! types like `bool`, `String`, etc.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    api::{
        sys::{
            attr_kind_t, attr_value__bindgen_ty_1, attr_value_t, SIM_alloc_attr_dict,
            SIM_alloc_attr_list, SIM_attr_dict_resize, SIM_attr_dict_set_item,
            SIM_attr_list_resize, SIM_attr_list_set_item, SIM_free_attribute,
        },
        ConfObject,
    },
    Error, Result,
};
use ordered_float::OrderedFloat;
use simics_macro::simics_exception;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ffi::{CStr, CString},
    hash::Hash,
    mem::size_of,
    ptr::null_mut,
};

pub type AttrKind = attr_kind_t;

#[derive(Clone)]
pub struct AttrValue(attr_value_t);

impl AttrValue {
    pub fn nil() -> Self {
        Self(attr_value_t {
            private_kind: AttrKind::Sim_Val_Nil,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 { integer: 0 },
        })
    }

    pub fn invalid() -> Self {
        Self(attr_value_t {
            private_kind: AttrKind::Sim_Val_Invalid,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 { integer: 0 },
        })
    }

    pub fn list(length: usize) -> Result<Self> {
        alloc_attr_list(length.try_into()?)
    }

    pub fn dict(size: usize) -> Result<Self> {
        alloc_attr_dict(size.try_into()?)
    }
}

impl AttrValue {
    /// Get a constant pointer to the inner attr value
    pub fn as_ptr(&self) -> *const attr_value_t {
        &self.0 as *const attr_value_t
    }

    /// Get a mutable pointer to the inner attr value
    pub fn as_mut_ptr(&mut self) -> *mut attr_value_t {
        &mut self.0 as *mut attr_value_t
    }

    /// Get the kind of the attr value
    pub fn kind(&self) -> AttrKind {
        self.0.private_kind
    }

    /// Get the size of the attr value. This is only non-zero in cases where the
    /// value is a collection (list, dict) or data type
    pub fn size(&self) -> u32 {
        self.0.private_size
    }

    /// Get whether the value is invalid type
    pub fn is_invalid(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Invalid
    }

    /// Get whether the value is nil type
    pub fn is_nil(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Nil
    }

    /// Get whether the value is integer type
    pub fn is_integer(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Integer
    }

    /// Get whether the value is unsigned integer type
    pub fn is_unsigned(&self) -> bool {
        self.is_integer() && self.size() == 0
    }

    /// Get whether the value is unsigned integer type
    pub fn is_signed(&self) -> bool {
        self.is_integer() && self.size() == 1
    }

    /// Get whether the value is boolean type
    pub fn is_boolean(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Boolean
    }

    /// Get whether the value is string type
    pub fn is_string(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_String
    }

    /// Get whether the value is floating type
    pub fn is_floating(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Floating
    }

    /// Get whether the value is object type
    pub fn is_object(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Object
    }

    /// Get whether the value is data type
    pub fn is_data(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Data
    }

    /// Get whether the value is list type
    pub fn is_list(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_List
    }

    /// Get whether the value is dict type
    pub fn is_dict(&self) -> bool {
        self.kind() == AttrKind::Sim_Val_Dict
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_integer(&self) -> Option<i64> {
        self.is_integer()
            .then_some(unsafe { self.0.private_u.integer })
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_unsigned(&self) -> Option<u64> {
        self.is_unsigned().then_some(u64::from_le_bytes(unsafe {
            self.0.private_u.integer.to_le_bytes()
        }))
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_signed(&self) -> Option<i64> {
        self.is_signed()
            .then_some(unsafe { self.0.private_u.integer })
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_boolean(&self) -> Option<bool> {
        self.is_boolean()
            .then_some(unsafe { self.0.private_u.boolean })
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_string(&self) -> Option<String> {
        self.is_boolean()
            .then(|| {
                unsafe { CStr::from_ptr(self.0.private_u.string) }
                    .to_str()
                    .ok()
                    .map(|s| s.to_string())
            })
            .flatten()
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_floating(&self) -> Option<f64> {
        self.is_floating()
            .then_some(unsafe { self.0.private_u.floating })
    }

    /// Get the value as an integer, if it is one, or `None` otherwise.
    pub fn as_object(&self) -> Option<*mut ConfObject> {
        self.is_object()
            .then_some(unsafe { self.0.private_u.object })
    }

    /// Get the value as an integer, if it is one, or `None` otherwise. Data is copied, the
    /// `AttrValue` maintains ownership.
    pub fn as_data<T>(&self) -> Option<T>
    where
        T: Clone,
    {
        if self.is_data() {
            // NOTE: This is leaked because the semantics of data ownership are that
            // returned data is owned by the attr and ownership is *not* returned to the
            // caller. It must be freed elsewhere.
            let data = Box::leak(unsafe { Box::from_raw(self.0.private_u.data as *mut T) });

            Some(data.clone())
        } else {
            None
        }
    }

    /// Get the value as a list, if it is one, or `None` otherwise. Data is copied, the
    /// `AttrValue` maintains ownership.
    pub fn as_list<T>(&self) -> Result<Option<Vec<T>>>
    where
        T: TryFrom<AttrValue> + Clone,
    {
        Ok(if self.is_list() {
            let size = self.size() as isize;

            // Rust vectors cannot be heterogeneous

            let items = (0..size)
                // NOTE: These are leaked because the semantics of data ownership are that
                // returned data is owned by the attr and ownership is *not* returned to the
                // caller. It must be freed elsewhere.
                .map(|i| Box::leak(unsafe { Box::from_raw(self.0.private_u.list.offset(i)) }))
                .collect::<Vec<_>>();

            if items
                .iter()
                .all(|i| Some(i.private_kind) == items.first().map(|f| f.private_kind))
            {
                Some(
                    items
                        .into_iter()
                        .map(|i| {
                            AttrValue(*i)
                                .try_into()
                                .map_err(|_| Error::AttrValueConversionError)
                        })
                        .collect::<Result<Vec<_>>>()?,
                )
            } else {
                None
            }
        } else {
            None
        })
    }

    /// Get the value as a dict, if it is one, or `None` otherwise. Data is copied, the
    /// `AttrValue` maintains ownership.
    pub fn as_dict<T, U>(&self) -> Result<Option<BTreeMap<T, U>>>
    where
        T: TryFrom<AttrValue> + Ord,
        U: TryFrom<AttrValue>,
    {
        Ok(if self.is_dict() {
            let size = self.size() as isize;

            let items = (0..size)
                .map(|i| {
                    // NOTE: These are leaked because the semantics of data ownership are that
                    // returned data is owned by the attr and ownership is *not* returned to the
                    // caller. It must be freed elsewhere.
                    (
                        Box::leak(unsafe { Box::from_raw(self.0.private_u.dict.offset(i)) }).key,
                        Box::leak(unsafe { Box::from_raw(self.0.private_u.dict.offset(i)) }).value,
                    )
                })
                .collect::<Vec<_>>();

            if items.iter().all(|(k, v)| {
                Some(k.private_kind) == items.first().map(|f| f.0.private_kind)
                    && Some(v.private_kind) == items.first().map(|f| f.1.private_kind)
            }) {
                Some(
                    items
                        .into_iter()
                        .map(|(k, v)| {
                            AttrValue(k)
                                .try_into()
                                .map_err(|_| Error::AttrValueConversionError)
                                .and_then(|k| {
                                    AttrValue(v)
                                        .try_into()
                                        .map_err(|_| Error::AttrValueConversionError)
                                        .map(|v| (k, v))
                                })
                        })
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .collect::<BTreeMap<_, _>>(),
                )
            } else {
                None
            }
        } else {
            None
        })
    }
}

impl From<attr_value_t> for AttrValue {
    fn from(value: attr_value_t) -> Self {
        AttrValue(value)
    }
}

impl From<AttrValue> for attr_value_t {
    fn from(value: AttrValue) -> Self {
        value.0
    }
}

impl From<i64> for AttrValue {
    fn from(value: i64) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_Integer,
            private_size: 1,
            private_u: attr_value__bindgen_ty_1 { integer: value },
        })
    }
}

impl From<u64> for AttrValue {
    fn from(value: u64) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_Integer,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 {
                integer: i64::from_le_bytes(value.to_le_bytes()),
            },
        })
    }
}

impl From<f64> for AttrValue {
    fn from(value: f64) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_Floating,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 { floating: value },
        })
    }
}

macro_rules! impl_from_signed {
    ($t:ty) => {
        impl From<$t> for AttrValue {
            fn from(value: $t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(value as i64)
            }
        }

        impl From<&$t> for AttrValue {
            fn from(value: &$t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(*value as i64)
            }
        }
    };
}

macro_rules! impl_from_unsigned {
    ($t:ty) => {
        impl From<$t> for AttrValue {
            fn from(value: $t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(value as u64)
            }
        }

        impl From<&$t> for AttrValue {
            fn from(value: &$t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(*value as u64)
            }
        }
    };
}

macro_rules! impl_from_float {
    ($t:ty) => {
        impl From<$t> for AttrValue {
            fn from(value: $t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(value as f64)
            }
        }

        impl From<&$t> for AttrValue {
            fn from(value: &$t) -> AttrValue {
                #[allow(clippy::unnecessary_cast)]
                AttrValue::from(*value as f64)
            }
        }
    };
}

impl_from_unsigned! { u8 }
impl_from_unsigned! { u16 }
impl_from_unsigned! { u32 }
impl_from_unsigned! { usize }
impl_from_signed! { i8 }
impl_from_signed! { i16 }
impl_from_signed! { i32 }
impl_from_signed! { isize }
impl_from_float! { f32 }

impl From<OrderedFloat<f32>> for AttrValue {
    fn from(value: OrderedFloat<f32>) -> Self {
        AttrValue::from(value.0 as f64)
    }
}

impl From<&OrderedFloat<f32>> for AttrValue {
    fn from(value: &OrderedFloat<f32>) -> Self {
        AttrValue::from(value.0 as f64)
    }
}

impl From<OrderedFloat<f64>> for AttrValue {
    fn from(value: OrderedFloat<f64>) -> Self {
        AttrValue::from(value.0)
    }
}

impl From<&OrderedFloat<f64>> for AttrValue {
    fn from(value: &OrderedFloat<f64>) -> Self {
        AttrValue::from(value.0)
    }
}

impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_String,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 {
                string: CString::new(value)
                    .expect("Failed to allocate memory for string")
                    .into_raw(),
            },
        })
    }
}

impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_String,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 {
                string: CString::new(value)
                    .expect("Failed to allocate memory for string")
                    .into_raw(),
            },
        })
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_Boolean,
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 { boolean: value },
        })
    }
}

impl From<*mut ConfObject> for AttrValue {
    fn from(value: *mut ConfObject) -> Self {
        AttrValue(attr_value_t {
            private_kind: if value.is_null() {
                AttrKind::Sim_Val_Nil
            } else {
                AttrKind::Sim_Val_Object
            },
            private_size: 0,
            private_u: attr_value__bindgen_ty_1 { object: value },
        })
    }
}

// Conversions from types whose conversion to AttrValue is always fallible. These are mostly
// collections. These implementations are split into two parts: collections whose elements are
// infallibly convertible to Attrvalue, and those whose elements are only fallibly convertible
// to AttrValue. For example, if a collection contains collections.
//
// NOTE: From<T> for U implies TryFrom<T> for U, so this is just a more general way to allow
// anything convertible.

impl<T> TryFrom<Option<T>> for AttrValue
where
    AttrValue: TryFrom<T>,
{
    type Error = Error;

    fn try_from(value: Option<T>) -> Result<Self> {
        if let Some(value) = value {
            AttrValue::try_from(value).map_err(|_| Error::AttrValueConversionError)
        } else {
            Ok(AttrValue::nil())
        }
    }
}

impl<T> TryFrom<&[T]> for AttrValue
where
    T: TryInto<AttrValue> + Clone,
{
    type Error = Error;

    fn try_from(value: &[T]) -> Result<Self> {
        let mut list = AttrValue::list(value.len())?;
        value.iter().enumerate().try_for_each(|(i, a)| {
            a.clone()
                .try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|a| attr_list_set_item(&mut list, i as u32, a))
        })?;
        Ok(list)
    }
}

impl<T> TryFrom<Vec<T>> for AttrValue
where
    T: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: Vec<T>) -> Result<Self> {
        let mut list = AttrValue::list(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, a)| {
            a.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|a| attr_list_set_item(&mut list, i as u32, a))
        })?;
        Ok(list)
    }
}

impl<T> TryFrom<HashSet<T>> for AttrValue
where
    T: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: HashSet<T>) -> Result<Self> {
        let mut list = AttrValue::list(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, a)| {
            a.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|a| attr_list_set_item(&mut list, i as u32, a))
        })?;

        Ok(list)
    }
}

impl<T> TryFrom<BTreeSet<T>> for AttrValue
where
    T: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: BTreeSet<T>) -> Result<Self> {
        let mut list = AttrValue::list(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, a)| {
            a.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|a| attr_list_set_item(&mut list, i as u32, a))
        })?;
        Ok(list)
    }
}

impl<T, U> TryFrom<HashMap<T, U>> for AttrValue
where
    T: TryInto<AttrValue>,
    U: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: HashMap<T, U>) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, (k, v))| {
            k.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

impl<T, U> TryFrom<BTreeMap<T, U>> for AttrValue
where
    T: TryInto<AttrValue>,
    U: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: BTreeMap<T, U>) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, (k, v))| {
            k.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

impl<T, U> TryFrom<&[(T, U)]> for AttrValue
where
    T: TryInto<AttrValue> + Clone,
    U: TryInto<AttrValue> + Clone,
{
    type Error = Error;

    fn try_from(value: &[(T, U)]) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.iter().enumerate().try_for_each(|(i, (k, v))| {
            k.clone()
                .try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.clone()
                        .try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

impl<T, U> TryFrom<Vec<(T, U)>> for AttrValue
where
    T: TryInto<AttrValue>,
    U: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: Vec<(T, U)>) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, (k, v))| {
            k.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

impl<T, U> TryFrom<HashSet<(T, U)>> for AttrValue
where
    T: TryInto<AttrValue>,
    U: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: HashSet<(T, U)>) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, (k, v))| {
            k.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

impl<T, U> TryFrom<BTreeSet<(T, U)>> for AttrValue
where
    T: TryInto<AttrValue>,
    U: TryInto<AttrValue>,
{
    type Error = Error;

    fn try_from(value: BTreeSet<(T, U)>) -> Result<Self> {
        let mut dict = AttrValue::dict(value.len())?;
        value.into_iter().enumerate().try_for_each(|(i, (k, v))| {
            k.try_into()
                .map_err(|_| Error::AttrValueConversionError)
                .and_then(|k| {
                    v.try_into()
                        .map_err(|_| Error::AttrValueConversionError)
                        .map(|v| (k, v))
                })
                .and_then(|(k, v)| attr_dict_set_item(&mut dict, i as u32, k, v))
        })?;
        Ok(dict)
    }
}

// Conversions *from* AttrValue to other types. These are all fallible because the AttrValue may
// not be the type an attempt is being made to convert it to.

macro_rules! impl_try_into_unsigned {
    ($t:ty) => {
        impl TryFrom<AttrValue> for $t {
            type Error = Error;

            fn try_from(value: AttrValue) -> Result<Self> {
                value
                    .as_unsigned()
                    .ok_or_else(|| Error::AttrValueType {
                        actual: value.kind(),
                        expected: AttrKind::Sim_Val_Integer,
                    })
                    .and_then(|i| i.try_into().map_err(|_| Error::AttrValueConversionError))
            }
        }
    };
}

macro_rules! impl_try_into_signed {
    ($t:ty) => {
        impl TryFrom<AttrValue> for $t {
            type Error = Error;

            fn try_from(value: AttrValue) -> Result<Self> {
                value
                    .as_signed()
                    .ok_or_else(|| Error::AttrValueType {
                        actual: value.kind(),
                        expected: AttrKind::Sim_Val_Integer,
                    })
                    .and_then(|i| i.try_into().map_err(|_| Error::AttrValueConversionError))
            }
        }
    };
}

macro_rules! impl_try_into_float {
    ($t:ty) => {
        impl TryFrom<AttrValue> for $t {
            type Error = Error;

            fn try_from(value: AttrValue) -> Result<Self> {
                value
                    .as_floating()
                    .ok_or_else(|| Error::AttrValueType {
                        actual: value.kind(),
                        expected: AttrKind::Sim_Val_Floating,
                    })
                    .and_then(|f| f.try_into().map_err(|_| Error::AttrValueConversionError))
            }
        }
    };
}

impl_try_into_unsigned! { u8 }
impl_try_into_unsigned! { u16 }
impl_try_into_unsigned! { u32 }
impl_try_into_unsigned! { u64 }
impl_try_into_unsigned! { usize }
impl_try_into_signed! { i8 }
impl_try_into_signed! { i16 }
impl_try_into_signed! { i32 }
impl_try_into_signed! { i64 }
impl_try_into_signed! { isize }
impl_try_into_float! { f64 }

impl TryFrom<AttrValue> for bool {
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value.as_boolean().ok_or_else(|| Error::AttrValueType {
            actual: value.kind(),
            expected: AttrKind::Sim_Val_Boolean,
        })
    }
}

impl TryFrom<AttrValue> for String {
    type Error = Error;
    fn try_from(value: AttrValue) -> Result<Self> {
        value.as_string().ok_or_else(|| Error::AttrValueType {
            actual: value.kind(),
            expected: AttrKind::Sim_Val_String,
        })
    }
}

impl<T> TryFrom<AttrValue> for Vec<T>
where
    T: TryFrom<AttrValue> + Clone,
{
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value.as_list()?.ok_or_else(|| Error::AttrValueType {
            actual: value.kind(),
            expected: AttrKind::Sim_Val_List,
        })
    }
}

impl<T> TryFrom<AttrValue> for HashSet<T>
where
    T: TryFrom<AttrValue> + Eq + Hash + Clone,
{
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value
            .as_list()?
            .ok_or_else(|| Error::AttrValueType {
                actual: value.kind(),
                expected: AttrKind::Sim_Val_List,
            })
            .map(|s| s.into_iter().collect::<HashSet<_>>())
    }
}

impl<T> TryFrom<AttrValue> for BTreeSet<T>
where
    T: TryFrom<AttrValue> + Ord + Clone,
{
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value
            .as_list()?
            .ok_or_else(|| Error::AttrValueType {
                actual: value.kind(),
                expected: AttrKind::Sim_Val_List,
            })
            .map(|s| s.into_iter().collect::<BTreeSet<_>>())
    }
}

impl<T, U> TryFrom<AttrValue> for HashMap<T, U>
where
    T: TryFrom<AttrValue> + Eq + Hash + Ord,
    U: TryFrom<AttrValue>,
{
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value
            .as_dict()?
            .ok_or_else(|| Error::AttrValueType {
                actual: value.kind(),
                expected: AttrKind::Sim_Val_Dict,
            })
            .map(|d| d.into_iter().collect::<HashMap<_, _>>())
    }
}

impl<T, U> TryFrom<AttrValue> for BTreeMap<T, U>
where
    T: TryFrom<AttrValue> + Ord,
    U: TryFrom<AttrValue>,
{
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        value.as_dict()?.ok_or_else(|| Error::AttrValueType {
            actual: value.kind(),
            expected: AttrKind::Sim_Val_Dict,
        })
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum AttrValueType {
    Nil,
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Float(OrderedFloat<f64>),
    String(String),
    List(Vec<Self>),
    Dict(BTreeMap<Self, Self>),
}

impl TryFrom<AttrValueType> for AttrValue {
    type Error = Error;

    fn try_from(value: AttrValueType) -> Result<Self> {
        match value {
            AttrValueType::Nil => Ok(AttrValue::nil()),
            AttrValueType::Bool(v) => v.try_into().map_err(|_| Error::AttrValueConversionError),
            AttrValueType::Signed(v) => v.try_into().map_err(|_| Error::AttrValueConversionError),
            AttrValueType::Unsigned(v) => v.try_into().map_err(|_| Error::AttrValueConversionError),
            AttrValueType::Float(v) => v.try_into().map_err(|_| Error::AttrValueConversionError),
            AttrValueType::String(v) => v.try_into().map_err(|_| Error::AttrValueConversionError),
            AttrValueType::List(v) => v.try_into(),
            AttrValueType::Dict(v) => v.try_into(),
        }
    }
}

macro_rules! impl_attr_value_type_from {
    ($t:ty, $($variant:tt)+) => {
        impl From<$t> for AttrValueType {
            fn from(value: $t) -> Self {
                $($variant)+(value.into())
            }
        }
    };
}

impl_attr_value_type_from! { u8, Self::Unsigned }
impl_attr_value_type_from! { u16, Self::Unsigned }
impl_attr_value_type_from! { u32, Self::Unsigned }
impl_attr_value_type_from! { u64, Self::Unsigned }
impl_attr_value_type_from! { i8, Self::Signed }
impl_attr_value_type_from! { i16, Self::Signed }
impl_attr_value_type_from! { i32, Self::Signed }
impl_attr_value_type_from! { i64, Self::Signed }
impl_attr_value_type_from! { f64, Self::Float }
impl_attr_value_type_from! { bool, Self::Bool}
impl_attr_value_type_from! { String, Self::String }

impl<T> From<Vec<T>> for AttrValueType
where
    T: Into<AttrValueType>,
{
    fn from(value: Vec<T>) -> Self {
        Self::List(value.into_iter().map(|i| i.into()).collect::<Vec<_>>())
    }
}

impl<T> From<BTreeSet<T>> for AttrValueType
where
    T: Into<AttrValueType>,
{
    fn from(value: BTreeSet<T>) -> Self {
        Self::List(value.into_iter().map(|i| i.into()).collect::<Vec<_>>())
    }
}

impl<T> From<HashSet<T>> for AttrValueType
where
    T: Into<AttrValueType>,
{
    fn from(value: HashSet<T>) -> Self {
        Self::List(value.into_iter().map(|i| i.into()).collect::<Vec<_>>())
    }
}

impl<T, U> From<BTreeMap<T, U>> for AttrValueType
where
    T: Into<AttrValueType>,
    U: Into<AttrValueType>,
{
    fn from(value: BTreeMap<T, U>) -> Self {
        Self::Dict(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect::<BTreeMap<_, _>>(),
        )
    }
}

impl<T, U> From<HashMap<T, U>> for AttrValueType
where
    T: Into<AttrValueType>,
    U: Into<AttrValueType>,
{
    fn from(value: HashMap<T, U>) -> Self {
        Self::Dict(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect::<BTreeMap<_, _>>(),
        )
    }
}

impl From<&str> for AttrValueType {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl<T> From<Option<T>> for AttrValueType
where
    T: Into<AttrValueType>,
{
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            Self::Nil
        }
    }
}

impl TryFrom<AttrValue> for AttrValueType {
    type Error = Error;

    fn try_from(value: AttrValue) -> Result<Self> {
        if value.is_nil() {
            Ok(Self::Nil)
        } else if let Some(b) = value.as_boolean() {
            Ok(Self::Bool(b))
        } else if let Some(i) = value.as_signed() {
            Ok(Self::Signed(i))
        } else if let Some(i) = value.as_unsigned() {
            Ok(Self::Unsigned(i))
        } else if let Some(f) = value.as_floating() {
            Ok(Self::Float(OrderedFloat(f)))
        } else if let Some(s) = value.as_string() {
            Ok(Self::String(s))
        } else if let Ok(Some(l)) = value.as_list() {
            Ok(Self::List(l))
        } else if let Ok(Some(d)) = value.as_dict() {
            Ok(Self::Dict(d))
        } else {
            Err(Error::AttrValueTypeUnknown)
        }
    }
}

/// Create a new invalid [`AttrValue`]
pub fn make_attr_invalid() -> AttrValue {
    AttrValue::invalid()
}

/// Create a new nil [`AttrValue`]
pub fn make_attr_nil() -> AttrValue {
    AttrValue::nil()
}

/// Create a new uint64 [`AttrValue`] with a value of `i`
pub fn make_attr_uint64(i: u64) -> AttrValue {
    i.into()
}

/// Create a new int64 [`AttrValue`] with a value of `i`
pub fn make_attr_int64(i: i64) -> AttrValue {
    i.into()
}

/// Create a new boolean [`AttrValue`] with a value of `b`
pub fn make_attr_boolean(b: bool) -> AttrValue {
    b.into()
}

/// Create a newly allocated string [`AttrValue`] with a value of `s`. The string is copied.
pub fn make_attr_string<S>(s: S) -> AttrValue
where
    S: AsRef<str>,
{
    s.as_ref().into()
}

/// Create a new floating point [`AttrValue`] with a value of `d`
pub fn make_attr_floating(d: f64) -> AttrValue {
    d.into()
}

/// Create a new object [`AttrValue`] with a value of `obj` The value is not copied, so the
/// pointer must remain valid.
pub fn make_attr_object(obj: *mut ConfObject) -> AttrValue {
    obj.into()
}

/// Create a new data [`AttrValue`], which is effectively a fat pointer to the data, with a given
/// size. The data will be copied into a [`Box`], which will be converted to a raw pointer. Care
/// must be taken to deallocate the data later either via Box::from_raw or by allowing SIMICS
/// to free it.
pub fn make_attr_data<T>(data: T) -> Result<AttrValue> {
    let data = Box::new(data);
    let data_raw = Box::into_raw(data);

    debug_assert!(
        std::mem::size_of_val(&data_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    let data_size = u32::try_from(size_of::<*mut T>())?;

    if !data_raw.is_null() || data_size == 0 {
        Err(Error::InvalidNullDataSize)
    } else {
        Ok(AttrValue(attr_value_t {
            private_kind: AttrKind::Sim_Val_Data,
            private_size: data_size,
            private_u: attr_value__bindgen_ty_1 {
                data: data_raw as *mut u8,
            },
        }))
    }
}

/// Create a new attribute list
pub fn make_attr_list<T>(attrs: Vec<T>) -> Result<AttrValue>
where
    T: Into<AttrValue>,
{
    attrs.try_into()
}

#[simics_exception]
/// Allocate a new attribute list of a given length
pub fn alloc_attr_list(length: u32) -> AttrValue {
    AttrValue(unsafe { SIM_alloc_attr_list(length) })
}

/// Create a new attribute dict
pub fn make_attr_dict(attrs: Vec<(AttrValue, AttrValue)>) -> Result<AttrValue> {
    attrs.try_into()
}

#[simics_exception]
/// Allocate a new attribute dictionary of a given size
pub fn alloc_attr_dict(length: u32) -> AttrValue {
    AttrValue(unsafe { SIM_alloc_attr_dict(length) })
}

#[simics_exception]
/// Set an element in an attribute list at a given index to a given value
pub fn attr_list_set_item(attr: &mut AttrValue, index: u32, elem: AttrValue) {
    unsafe { SIM_attr_list_set_item(attr.as_mut_ptr(), index, elem.into()) }
}

#[simics_exception]
/// Resize an attribute list. Shrinking the list triggers destruction of the dropped items
pub fn attr_list_resize(attr: &mut AttrValue, newsize: u32) {
    unsafe { SIM_attr_list_resize(attr.as_mut_ptr(), newsize) };
}

#[simics_exception]
/// Set an item at a key and index to a value in a given dictionary
pub fn attr_dict_set_item(attr: &mut AttrValue, index: u32, key: AttrValue, value: AttrValue) {
    unsafe { SIM_attr_dict_set_item(attr.as_mut_ptr(), index, key.into(), value.into()) };
}

#[simics_exception]
/// Resize an attribute dictionary. Shrinking the list triggers destruction of the dropped items
pub fn attr_dict_resize(attr: &mut AttrValue, newsize: u32) {
    unsafe { SIM_attr_dict_resize(attr.as_mut_ptr(), newsize) };
}

/// Check whether an [`AttrValue`] is nil
pub fn attr_is_nil(attr: AttrValue) -> bool {
    attr.kind() == AttrKind::Sim_Val_Nil
}

/// Check whether an [`AttrValue`] is int64
pub fn attr_is_int64(attr: AttrValue) -> bool {
    attr.is_integer()
}

/// Check whether an [`AttrValue`] is uint64
pub fn attr_is_uint64(attr: AttrValue) -> bool {
    attr.is_integer() && (attr.size() != 0 || attr.as_integer().is_some_and(|i| i >= 0))
}

/// Check whether an [`AttrValue`] is an integer
pub fn attr_is_integer(attr: AttrValue) -> bool {
    attr.is_integer()
}

/// Get an [`AttrValue`] as an integer
pub fn attr_integer(attr: AttrValue) -> Result<i64> {
    attr.as_integer().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_Integer,
    })
}

/// Check whether an [`AttrValue`] is a boolean
pub fn attr_is_boolean(attr: AttrValue) -> bool {
    attr.is_boolean()
}

/// Get an [`AttrValue`] as a boolean
pub fn attr_boolean(attr: AttrValue) -> Result<bool> {
    attr.as_boolean().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_Boolean,
    })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_string(attr: AttrValue) -> bool {
    attr.is_string()
}

/// Get an [`AttrValue`] as a String
pub fn attr_string(attr: AttrValue) -> Result<String> {
    attr.as_string().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_String,
    })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_floating(attr: AttrValue) -> bool {
    attr.is_floating()
}

/// Get an [`AttrValue`] as a f64
pub fn attr_floating(attr: AttrValue) -> Result<f64> {
    attr.as_floating().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_Floating,
    })
}

/// Check whether an [`AttrValue`] is a String
pub fn attr_is_object(attr: AttrValue) -> bool {
    attr.is_object()
}

/// Get an [`AttrValue`] as an object
pub fn attr_object(attr: AttrValue) -> Result<*mut ConfObject> {
    attr.as_object().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_Object,
    })
}

/// Get an [`AttrValue`] as an object or nil if the object is a null pointer
pub fn attr_object_or_nil(attr: AttrValue) -> *mut ConfObject {
    attr.as_object().unwrap_or(null_mut())
}

/// Check whether an [`AttrValue`] is invalid
pub fn attr_is_invalid(attr: AttrValue) -> bool {
    attr.is_invalid()
}

/// Check whether an [`AttrValue`] is data
pub fn attr_is_data(attr: AttrValue) -> bool {
    attr.is_data()
}

/// Get the size of an [`AttrValue`]'s data
pub fn attr_data_size(attr: AttrValue) -> Result<u32> {
    attr.is_data()
        .then(|| attr.size())
        .ok_or_else(|| Error::AttrValueType {
            actual: attr.kind(),
            expected: AttrKind::Sim_Val_Data,
        })
}

pub fn attr_data<T>(attr: AttrValue) -> Result<T>
where
    T: Clone,
{
    attr.as_data().ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_Data,
    })
}

/// Check whether an [`AttrValue`] is a list
pub fn attr_is_list(attr: AttrValue) -> bool {
    attr.is_list()
}

/// Get the size of an [`AttrValue`]'s list
pub fn attr_list_size(attr: AttrValue) -> Result<u32> {
    attr.is_list()
        .then(|| attr.size())
        .ok_or_else(|| Error::AttrValueType {
            actual: attr.kind(),
            expected: AttrKind::Sim_Val_List,
        })
}

/// Retrieve a list item from an attr list without converting the item to a specific type
pub fn attr_list_item<T>(attr: AttrValue, index: usize) -> Result<AttrValue> {
    let list: Vec<AttrValue> = attr.as_list()?.ok_or_else(|| Error::AttrValueType {
        actual: attr.kind(),
        expected: AttrKind::Sim_Val_List,
    })?;

    list.get(index)
        .cloned()
        .ok_or(Error::AttrValueListIndexOutOfBounds {
            index,
            length: list.len(),
        })
}

/// Check whether an [`AttrValue`] is a dict
pub fn attr_is_dict(attr: AttrValue) -> bool {
    attr.is_dict()
}

/// Get the size of an an [`AttrValue`]'s dict
pub fn attr_dict_size(attr: AttrValue) -> Result<u32> {
    attr.is_dict()
        .then(|| attr.size())
        .ok_or(Error::AttrValueType {
            actual: attr.kind(),
            expected: AttrKind::Sim_Val_Data,
        })
}

/// Get a key for an [`AttrValue`]'s dict
pub fn attr_dict_key(attr: AttrValue, index: u32) -> Result<AttrValue> {
    if index < attr.size() {
        attr.is_dict()
            .then(|| {
                AttrValue(
                    // NOTE: This is leaked because the semantics of data ownership are that
                    // returned data is owned by the attr and ownership is *not* returned to the
                    // caller. It must be freed elsewhere.
                    Box::leak(unsafe {
                        Box::from_raw(attr.0.private_u.dict.offset(index as isize))
                    })
                    .key,
                )
            })
            .ok_or_else(|| Error::AttrValueType {
                actual: attr.kind(),
                expected: AttrKind::Sim_Val_Dict,
            })
    } else {
        Err(Error::AttrValueDictIndexOutOfBounds {
            index: index as usize,
            size: attr.size() as usize,
        })
    }
}

/// Get a value for an [`AttrValue`]'s dict
pub fn attr_dict_value(attr: AttrValue, index: u32) -> Result<AttrValue> {
    if index < attr.size() {
        attr.is_dict()
            .then(|| {
                AttrValue(
                    // NOTE: This is leaked because the semantics of data ownership are that
                    // returned data is owned by the attr and ownership is *not* returned to the
                    // caller. It must be freed elsewhere.
                    Box::leak(unsafe {
                        Box::from_raw(attr.0.private_u.dict.offset(index as isize))
                    })
                    .value,
                )
            })
            .ok_or_else(|| Error::AttrValueType {
                actual: attr.kind(),
                expected: AttrKind::Sim_Val_Dict,
            })
    } else {
        Err(Error::AttrValueDictIndexOutOfBounds {
            index: index as usize,
            size: attr.size() as usize,
        })
    }
}

#[simics_exception]
/// Free an attr value. [`attr_free`] should be used instead where possible.
pub fn free_attribute(attr: AttrValue) {
    unsafe { SIM_free_attribute(attr.0) }
}
