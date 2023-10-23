// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// The default register the fuzzer expects to contain a pointer to an area to write
/// each testcase into when using an in-target harness
pub const DEFAULT_TESTCASE_AREA_REGISTER_NAME: &str = "x0";
/// The default register the fuzzer expects to contain a pointer to a variable,
/// initially containing the maximum size of the area pointed to by
/// `DEFAULT_TESTCASE_AREA_REGISTER_NAME`, which will be written each fuzzer execution
/// to contain the actual size of the current testcase.
pub const DEFAULT_TESTCASE_SIZE_REGISTER_NAME: &str = "x1";
