use anyhow::{bail, Result};
use bindgen::{
    callbacks::{MacroParsingBehavior, ParseCallbacks},
    Builder, CargoCallbacks,
};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use dotenvy_macro::dotenv;
use std::{
    collections::HashSet,
    env::var,
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
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

    include_paths
        .iter()
        .position(|p| p.file_name() == Some(OsStr::new("attr-value.h")))
        .expect("No header attr-value.h found");

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
        Self(IGNORE_MACROS.into_iter().map(|s| s.to_owned()).collect())
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
        .clang_arg("-fretain-comments-from-system-headers")
        .clang_arg("-fparse-all-comments")
        .generate_comments(true)
        .header(header_path)
        .parse_callbacks(Box::new(CargoCallbacks))
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
        .generate()?;
    bindings.write_to_file(&simics_bindings_path)?;
    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    let wrapper_path = write_simics_include_wrapper()?;
    generate_simics_api(&wrapper_path)?;
    Ok(())
}
