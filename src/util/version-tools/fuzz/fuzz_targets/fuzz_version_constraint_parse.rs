// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use std::str::FromStr;
use version_tools::VersionConstraint;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    VersionConstraint::from_str(&input).ok();
});
