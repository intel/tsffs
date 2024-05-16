// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub(crate) enum FuzzerMessage {
    String(String),
    Interesting { indices: Vec<usize>, input: Vec<u8> },
    Crash { indices: Vec<usize>, input: Vec<u8> },
    Timeout { indices: Vec<usize>, input: Vec<u8> },
}
