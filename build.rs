// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// The environment variable containing the path to the Simics installation
const SIMICS_BASE_ENV: &str = "SIMICS_BASE";

fn main() {
    println!("cargo:rerun-if-env-changed={SIMICS_BASE_ENV}");
    simics_build_utils::emit_cfg_directives().expect("Failed to emit cfg directives");
    simics_build_utils::emit_link_info().expect("Failed to emit link info");
}
