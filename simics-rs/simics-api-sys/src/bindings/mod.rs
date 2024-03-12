// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Raw bindings to the SIMICS API

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]

include!(concat!(env!("OUT_DIR"), "/bindings-auto.rs"));

pub const SIMICS_API_BINDINGS: &str = include_str!(concat!(env!("OUT_DIR"), "/bindings-auto.rs"));
