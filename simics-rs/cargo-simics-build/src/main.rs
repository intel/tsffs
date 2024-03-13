// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

// #![deny(missing_docs)]

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    // Parse CLI arguments
    let cmd = cargo_simics_build::Cmd::parse();
    cargo_simics_build::App::run(cmd)?;

    Ok(())
}
