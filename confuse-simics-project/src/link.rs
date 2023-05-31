use std::{
    collections::HashSet,
    process::{Command, Stdio},
};

use crate::{simics_home, util::find_file_in_simics_base};
use anyhow::{ensure, Context, Result};
use confuse_simics_manifest::simics_base_version;
use log::info;
use regex::Regex;

/// Emit cargo directives to link to SIMICS given a particular version constraint
pub fn link_simics<S: AsRef<str>>(version_constraint: S) -> Result<()> {
    let simics_home_dir = simics_home()?;

    let simics_base_info = simics_base_version(&simics_home_dir, &version_constraint)?;
    let simics_base_dir = simics_base_info.get_package_path(&simics_home_dir)?;
    println!(
        "Found simics base for version '{}' in {}",
        version_constraint.as_ref(),
        simics_base_dir.display()
    );

    let simics_common_lib = find_file_in_simics_base(&simics_base_dir, "libsimics-common.so")?;
    println!(
        "Found simics common library: {}",
        simics_common_lib.display()
    );

    let simics_bin_dir = simics_home_dir
        .join(format!(
            "simics-{}",
            simics_base_version(simics_home()?, version_constraint)?.version
        ))
        .join("bin");

    ensure!(
        simics_bin_dir.is_dir(),
        "No bin directory found in {}",
        simics_home_dir.display()
    );

    let mut output = Command::new("ld.so")
        .arg(&simics_common_lib)
        .stdout(Stdio::piped())
        .output()?;

    if !output.status.success() {
        output = Command::new("ldd")
            .arg(simics_common_lib)
            .stdout(Stdio::piped())
            .output()?;
    }

    ensure!(
        output.status.success(),
        "Command failed to obtain dependency listing"
    );

    let ld_line_pattern = Regex::new(r#"\s*([^\s]+)\s*=>\s*not\sfound"#)?;
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

    for lib_name in notfound_libs {
        println!("cargo:rustc-link-lib=dylib:+verbatim={}", lib_name);
        let found_lib = find_file_in_simics_base(&simics_base_dir, lib_name)?;
        let found_lib_parent = found_lib.parent().context("No parent path found")?;
        lib_search_dirs.insert(found_lib_parent.to_path_buf());
    }

    for lib_search_dir in &lib_search_dirs {
        println!(
            "cargo:rustc-link-search=native={}",
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
