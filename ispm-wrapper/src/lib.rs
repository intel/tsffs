// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Wrappers for the small subset of ISPM commands the fuzzer and its build processes need to
//!
//! To implement or update this subset using public SIMICS, install ISPM (Intel SIMICS
//! Package Manager) to `~/simics-public/ispm/`, then:
//!
//! ```sh,ignore
//! npx asar -h
//! npx
//! npx asar extract ~/simics-public/ispm/resources/app.asar \
//!     ~/simics-public/ispm/resources/app.asar.extracted
//! npx webcrack ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.js \
//!     > ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.unmin.js
//! npx deobfuscator ~/simics-public/ispm/resources/app.asar.extracted/dist/electron/main.js
//! ```

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct IPathObject {
    id: isize,
    priority: isize,
    value: PathBuf,
    enabled: bool,
    #[serde(rename = "isWritable")]
    writable: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RepoPath {
    value: String,
    enabled: bool,
    priority: isize,
    id: isize,
}

#[derive(Deserialize, Clone, Debug)]
/// V3 configuration, all fields are optional so older configs that we support should also work
/// without an issue
pub struct Config {
    archives: Vec<RepoPath>,
    #[serde(rename = "cacheTimeout")]
    cache_timeout: Option<isize>,
    #[serde(rename = "installPath")]
    install_path: IPathObject,
}
