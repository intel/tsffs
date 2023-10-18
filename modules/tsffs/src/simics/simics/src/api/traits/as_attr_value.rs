// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    api::{
        attr_boolean, attr_dict_key, attr_dict_size, attr_dict_value, attr_floating, attr_integer,
        attr_list_item, attr_list_size, attr_string, make_attr_boolean, make_attr_dict,
        make_attr_floating, make_attr_int64, make_attr_list, make_attr_string_adopt,
        make_attr_uint64, AttrValue,
    },
    Error, Result,
};
use ordered_float::OrderedFloat;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::Hash,
};

pub trait AsAttrValue {
    /// Return a copy of this object as an attr value. Used to return data through the
    /// SIMICS API
    fn as_attr_value(&self) -> Result<AttrValue>;
}

macro_rules! impl_from_unsigned {
    ($t:ty) => {
        impl AsAttrValue for $t {
            fn as_attr_value(&self) -> Result<AttrValue> {
                #[allow(clippy::unnecessary_cast)]
                make_attr_uint64(*self as u64)
            }
        }
    };
}

macro_rules! impl_from_signed {
    ($t:ty) => {
        impl AsAttrValue for $t {
            fn as_attr_value(&self) -> Result<AttrValue> {
                #[allow(clippy::unnecessary_cast)]
                Ok(make_attr_int64(*self as i64))
            }
        }
    };
}

macro_rules! impl_from_float {
    ($t:ty) => {
        impl AsAttrValue for $t {
            fn as_attr_value(&self) -> Result<AttrValue> {
                #[allow(clippy::unnecessary_cast)]
                Ok(make_attr_floating(*self as f64))
            }
        }
    };
}

impl_from_unsigned! { u8 }
impl_from_unsigned! { u16 }
impl_from_unsigned! { u32 }
impl_from_unsigned! { u64 }

impl_from_signed! { i8 }
impl_from_signed! { i16 }
impl_from_signed! { i32 }
impl_from_signed! { i64 }

impl_from_float! { f32 }
impl_from_float! { f64 }

impl AsAttrValue for bool {
    fn as_attr_value(&self) -> Result<AttrValue> {
        Ok(make_attr_boolean(*self))
    }
}

impl AsAttrValue for String {
    fn as_attr_value(&self) -> Result<AttrValue> {
        make_attr_string_adopt(self)
    }
}

impl<T> AsAttrValue for Vec<T>
where
    T: AsAttrValue,
{
    fn as_attr_value(&self) -> Result<AttrValue> {
        let attrs = self
            .iter()
            .map(|i| i.as_attr_value())
            .collect::<Result<Vec<AttrValue>>>()?;
        make_attr_list(attrs)
    }
}

impl<T> AsAttrValue for HashSet<T>
where
    T: AsAttrValue,
{
    fn as_attr_value(&self) -> Result<AttrValue> {
        let attrs = self
            .iter()
            .map(|i| i.as_attr_value())
            .collect::<Result<Vec<AttrValue>>>()?;
        make_attr_list(attrs)
    }
}

impl<T> AsAttrValue for BTreeSet<T>
where
    T: AsAttrValue,
{
    fn as_attr_value(&self) -> Result<AttrValue> {
        let attrs = self
            .iter()
            .map(|i| i.as_attr_value())
            .collect::<Result<Vec<AttrValue>>>()?;
        make_attr_list(attrs)
    }
}

impl<T, U> AsAttrValue for HashMap<T, U>
where
    T: AsAttrValue,
    U: AsAttrValue,
{
    fn as_attr_value(&self) -> Result<AttrValue> {
        let attrs = self
            .iter()
            .map(|(k, v)| match (k.as_attr_value(), v.as_attr_value()) {
                (Ok(k), Ok(v)) => Ok((k, v)),
                (Err(e), _) => Err(e),
                (_, Err(e)) => Err(e),
            })
            .collect::<Result<Vec<(AttrValue, AttrValue)>>>()?;
        make_attr_dict(attrs)
    }
}

impl<T, U> AsAttrValue for BTreeMap<T, U>
where
    T: AsAttrValue,
    U: AsAttrValue,
{
    fn as_attr_value(&self) -> Result<AttrValue> {
        let attrs = self
            .iter()
            .map(|(k, v)| match (k.as_attr_value(), v.as_attr_value()) {
                (Ok(k), Ok(v)) => Ok((k, v)),
                (Err(e), _) => Err(e),
                (_, Err(e)) => Err(e),
            })
            .collect::<Result<Vec<(AttrValue, AttrValue)>>>()?;
        make_attr_dict(attrs)
    }
}

pub trait FromAttrValue {
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_to_integer {
    ($t:ty) => {
        impl FromAttrValue for $t {
            fn from_attr_value(value: AttrValue) -> Result<Self>
            where
                Self: Sized,
            {
                attr_integer(value).and_then(|i| <$t>::try_from(i).map_err(Error::from))
            }
        }
    };
}

macro_rules! impl_to_float {
    ($t:ty) => {
        impl FromAttrValue for $t {
            fn from_attr_value(value: AttrValue) -> Result<Self>
            where
                Self: Sized,
            {
                Ok(attr_floating(value)? as $t)
            }
        }
    };
}

impl_to_integer! { u8 }
impl_to_integer! { u16 }
impl_to_integer! { u32 }
impl_to_integer! { u64 }

impl_to_integer! { i8 }
impl_to_integer! { i16 }
impl_to_integer! { i32 }
impl_to_integer! { i64 }

impl_to_float! { f32 }
impl_to_float! { f64 }

impl FromAttrValue for bool {
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        attr_boolean(value)
    }
}

impl FromAttrValue for String {
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        attr_string(value)
    }
}

impl<T> FromAttrValue for Vec<T>
where
    T: FromAttrValue,
{
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        let size = attr_list_size(value)?;

        (0..size)
            .map(|i| attr_list_item(value, i).and_then(T::from_attr_value))
            .collect::<Result<Vec<T>>>()
    }
}

impl<T> FromAttrValue for HashSet<T>
where
    T: FromAttrValue + Hash + Eq,
{
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        let size = attr_list_size(value)?;

        (0..size)
            .map(|i| attr_list_item(value, i).and_then(T::from_attr_value))
            .collect::<Result<HashSet<T>>>()
    }
}

impl<T> FromAttrValue for BTreeSet<T>
where
    T: FromAttrValue + Ord,
{
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        let size = attr_list_size(value)?;

        (0..size)
            .map(|i| attr_list_item(value, i).and_then(T::from_attr_value))
            .collect::<Result<BTreeSet<T>>>()
    }
}

impl<T, U> FromAttrValue for HashMap<T, U>
where
    T: FromAttrValue + Hash + Eq,
    U: FromAttrValue,
{
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        let size = attr_dict_size(value)?;

        let items = (0..size)
            .map(|i| {
                attr_dict_key(value, i)
                    .and_then(|k| attr_dict_value(value, i).map(|v| (k, v)))
                    .and_then(|(k, v)| T::from_attr_value(k).map(|k| (k, v)))
                    .and_then(|(k, v)| U::from_attr_value(v).map(|v| (k, v)))
            })
            .collect::<Result<Vec<(T, U)>>>()?;

        Ok(items.into_iter().collect::<HashMap<T, U>>())
    }
}

impl<T, U> FromAttrValue for BTreeMap<T, U>
where
    T: FromAttrValue + Ord,
    U: FromAttrValue,
{
    fn from_attr_value(value: AttrValue) -> Result<Self>
    where
        Self: Sized,
    {
        let size = attr_dict_size(value)?;

        let items = (0..size)
            .map(|i| {
                attr_dict_key(value, i)
                    .and_then(|k| attr_dict_value(value, i).map(|v| (k, v)))
                    .and_then(|(k, v)| T::from_attr_value(k).map(|k| (k, v)))
                    .and_then(|(k, v)| U::from_attr_value(v).map(|v| (k, v)))
            })
            .collect::<Result<Vec<(T, U)>>>()?;

        Ok(items.into_iter().collect::<BTreeMap<T, U>>())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum AttrValueType {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    String(String),
    List(Vec<AttrValueType>),
    Set(BTreeSet<AttrValueType>),
    Dict(BTreeMap<AttrValueType, AttrValueType>),
}

impl AsAttrValue for AttrValueType {
    fn as_attr_value(&self) -> Result<AttrValue> {
        match self {
            AttrValueType::Bool(v) => (*v).as_attr_value(),
            AttrValueType::U8(v) => (*v).as_attr_value(),
            AttrValueType::U16(v) => (*v).as_attr_value(),
            AttrValueType::U32(v) => (*v).as_attr_value(),
            AttrValueType::U64(v) => (*v).as_attr_value(),
            AttrValueType::I8(v) => (*v).as_attr_value(),
            AttrValueType::I16(v) => (*v).as_attr_value(),
            AttrValueType::I32(v) => (*v).as_attr_value(),
            AttrValueType::I64(v) => (*v).as_attr_value(),
            AttrValueType::F32(v) => (*v).as_attr_value(),
            AttrValueType::F64(v) => (*v).as_attr_value(),
            AttrValueType::String(v) => v.clone().as_attr_value(),
            AttrValueType::List(v) => v.clone().as_attr_value(),
            AttrValueType::Set(v) => v.clone().as_attr_value(),
            AttrValueType::Dict(v) => v.clone().as_attr_value(),
        }
    }
}
