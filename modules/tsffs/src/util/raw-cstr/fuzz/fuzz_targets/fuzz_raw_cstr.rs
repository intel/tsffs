// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![forbid(unsafe_code)]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use raw_cstr::raw_cstr;

#[derive(Arbitrary, Debug)]
struct Input {
    inputs: Vec<String>,
}

fuzz_target!(|data: Input| -> Corpus {
    for s in &data.inputs {
        if !s.contains('\0') {
            let _ = raw_cstr(s).unwrap();
        } else {
            return Corpus::Reject;
        }
    }

    Corpus::Keep
});
