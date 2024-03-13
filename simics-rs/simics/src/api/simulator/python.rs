// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Direct Python interface

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::{
    simics_exception,
    sys::{
        SIM_call_python_function, SIM_run_python, SIM_source_python, VT_call_python_module_function,
    },
    AttrValue, Error, Result,
};
use raw_cstr::raw_cstr;
use std::{any::type_name, path::Path};

#[simics_exception]
/// Source a python file of SIMICS python code
pub fn source_python<P>(file: P) -> Result<()>
where
    P: AsRef<Path>,
{
    unsafe {
        SIM_source_python(raw_cstr(
            file.as_ref().to_str().ok_or_else(|| Error::ToString)?,
        )?)
    };
    Ok(())
}

#[simics_exception]
/// Run (by eval-ing) python code in the simulator context. The result is returned as an
/// `AttrValue`, which is typed according to the python command run.
pub fn run_python<S>(line: S) -> Result<AttrValue>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_run_python(raw_cstr(line)?) }.into())
}

#[simics_exception]
/// Call a python function with a set of arguments. The arguments must be
/// convertible to `AttrValue`.
///
/// # Examples
///
/// We can run python code like so:
///
/// ```rust,ignore
/// let res = call_python_function("print", ["a", "b", "c"])?;
/// ```
pub fn call_python_function<S, I, T>(function: S, args: I) -> Result<AttrValue>
where
    S: AsRef<str>,
    I: IntoIterator<Item = T>,
    T: TryInto<AttrValue>,
{
    let args: Vec<AttrValue> = args
        .into_iter()
        .map(|a| {
            a.try_into().map_err(|_| Error::ToAttrValueConversionError {
                ty: type_name::<T>().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let args: AttrValue = args.try_into()?;
    let mut args = args.into();
    Ok(unsafe { SIM_call_python_function(raw_cstr(function)?, &mut args as *mut _) }.into())
}

#[simics_exception]
/// Call a python function with a set of arguments. The arguments must be
/// convertible to `AttrValue`.
///
/// # Examples
///
/// We can run python code like so. If you need to pass a heterogeneous list of
/// arguments, you need to convert them to `AttrValue`s before passing them to this
/// function because we cannot create a heterogeneous `Vec`. For homogeneous lists of
/// arguments, they can be passed directly as in `call_python_function`.
///
/// ```rust,ignore
/// let level: AttrValue = 1i32.into();
/// let sim: AttrValue = get_object("sim")?.into()?;
/// let group: AttrValue = 0i32.into();
/// let message: AttrValue = "Hello".into();
/// let res = call_python_module_function("simics", "SIM_log_info" [level, sim, group, message])?;
/// ```
pub fn call_python_module_function<S, I, T>(module: S, function: S, args: I) -> Result<AttrValue>
where
    S: AsRef<str>,
    I: IntoIterator<Item = T>,
    T: TryInto<AttrValue>,
{
    let args: Vec<AttrValue> = args
        .into_iter()
        .map(|a| {
            a.try_into().map_err(|_| Error::ToAttrValueConversionError {
                ty: type_name::<T>().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let args: AttrValue = args.try_into()?;
    let mut args = args.into();
    Ok(unsafe {
        VT_call_python_module_function(
            raw_cstr(module.as_ref())?,
            raw_cstr(function.as_ref())?,
            &mut args as *mut _,
        )
    }
    .into())
}
