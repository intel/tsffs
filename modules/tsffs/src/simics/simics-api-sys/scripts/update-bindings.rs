#!/usr/bin/env -S cargo +nightly -Zscript

// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! ```cargo
//! [dependencies]
//! clap = { version = "*", features = ["derive"] }
//! anyhow = "*"
//! bindgen = "*"
//! bytes = "*"
//! flate2 = "*"
//! reqwest = { version = "*", features = ["blocking"] }
//! tar = "*"
//! toml_edit = "*"
//! versions = "*"
//! walkdir = "*"
//! ```

// Bindings build script for SIMICS sys API
//
// This script will download every version of the SIMICS 6 Base package and generate an API
// bindings file for each one. It will output the bindings to the `src/bindings` directory of
// this crate, where they will be included by feature flag with the `src/bindings/mod.rs` file.

use anyhow::{anyhow, bail, ensure, Result};
use bindgen::{
    callbacks::{MacroParsingBehavior, ParseCallbacks},
    Builder, FieldVisibilityKind,
};
use bytes::{buf::Reader, Buf};
use clap::Parser;
use flate2::bufread::GzDecoder;
use reqwest::blocking::get;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs::{create_dir_all, read, read_dir, File, OpenOptions},
    io::{copy, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread::spawn,
};
use tar::Archive;
use toml_edit::{value, Array, Document, Item, Table};
use versions::Versioning;
use walkdir::WalkDir;

/// Base path to where the SIMICS packages can be downloaded from
const SIMICS_BASE_PACKAGE_NUMBER: usize = 1000;

/// Base package filename pattern
const SIMICS_BASE_PACKAGE_FILENAME_PATTERN: &str = r#"simics-pkg-1000-([0-9\.]+)-linux64.tar"#;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    /// List of base versions, defaults to everything if none are provided
    base_versions: Vec<String>,
    #[arg(short, long)]
    /// Directory to use to download and unpack packages
    packages_dir: PathBuf,
    #[arg(short = 'B', long)]
    /// Directory to use to output bindings
    bindings_dir: PathBuf,
    #[arg(short, long)]
    /// Cargo.toml file to update with required features
    toml_file: PathBuf,
    #[arg(short, long)]
    /// ISPM tar.gz file
    ispm_tarball_url: String,
    #[arg(short, long)]
    simics_package_repo_url: String,
}

// https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-1312298570
const IGNORE_MACROS: [&str; 20] = [
    "FE_DIVBYZERO",
    "FE_DOWNWARD",
    "FE_INEXACT",
    "FE_INVALID",
    "FE_OVERFLOW",
    "FE_TONEAREST",
    "FE_TOWARDZERO",
    "FE_UNDERFLOW",
    "FE_UPWARD",
    "FP_INFINITE",
    "FP_INT_DOWNWARD",
    "FP_INT_TONEAREST",
    "FP_INT_TONEARESTFROMZERO",
    "FP_INT_TOWARDZERO",
    "FP_INT_UPWARD",
    "FP_NAN",
    "FP_NORMAL",
    "FP_SUBNORMAL",
    "FP_ZERO",
    "IPPORT_RESERVED",
];

fn download_file<S, P>(url: S, path: P) -> Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let response = get(url.as_ref())?;

    println!("{:?}", response);
    let mut dest = if path.as_ref().is_dir() {
        bail!("Can't download into directory. Provide a file path");
    } else {
        ensure!(
            path.as_ref()
                .parent()
                .ok_or_else(|| anyhow!("No parent of path"))?
                .is_dir(),
            "Parent of path must be directory"
        );
        File::create(path.as_ref())?
    };
    copy(&mut response.bytes()?.reader(), &mut dest)?;
    Ok(())
}

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if self.0.contains(name) {
            MacroParsingBehavior::Ignore
        } else {
            MacroParsingBehavior::Default
        }
    }
}

impl IgnoreMacros {
    fn new() -> Self {
        Self(IGNORE_MACROS.into_iter().map(|s| s.to_string()).collect())
    }
}

fn generate_include_wrapper<P>(base_package_path: P) -> Result<String>
where
    P: AsRef<Path>,
{
    let simics_include_path = base_package_path.as_ref().join("src").join("include");
    let simics_include_path = simics_include_path.canonicalize()?;

    let mut include_paths = WalkDir::new(&simics_include_path)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let simics_include_path = &simics_include_path;
            match p.path().extension() {
                Some(e) => {
                    if e == "h" {
                        match p.path().canonicalize() {
                            Ok(p) => p.strip_prefix(simics_include_path).map_or_else(
                                |e| {
                                    eprintln!(
                                        "Failed to strip prefix {} from {}: {}",
                                        simics_include_path.display(),
                                        p.display(),
                                        e
                                    );
                                    None::<PathBuf>
                                },
                                |p| Some(p.to_path_buf()),
                            ),
                            Err(e) => {
                                eprintln!(
                                    "Failed to canonicalize path {}: {}",
                                    p.path().display(),
                                    e
                                );
                                None
                            }
                        }
                    } else {
                        eprintln!("Ignoring path {}, no '.h' extension", p.path().display());
                        None
                    }
                }
                None => {
                    eprintln!("Ignoring path {}, no extension", p.path().display());
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    // We need to move python-header.h to the beginning of the list
    if let Some(python_hdr_pos) = include_paths
        .iter()
        .position(|p| p.file_name() == Some(OsStr::new("python-header.h")))
    {
        include_paths.swap(0, python_hdr_pos);
    }

    let hdr_denylist = vec![
        // Most of these are denylisted because they include follower-time.h and it's :/ broken
        "global.h",
        "vtutils.h",
        "libfollower.h",
        "follower-time.h",
        "follower.h",
        "link-endpoint.h",
        "data-structs.h",
        // slave-time.h/slave.h is also broken (it is the old name for follower)
        "slave-time.h",
        "slave.h",
    ];

    hdr_denylist.iter().for_each(|le| {
        if let Some(pos) = include_paths
            .iter()
            .position(|p| p.file_name() == Some(OsStr::new(le)))
        {
            include_paths.remove(pos);
        }
    });

    let include_stmts = include_paths
        .iter()
        .map(|p| format!("#include <{}>", p.display()))
        .collect::<Vec<_>>();
    let wrapper = include_stmts.join("\n") + "\n";

    Ok(wrapper)
}

fn get_existing_versions(args: &Args) -> Result<Vec<String>> {
    let mut versions = args
        .bindings_dir
        .read_dir()?
        .filter_map(|de| de.ok())
        .map(|de| de.path())
        .filter(|p| {
            p.is_file()
                && if let Some(ext) = p.extension() {
                    ext == "rs"
                } else {
                    false
                }
        })
        .filter_map(|p| {
            p.file_name()
                .map(|fname| fname.to_string_lossy().to_string())
        })
        .filter(|fname| fname.starts_with("bindings-"))
        .filter_map(|fname| {
            let fname_parts = fname.split('-').collect::<Vec<_>>();
            fname_parts
                .get(1)
                .map(|part| part.to_string().replace(".rs", ""))
        })
        .filter_map(|v| Versioning::new(&v))
        .collect::<Vec<_>>();

    versions.sort();

    Ok(versions.iter().map(|v| format!("{}", v)).collect())
}

fn generate_bindings(args: &Args) -> Result<()> {
    if !args.bindings_dir.exists() {
        create_dir_all(&args.bindings_dir)?;
    }

    let existing_versions = get_existing_versions(args)?;

    let mut base_versions = if !args.base_versions.is_empty() {
        args.base_versions
            .clone()
            .iter()
            .filter(|s| !existing_versions.contains(&s))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        bail!("No base versions provided");
    };

    let include_wrappers = base_versions
        .iter()
        .map(|v| {
            println!("Generating include wrapper for version {}", v);
            let base_package_path = args.packages_dir.join(format!("simics-{}", v));
            (
                v,
                (
                    generate_include_wrapper(&base_package_path)
                        .expect("Couldn't generate wrapper"),
                    base_package_path,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    for (base_version, (wrapper, base_path)) in include_wrappers {
        let bindings_file = args
            .bindings_dir
            .join(format!("bindings-{}.rs", base_version));
        let simics_include_path = base_path.join("src").join("include");
        let simics_linux64_include_path = base_path.join("linux64").join("include");
        let mut python_include_versions = read_dir(&simics_linux64_include_path)?
            .filter_map(|de| de.ok())
            .map(|de| de.path())
            .filter_map(|p| {
                if p.is_dir() {
                    let dirname = p.components().last().unwrap();
                    let name = dirname.as_os_str().to_string_lossy().to_string();
                    if name.contains("python") {
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .filter_map(|p| Versioning::new(&p.replace("python", "")))
            .collect::<Vec<_>>();

        python_include_versions.sort();
        let simics_python_include_path = simics_linux64_include_path.join(format!(
            "python{}",
            python_include_versions
                .last()
                .ok_or_else(|| anyhow!("No python versions found"))?
        ));

        println!("Generating bindings file {} with simics include path {} and simics python include path {}", bindings_file.display(), simics_include_path.display(), simics_python_include_path.display());

        let header_path = args.packages_dir.join(format!("simics-{}.h", base_version));

        let mut wrapper_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&header_path)?;

        wrapper_file.write_all(wrapper.as_bytes())?;

        let header = header_path.as_os_str().to_string_lossy().to_string();

        let bindings = Builder::default()
            .clang_arg(format!("-I{}", simics_include_path.display()))
            .clang_arg(format!("-I{}", simics_python_include_path.display()))
            .clang_arg("-fretain-comments-from-system-headers")
            .clang_arg("-fparse-all-comments")
            // We don't care at all what warnings simics has if they aren't errors :)
            .clang_arg("-Wno-everything")
            .default_visibility(FieldVisibilityKind::Public)
            .derive_default(true)
            .derive_hash(true)
            .derive_partialord(true)
            .derive_ord(true)
            .derive_eq(true)
            .derive_partialeq(true)
            .generate_comments(true)
            .header(header)
            // NOTE: These callbacks are required to emit `cargo:rerun-if-changed`
            // statements, so we do not need to use them in this script. If you want to
            // repurpose this script to use in a `build.rs`, you should re-enable this
            // line:
            // .parse_callbacks(Box::new(CargoCallbacks))
            .parse_callbacks(Box::new(IgnoreMacros::new()))
            // These functions and types use (i|u)128 which isn't FFI-safe, we block them because the warnings
            // are not important and annoying to parse through
            .blocklist_function("qecvt")
            .blocklist_function("qfcvt")
            .blocklist_function("qgcvt")
            .blocklist_function("qecvt_r")
            .blocklist_function("qfcvt_r")
            .blocklist_function("qgcvt_r")
            .blocklist_function("strtold")
            .blocklist_function("__acoshl")
            .blocklist_function("acoshl")
            .blocklist_function("__acosl")
            .blocklist_function("acosl")
            .blocklist_function("__asinhl")
            .blocklist_function("asinhl")
            .blocklist_function("__asinl")
            .blocklist_function("asinl")
            .blocklist_function("__atan2l")
            .blocklist_function("atan2l")
            .blocklist_function("__atanhl")
            .blocklist_function("atanhl")
            .blocklist_function("__atanl")
            .blocklist_function("atanl")
            .blocklist_function("__cbrtl")
            .blocklist_function("cbrtl")
            .blocklist_function("__ceill")
            .blocklist_function("ceill")
            .blocklist_function("__copysignl")
            .blocklist_function("copysignl")
            .blocklist_function("__coshl")
            .blocklist_function("coshl")
            .blocklist_function("__cosl")
            .blocklist_function("cosl")
            .blocklist_function("__dreml")
            .blocklist_function("dreml")
            .blocklist_function("__erfcl")
            .blocklist_function("erfcl")
            .blocklist_function("__erfl")
            .blocklist_function("erfl")
            .blocklist_function("__exp2l")
            .blocklist_function("exp2l")
            .blocklist_function("__expl")
            .blocklist_function("expl")
            .blocklist_function("__expm1l")
            .blocklist_function("expm1l")
            .blocklist_function("__fabsl")
            .blocklist_function("fabsl")
            .blocklist_function("__fdiml")
            .blocklist_function("fdiml")
            .blocklist_function("__finitel")
            .blocklist_function("finitel")
            .blocklist_function("__floorl")
            .blocklist_function("floorl")
            .blocklist_function("__fmal")
            .blocklist_function("fmal")
            .blocklist_function("__fmaxl")
            .blocklist_function("fmaxl")
            .blocklist_function("__fminl")
            .blocklist_function("fminl")
            .blocklist_function("__fmodl")
            .blocklist_function("fmodl")
            .blocklist_function("__fpclassifyl")
            .blocklist_function("__frexpl")
            .blocklist_function("frexpl")
            .blocklist_function("__gammal")
            .blocklist_function("gammal")
            .blocklist_function("__hypotl")
            .blocklist_function("hypotl")
            .blocklist_function("__ilogbl")
            .blocklist_function("ilogbl")
            .blocklist_function("__iseqsigl")
            .blocklist_function("__isinfl")
            .blocklist_function("isinfl")
            .blocklist_function("__isnanl")
            .blocklist_function("isnanl")
            .blocklist_function("__issignalingl")
            .blocklist_function("__j0l")
            .blocklist_function("j0l")
            .blocklist_function("__j1l")
            .blocklist_function("j1l")
            .blocklist_function("__jnl")
            .blocklist_function("jnl")
            .blocklist_function("__ldexpl")
            .blocklist_function("ldexpl")
            .blocklist_function("__lgammal")
            .blocklist_function("lgammal")
            .blocklist_function("__lgammal_r")
            .blocklist_function("lgammal_r")
            .blocklist_function("__llrintl")
            .blocklist_function("llrintl")
            .blocklist_function("__llroundl")
            .blocklist_function("llroundl")
            .blocklist_function("__log10l")
            .blocklist_function("log10l")
            .blocklist_function("__log1pl")
            .blocklist_function("log1pl")
            .blocklist_function("__log2l")
            .blocklist_function("log2l")
            .blocklist_function("__logbl")
            .blocklist_function("logbl")
            .blocklist_function("__logl")
            .blocklist_function("logl")
            .blocklist_function("__lrintl")
            .blocklist_function("lrintl")
            .blocklist_function("__lroundl")
            .blocklist_function("lroundl")
            .blocklist_function("__modfl")
            .blocklist_function("modfl")
            .blocklist_function("__nanl")
            .blocklist_function("nanl")
            .blocklist_function("__nearbyintl")
            .blocklist_function("nearbyintl")
            .blocklist_function("__nextafterl")
            .blocklist_function("nextafterl")
            .blocklist_function("__nexttoward")
            .blocklist_function("nexttoward")
            .blocklist_function("__nexttowardf")
            .blocklist_function("nexttowardf")
            .blocklist_function("__nexttowardl")
            .blocklist_function("nexttowardl")
            .blocklist_function("__powl")
            .blocklist_function("powl")
            .blocklist_function("__remainderl")
            .blocklist_function("remainderl")
            .blocklist_function("__remquol")
            .blocklist_function("remquol")
            .blocklist_function("__rintl")
            .blocklist_function("rintl")
            .blocklist_function("__roundl")
            .blocklist_function("roundl")
            .blocklist_function("__scalbl")
            .blocklist_function("scalbl")
            .blocklist_function("__scalblnl")
            .blocklist_function("scalblnl")
            .blocklist_function("__scalbnl")
            .blocklist_function("scalbnl")
            .blocklist_function("__signbitl")
            .blocklist_function("__significandl")
            .blocklist_function("significandl")
            .blocklist_function("__sinhl")
            .blocklist_function("sinhl")
            .blocklist_function("__sinl")
            .blocklist_function("sinl")
            .blocklist_function("__sqrtl")
            .blocklist_function("sqrtl")
            .blocklist_function("__tanhl")
            .blocklist_function("tanhl")
            .blocklist_function("__tanl")
            .blocklist_function("tanl")
            .blocklist_function("__tgammal")
            .blocklist_function("tgammal")
            .blocklist_function("__truncl")
            .blocklist_function("truncl")
            .blocklist_function("wcstold")
            .blocklist_function("__y0l")
            .blocklist_function("y0l")
            .blocklist_function("__y1l")
            .blocklist_function("y1l")
            .blocklist_function("__ynl")
            .blocklist_function("ynl")
            .blocklist_item("M_E")
            .blocklist_item("M_LOG2E")
            .blocklist_item("M_LOG10E")
            .blocklist_item("M_LN2")
            .blocklist_item("M_LN10")
            .blocklist_item("M_PI")
            .blocklist_item("M_PI_2")
            .blocklist_item("M_PI_4")
            .blocklist_item("M_1_PI")
            .blocklist_item("M_2_PI")
            .blocklist_item("M_2_SQRTPI")
            .blocklist_item("M_SQRT2")
            .blocklist_item("M_SQRT1_2")
            .blocklist_item("Py_MATH_PIl")
            .blocklist_item("Py_MATH_PI")
            .blocklist_item("Py_MATH_El")
            .blocklist_item("Py_MATH_E")
            .blocklist_item("Py_MATH_TAU")
            // Blocklisted because the doc comments cause doc tests to fail
            .blocklist_function("_PyErr_TrySetFromCause")
            .generate()?;
        bindings.write_to_file(&bindings_file)?;
    }

    Ok(())
}

fn install_packages(args: &Args) -> Result<()> {
    let fake_simics_home = args.packages_dir.clone();

    if !fake_simics_home.exists() {
        create_dir_all(&fake_simics_home)?;
    }

    let base_versions = if !args.base_versions.is_empty() {
        args.base_versions.clone()
    } else {
        bail!("No base versions");
    };

    let existing_versions = get_existing_versions(args)?;
    let base_versions_needed = base_versions
        .iter()
        .filter(|v| {
            let version_dirname = format!("simics-{}", v);
            let version_dir = fake_simics_home.join(version_dirname);
            // Filter out versions we already downloaded *or* already have a binding file for
            // because we assume things won't get yanked and re-published with the same
            // version number
            !version_dir.is_dir() && !existing_versions.contains(&v)
        })
        .collect::<Vec<_>>();

    println!(
        "Installing packages to fake simics home at {}",
        fake_simics_home.display()
    );
    let ispm_dest = fake_simics_home.join("ispm");
    let ispm_tarball = fake_simics_home.join("ispm.tar.gz");

    if !ispm_dest.is_dir() {
        download_file(&args.ispm_tarball_url, &ispm_tarball)?;
        let tar = GzDecoder::new(BufReader::new(File::open(&ispm_tarball)?));
        let mut archive = Archive::new(tar);
        archive.unpack(&ispm_dest)?;
    }

    let ispm = read_dir(&ispm_dest)?
        .filter_map(|e| e.ok())
        .next()
        .ok_or_else(|| anyhow!("No entries in unpacked directory"))?
        .path()
        .join("ispm");

    if !base_versions_needed.is_empty() {
        let mut ispm_command = Command::new(ispm)
            .arg("install")
            .arg("--install-dir")
            .arg(&fake_simics_home)
            .arg("--package-repo")
            .arg(&args.simics_package_repo_url)
            .arg("-y")
            .args(
                base_versions_needed
                    .iter()
                    .rev()
                    .map(|v| format!("{}-{}", SIMICS_BASE_PACKAGE_NUMBER, v))
                    .collect::<Vec<_>>(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = ispm_command
            .stdout
            .take()
            .ok_or_else(|| anyhow!("No stdout to take"))?;
        let stderr = ispm_command
            .stderr
            .take()
            .ok_or_else(|| anyhow!("No stderr to take"))?;

        let stdout_reader = spawn(|| {
            let mut line = String::new();
            let mut out_reader = BufReader::new(stdout);
            loop {
                line.clear();
                let len = out_reader.read_line(&mut line);
                match len {
                    Ok(0) => {
                        break;
                    }
                    Err(e) => {
                        eprint!("Error reading from stdout: {}", e);
                    }
                    Ok(_) => {
                        let line = line.trim();
                        if !line.is_empty() {
                            println!("{}", line);
                        }
                    }
                }
            }
        });

        let stderr_reader = spawn(|| {
            let mut line = String::new();
            let mut out_reader = BufReader::new(stderr);
            loop {
                line.clear();
                let len = out_reader.read_line(&mut line);
                match len {
                    Ok(0) => {
                        break;
                    }
                    Err(e) => {
                        eprint!("Error reading from stderr: {}", e);
                    }
                    Ok(_) => {
                        let line = line.trim();
                        if !line.is_empty() {
                            println!("{}", line);
                        }
                    }
                }
            }
        });

        let status = ispm_command.wait()?;

        stdout_reader.join().expect("Could not join stdout reader");
        stderr_reader.join().expect("Could not join stderr reader");

        if !status.success() {
            bail!("Failed to run ispm command");
        }
    } else {
        println!("Skipping package install, all requested versions already exist");
    }
    Ok(())
}

fn generate_mod(args: &Args) -> Result<()> {
    let versions = get_existing_versions(args)?;

    let mut mod_text = r#"//! Raw bindings to the SIMICS API

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_cast)]
"#
    .to_string();

    for version in versions {
        mod_text += &format!(r#"#[cfg(feature = "{}")]"#, version);
        mod_text += "\n";
        mod_text += &format!(r#"include!("bindings-{}.rs");"#, version);
        mod_text += "\n";
    }

    let mut mod_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(args.bindings_dir.join("mod.rs"))?;

    mod_file.write_all(mod_text.as_bytes())?;

    Ok(())
}

fn update_cargo_toml(args: &Args) -> Result<()> {
    let versions = get_existing_versions(args)?;

    let mut doc = String::from_utf8_lossy(&read(&args.toml_file)?).parse::<Document>()?;

    doc["features"] = Item::Table(Table::new());

    for version in versions {
        doc["features"][version] = value(Array::default());
    }

    let updated_toml = doc.to_string();

    let mut toml_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&args.toml_file)?;

    toml_file.write_all(updated_toml.as_bytes())?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Download and install all the requested base versions into the packages directory

    install_packages(&args)?;
    /// Generate Rust bindings for all the downloaded versions
    generate_bindings(&args)?;
    // Generate a top-level mod.rs that includes the versioned bindings based on the set feature
    generate_mod(&args)?;
    // Add a feature to the Cargo.toml file for each version
    update_cargo_toml(&args)?;

    Ok(())
}
