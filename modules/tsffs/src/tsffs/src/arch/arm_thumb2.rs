// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// The default start magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_START: u64 = 1;
/// The default stop magic mnumber the fuzzer expects to be triggered, either
/// via an in-target macro or another means.
pub const DEFAULT_MAGIC_STOP: u64 = 2;
/// The default register the fuzzer expects to contain a pointer to an area to write
/// each testcase into when using an in-target harness
pub const DEFAULT_TESTCASE_AREA_REGISTER_NAME: &str = "r0";
/// The default register the fuzzer expects to contain a pointer to a variable,
/// initially containing the maximum size of the area pointed to by
/// `DEFAULT_TESTCASE_AREA_REGISTER_NAME`, which will be written each fuzzer execution
/// to contain the actual size of the current testcase.
pub const DEFAULT_TESTCASE_SIZE_REGISTER_NAME: &str = "r1";