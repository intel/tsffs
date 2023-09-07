// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Utilities for linking to the simics library. This linking process is only compatible
//! with Linux.

use crate::{
    manifest::package_version, package::PublicPackageNumber, simics::home::simics_home,
    util::find_file_in_dir,
};
use anyhow::{anyhow, ensure, Result};
use regex::Regex;
use std::{
    collections::HashSet,
    process::{Command, Stdio},
};

/// Emit cargo directives to link to SIMICS given a particular version constraint
pub fn link_simics_linux<S>(version_constraint: S) -> Result<()>
where
    S: AsRef<str>,
{
    let simics_home_dir = simics_home()?;

    let simics_base_info = package_version(
        &simics_home_dir,
        PublicPackageNumber::Base.into(),
        version_constraint.as_ref().parse()?,
    )?;
    let simics_base_version = simics_base_info.version.clone();
    let simics_base_dir = simics_base_info.path;
    println!(
        "Found simics base for version '{}' in {}",
        version_constraint.as_ref(),
        simics_base_dir.display()
    );

    let simics_common_lib = find_file_in_dir(&simics_base_dir, "libsimics-common.so")?;
    println!(
        "Found simics common library: {}",
        simics_common_lib.display()
    );

    let simics_bin_dir = simics_home_dir
        .join(format!("simics-{}", &simics_base_version))
        .join("bin");

    ensure!(
        simics_bin_dir.is_dir(),
        "No bin directory found in {}",
        simics_home_dir.display()
    );

    let mut output = Command::new("ld.so")
        .arg(&simics_common_lib)
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| {
            eprintln!("Error running 'ld.so': {}", e);
        })
        .ok();

    if output.is_none() || output.as_ref().is_some_and(|o| !o.status.success()) {
        output = Some(
            Command::new("ldd")
                .arg(simics_common_lib)
                .stdout(Stdio::piped())
                .output()
                .map_err(|e| {
                    eprintln!("Error running 'ldd': {}", e);
                    e
                })?,
        );
    }

    let output = output.ok_or_else(|| anyhow!("No output from 'ld.so' or 'ldd'"))?;

    ensure!(
        output.status.success(),
        "Command failed to obtain dependency listing"
    );

    let ld_line_pattern = Regex::new(r"\s*([^\s]+)\s*=>\s*(.*)")?;
    let mut notfound_libs: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| {
            if let Some(captures) = ld_line_pattern.captures(l) {
                captures.get(1)
            } else {
                None
            }
        })
        .map(|m| m.as_str().to_string())
        .collect();

    if !notfound_libs.contains(&"libsimics-common.so".to_string()) {
        notfound_libs.push("libsimics-common.so".to_string());
    }

    println!("Locating {}", notfound_libs.join(", "));

    let mut lib_search_dirs = HashSet::new();

    // NOTE: Right now, there aren't any recursive dependencies we need to worry about, it's only
    // vtutils, package-paths, libpython, and libsimics-common. *if* this changes, we will need to
    // reimplement this search recursively
    println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");

    for lib_name in notfound_libs {
        if let Ok(found_lib) = find_file_in_dir(&simics_base_dir, &lib_name) {
            // If we are running a build script right now, we will copy the library
            let found_lib_parent = found_lib
                .parent()
                .ok_or_else(|| anyhow!("No parent path found"))?;
            lib_search_dirs.insert(found_lib_parent.to_path_buf().canonicalize().map_err(|e| {
                anyhow!(
                    "Failed to canonicalize found library parent path {}: {}",
                    found_lib_parent.display(),
                    e
                )
            })?);
            println!("cargo:rustc-link-lib=dylib:+verbatim={}", &lib_name);
        } else {
            eprintln!("Warning! Could not find simics dependency library {}. Chances are, it is a system library and this is OK.", lib_name);
        }
    }

    for lib_search_dir in &lib_search_dirs {
        println!(
            "cargo:rustc-link-search=native={}",
            lib_search_dir.display()
        );
        // println!(
        //     "cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/{}{}",
        //     UPDIR_MAX,
        //     lib_search_dir.display()
        // )
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            lib_search_dir.display()
        );
    }

    // NOTE: This only works for `cargo run` and `cargo test` and won't work for just running
    // the output binary
    let search_dir_strings = lib_search_dirs
        .iter()
        .map(|pb| pb.to_string_lossy())
        .collect::<Vec<_>>();

    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={}",
        search_dir_strings.join(";")
    );
    Ok(())
}
