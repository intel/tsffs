use anyhow::{bail, Result};
use bindgen::{Builder, CargoCallbacks, callbacks::{MacroParsingBehavior, ParseCallbacks}};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use dotenvy_macro::dotenv;
use std::{
    collections::HashSet,
    env::var,
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

const SIMICS_HEADER_NAME: &str = "simics.h";
/// SIMICS_HOME must be provided containing a working SIMICS installation
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

/// Return the OUT_DIR build directory as a PathBuf
fn out_dir() -> Result<PathBuf> {
    match var("OUT_DIR") {
        Ok(out_dir) => Ok(PathBuf::from(out_dir)),
        Err(e) => Err(e.into()),
    }
}

/// Return the SIMICS_HOME directory as a PathBuf
fn simics_home() -> Result<PathBuf> {
    let simics_home = PathBuf::from(SIMICS_HOME);
    match simics_home.exists() {
        true => Ok(simics_home),
        false => {
            bail!(
                "SIMICS_HOME is defined, but {} does not exist.",
                SIMICS_HOME
            )
        }
    }
}

/// Set up the SIMICS simulator project
///
/// Expects SIMICS_HOME to be set, and errors if it is not. We can ostensibly download a fresh
/// copy of simics, but it's quite large (2G so this should be avoided).
fn setup_simics() -> Result<()> {
    let confuse_simics_project_dir = out_dir()?.join("simics");
    let latest_simics_manifest = simics_latest(simics_home()?)?;
    let simics_base_dir = simics_home()?.join(format!(
        "simics-{}",
        latest_simics_manifest.packages[&PackageNumber::Base].version
    ));
    let simics_qsp_x86_dir = simics_home()?.join(format!(
        "simics-qsp-x86-{}",
        latest_simics_manifest.packages[&PackageNumber::QuickStartPlatform].version
    ));

    assert!(
        simics_base_dir.exists(),
        "Simics base directory does not exist. Is install broken?"
    );
    assert!(
        simics_qsp_x86_dir.exists(),
        "Simics QSP directory does not exist. Is install broken?"
    );

    let simics_base_project_setup = simics_base_dir.join("bin").join("project-setup");

    assert!(
        simics_base_project_setup.exists(),
        "Simics project-setup tool not found."
    );

    Command::new(simics_base_project_setup)
        .arg("--ignore-existing-files")
        .arg(&confuse_simics_project_dir)
        .output()?;

    let _simics_project_project_setup =
        confuse_simics_project_dir.join("bin").join("project-setup");

    Ok(())
}

fn generate_simics_include_wrapper() -> Result<String> {
    let simics_include_path = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("src")
        .join("include");
    let mut include_paths = WalkDir::new(&simics_include_path)
        .into_iter()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let simics_include_path = &simics_include_path;
            match p.path().extension() {
                Some(e) => {
                    if e == "h" {
                        match p.path().canonicalize() {
                            Ok(p) => p
                                .strip_prefix(&simics_include_path)
                                .map_or_else(|_| None::<PathBuf>, |p| Some(p.to_path_buf())),
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                }
                None => None,
            }
        })
        .collect::<Vec<_>>();

    let python_hdr_pos = include_paths
        .iter()
        .position(|p| p.file_name() == Some(OsStr::new("python-header.h")))
        .expect("No header python-header.h found.");

    // We need to move python-header.h to the beginning of the list
    include_paths.swap(0, python_hdr_pos);

    let hdr_denylist = vec![
        // Most of these are denylisted because they include follower-time.h and it's :/ broken
        "global.h",
        "vtutils.h",
        "libfollower.h",
        "follower-time.h",
        "follower.h",
        "link-endpoint.h",
        "data-structs.h",
    ];

    hdr_denylist.iter().for_each(|le| {
        let pos = include_paths
            .iter()
            .position(|p| p.file_name() == Some(OsStr::new(le)))
            .expect(&format!("No header '{}' found.", le));
        include_paths.remove(pos);
    });

    let include_stmts = include_paths
        .iter()
        .map(|p| format!("#include <{}>", p.display()))
        .collect::<Vec<_>>();

    Ok(include_stmts.join("\n") + "\n")
}

fn write_simics_include_wrapper() -> Result<PathBuf> {
    let wrapper_path = out_dir()?.join(SIMICS_HEADER_NAME);
    let mut wrapper = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&wrapper_path)?;
    wrapper.write_all(generate_simics_include_wrapper()?.as_bytes())?;
    Ok(wrapper_path)
}

// https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-1312298570
const IGNORE_MACROS
: [&str; 20] = [
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
        Self(IGNORE_MACROS
            .into_iter().map(|s| s.to_owned()).collect())
    }
}

fn generate_simics_api<P: AsRef<Path>>(header: P) -> Result<()> {
    let simics_include_path = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("src")
        .join("include");
    let simics_python_include_path = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("include")
        // TODO: Do we need to make this dynamic?
        .join("python3.9");
    let header_path = header.as_ref().as_os_str().to_string_lossy().to_string();
    let simics_bindings_path = out_dir()?.join("simics_bindings.rs");
    let bindings = Builder::default()
        .clang_arg(format!("-I{}", simics_include_path.display()))
        .clang_arg(format!("-I{}", simics_python_include_path.display()))
        .header(header_path)
        .parse_callbacks(Box::new(CargoCallbacks))
        .parse_callbacks(Box::new(IgnoreMacros::new()))
        .blocklist_function("strtold")
        .generate()?;
    bindings.write_to_file(&simics_bindings_path)?;
    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    setup_simics()?;
    let wrapper_path = write_simics_include_wrapper()?;
    generate_simics_api(&wrapper_path)?;
    Ok(())
}
