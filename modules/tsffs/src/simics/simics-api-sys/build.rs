//! Build script for the low-level sys bindings crate to the SIMCS API implemented in:
//!
//! - libsimics-common.so
//! - libvtutils.so
//!
//! This build script requires the following environment variables to be set:
//!
//! - `SIMICS_BASE`
//! - `PYTHON3_INCLUDE`
//! - `INCLUDE_PATHS`
//! - `PYTHON3_LDFLAGS`
//! - `LDFLAGS`
//! - `LIBS`
//!
//! For test and development builds (e.g. under rust-analyzer), these values can be manually set
//! in the build environment, e.g. through the workspace's `.vscode/settings.json` file.
//!
//! Given a SIMICS installation directory /home/user/simics/ and a latest SIMICS base
//! version of 6.0.174, the variables would be set like so:
//!
//! - `SIMICS_BASE=/home/user/simics/simics-6.0.174`
//! - `PYTHON3_INCLUDE=-I/home/user/simics/simics-6.0.174/linux64/include/python3.9`
//! - `INCLUDE_PATHS=/home/user/simics/simics-6.0.174/src/include`
//! - `PYTHON3_LDFLAGS=/home/user/simics/simics-6.0.174/linux64/sys/lib/libpython3.so`
//!     - NOTE: This is *not* actually the shared object that needs to be linked against, we must
//!       link against the versioned shared object in the same directory e.g. libpython3.9.so.1.0.
//! - `LDFLAGS=-L/home/user/simics/simics-6.0.174/linux64/bin -z noexecstack -z relro -z now`
//! - `LIBS=-lsimics-common -lvtutils`

use anyhow::{anyhow, bail, ensure, Result};
use bindgen::{
    callbacks::{MacroParsingBehavior, ParseCallbacks},
    AliasVariation, Builder, EnumVariation, FieldVisibilityKind, MacroTypeVariation,
    NonCopyUnionStyle,
};
use scraper::{Html, Selector};
use std::{
    collections::HashSet,
    env::var,
    ffi::OsStr,
    fmt::Write,
    fs::{read, read_dir, write},
    iter::once,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// NOTE: The following environment variables are set under the SIMICS package make system
//
// PYTHON: The path to SIMICS's mini-python
// PYTHON3_LDFLAGS: The path to SIMICS's libpython3.so. Starting in 6.0.177, this variable can
// be empty, in which case we use a fallback.
// LIBS: `-lsimics-common -lvtutils`
// CXX_INCLUDE_PATHS: Path to SIMICS_BASE/linux64/api
// PYTHON3: The path to SIMICS's mini-python
// SIMICS_WORKSPACE: The path to the package root
// MOD_MAKEFILE: The makefile being invoked to build the module currently being built
// SIMICS_BASE: The path to the simics base version for the package/project
// DMLC: The path to dml python
// PYTHON3_INCLUDE: -I flag for the path to SIMICS's python3 include directory
// DML_INCLUDE_PATHS: The path to the dml api include directory
// SIMICS_MAJOR_VERSION: The simics major version
// SIMICS_PROJECT: The path to the simics project
// PYTHON_LDFLAGS: The path to SIMICS's libpython3.so
// MODULE_MAKEFILE: The path to the module.mk makefile in simics base
// SRC_BASE: The path to the modules directory for the package
// CCLDFLAGS_DYN: CC/LD flags for dynamic linking
// LDFLAGS: Linker flags to link to libsimics and friends
// SIMICS_PACKAGE_LIST: The path to the .package-list file for this package
// SIMICS_MODEL_BUILDER: The simics used to build models
// PYTHON_INCLUDE: -I flag for the path to SIMICS's python3 include directory
// DMLC_DIR: The path to the simics bin directory containing the dml compiler
// PY2TO3: Path to the py-2to3 tool
// LDFLAGS_PY3: -L flag to include simics base's bin directory
// INCLUDE_PATHS: The path to the SIMICS base include directory

/// Name for the environment variable set by the SIMICS build system to the path to the
/// simics base package. We actually use SIMICS_MODEL_BUILDER here, because we are indeed
/// building a model.
const SIMICS_BASE_ENV: &str = "SIMICS_BASE";
/// Name for the environment variable set by the SIMICS build system to the flag to
/// include e.g.  -I SIMICS_BASE/linux64/include/python3.9/
const PYTHON3_INCLUDE_ENV: &str = "PYTHON3_INCLUDE";
/// Name for the ldflags environment variable, which will point to
/// Name for the environment variable by the SIMICS build system to the path to the
/// simics include directory e.g.  SIMICS_BASE/src/include/
const INCLUDE_PATHS_ENV: &str = "INCLUDE_PATHS";

/// Name for the environment variable set by the SIMICS build system to the libpython3.so library
const PYTHON3_LDFLAGS_ENV: &str = "PYTHON3_LDFLAGS";
/// Name for the LDFLAGS environment variable set by the SIMICS build system containing
/// the link search path for the libsimics library, among other flags. e.g. -LPATH -z noexecstack
const LDFLAGS_ENV: &str = "LDFLAGS";
/// Name for the environment variable containing shared library link flags for simics common and
/// vtutils
const LIBS_ENV: &str = "LIBS";

/// Name for the environment variable set by cargo to the path to the OUT_DIR used for intermediate
/// build results
const OUT_DIR_ENV: &str = "OUT_DIR";
/// The name of the file generated when generating bindings in auto (package-time) mode.
const AUTO_BINDINGS_FILENAME: &str = "bindings-auto.rs";
/// The name of the file generated including the automatic simics version declaration when
/// generating bindings in auto (package-time) mode.
const AUTO_BINDINGS_VERSION_FILENAME: &str = "version-auto.rs";

#[cfg(not(windows))]
/// The name of the binary/library/object subdirectory on linux systems
const HOST_DIRNAME: &str = "linux64";
#[cfg(windows)]
/// The name of the binary/library/object subdirectory on windows systems
const HOST_DIRNAME: &str = "win64";

/// The path in SIMICS_BASE/HOST_TYPE/ of the HTML file containing HAP documentation required
/// for high level codegen of builtin HAPs
const HAP_DOC_PATH: &str = "doc/html/rm-base/rm-haps.html";

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

const HDR_DENYLIST: [&str; 9] = [
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

fn generate_include_wrapper<P, S>(simics_include_path: P, hap_code: S) -> Result<String>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let simics_include_path = simics_include_path.as_ref().to_path_buf().canonicalize()?;
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
                                        "cargo:warning=Failed to strip prefix {} from {}: {}",
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
                                    "cargo:warning=Failed to canonicalize path {}: {}",
                                    p.path().display(),
                                    e
                                );
                                None
                            }
                        }
                    } else {
                        eprintln!(
                            "cargo:warning=Ignoring path {}, no '.h' extension",
                            p.path().display()
                        );
                        None
                    }
                }
                None => {
                    eprintln!(
                        "cargo:warning=Ignoring path {}, no extension",
                        p.path().display()
                    );
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

    HDR_DENYLIST.iter().for_each(|le| {
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
    let wrapper = include_stmts.join("\n") + "\n" + hap_code.as_ref();

    println!("{wrapper}");

    Ok(wrapper)
}

/// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
/// exist, returns an error
fn subdir<P>(dir: P) -> Result<PathBuf>
where
    P: AsRef<Path>,
{
    let subdirs = read_dir(dir)?
        .filter_map(|p| p.ok())
        .map(|p| p.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    ensure!(
        subdirs.len() == 1,
        "Expected exactly 1 sub-directory, found {}",
        subdirs.len()
    );

    subdirs
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("No sub-directories found"))
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let base_dir_path = PathBuf::from(
        var(SIMICS_BASE_ENV)
            .map_err(|e| anyhow!("No environment variable {SIMICS_BASE_ENV} found: {e}"))?,
    );

    ensure!(
        base_dir_path.is_dir(),
        "{} is not a directory",
        base_dir_path.display()
    );

    let hap_doc_path = base_dir_path.join(HOST_DIRNAME).join(HAP_DOC_PATH);

    let hap_document =
        Html::parse_document(&String::from_utf8(read(&hap_doc_path).map_err(|e| {
            anyhow!(
                "Error reading document path {} to extract HAP definitions: {e}",
                hap_doc_path.display()
            )
        })?)?);

    let haps_selector = Selector::parse(r#"article"#).unwrap();
    let haps_id_selector = Selector::parse(r#"h2"#).unwrap();
    let section_selector = Selector::parse(r#"section"#).unwrap();
    let hap_code_selector = Selector::parse(r#"pre"#).unwrap();
    let hap_description_selector = Selector::parse(r#"h3"#).unwrap();
    let hap_index_selector = Selector::parse(r#"code"#).unwrap();

    let haps_article = hap_document.select(&haps_selector).next().unwrap();
    let haps_names = haps_article
        .select(&haps_id_selector)
        .filter_map(|h| h.value().id())
        .collect::<Vec<_>>();
    let haps_sections = haps_article.select(&section_selector).collect::<Vec<_>>();
    let haps_code_indices_descriptions = haps_sections
        .iter()
        .map(|s| {
            let code = s
                .select(&hap_code_selector)
                .next()
                .unwrap()
                .inner_html()
                .trim()
                .to_string();
            let maybe_index = s
                .select(&hap_index_selector)
                .next()
                .map(|i| i.inner_html().trim().to_string());
            let maybe_description = s
                .select(&hap_description_selector)
                .last()
                .and_then(|i| i.next_sibling())
                .and_then(|n| n.value().as_text().map(|t| t.trim().to_string()));
            (code, maybe_index, maybe_description)
        })
        .collect::<Vec<_>>();

    let hap_code = haps_names
        .iter()
        .zip(haps_code_indices_descriptions.iter())
        .try_fold(
            String::default(),
            |mut s, (name, (code, maybe_index, maybe_description))| {
                let mut hap_name_name = name.to_ascii_uppercase();
                hap_name_name += "_HAP_NAME";
                let mut hap_callback_name = name.to_ascii_lowercase();
                hap_callback_name += "_hap_callback";
                let code = code
                    .replace("(*)", &format!("(*{})", hap_callback_name))
                    .replace(['/', '-'], "_");
                let comment = format!(
                    "/**\n * Index: {}\n * Description: {}\n */",
                    maybe_index
                        .as_ref()
                        .unwrap_or(&"Indices not supported".to_string()),
                    maybe_description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                );

                write!(
                    &mut s,
                    "#define {} \"{}\"\n{}\ntypedef {}\n",
                    hap_name_name, name, comment, code
                )
                .and_then(|_| Ok(s))
            },
        )?;

    let simics_base_version = base_dir_path
                .file_name()
                .ok_or_else(|| anyhow!("No file name found in SIMICS base path"))?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert file name to string"))?
                .split('-')
                .last()
                .ok_or_else(|| anyhow!("Could not split to obtain version: SIMICS base directory may not be in the format simics-X.X.XXX"))?
                .to_string();

    let simics_base_version_const_declaration = format!(
        r#"pub const SIMICS_VERSION: &str = "{}";"#,
        simics_base_version
    );

    let out_dir_path = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    let bindings_file_path = out_dir_path.join(AUTO_BINDINGS_FILENAME);
    let version_file_path = out_dir_path.join(AUTO_BINDINGS_VERSION_FILENAME);

    write(version_file_path, simics_base_version_const_declaration)?;

    let include_paths_env = var(INCLUDE_PATHS_ENV).or_else(|e| {
            println!("cargo:warning=No environment variable {INCLUDE_PATHS_ENV} set. Using default include paths: {e}");
            base_dir_path
                .join("src")
                .join("include")
                .to_str()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("Could not convert path to string"))
        })?;

    let include_paths = PathBuf::from(&include_paths_env);

    let wrapper_contents = generate_include_wrapper(include_paths, hap_code)?;

    let bindings =
            Builder::default()
                .clang_arg(var(PYTHON3_INCLUDE_ENV).or_else(|e| {
                    println!("cargo:warning=No environment variable {PYTHON3_INCLUDE_ENV} set. Using default include paths: {e}");
                    subdir(base_dir_path
                        .join(HOST_DIRNAME)
                        .join("include"))
                        .and_then(|p| {
                            p.to_str()
                            .map(|s| format!("-I{}", s))
                            .ok_or_else(|| anyhow!("Could not convert path to string"))
                        })
                })?)
                .clang_arg(format!("-I{}", include_paths_env,))
                .clang_arg("-fretain-comments-from-system-headers")
                .clang_arg("-fparse-all-comments")
                // We don't care at all what warnings simics has if they aren't errors :)
                .clang_arg("-Wno-everything")
                .default_visibility(FieldVisibilityKind::Public)
                .default_alias_style(AliasVariation::TypeAlias)
                .default_enum_style(EnumVariation::Rust {
                    non_exhaustive: false,
                })
                .default_macro_constant_type(MacroTypeVariation::Unsigned)
                .default_non_copy_union_style(NonCopyUnionStyle::BindgenWrapper)
                .derive_default(true)
                .derive_hash(true)
                .derive_partialord(true)
                .derive_ord(true)
                .derive_eq(true)
                .derive_partialeq(true)
                .generate_comments(true)
                .header_contents("wrapper.h", &wrapper_contents)
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
                // Blocklisted because packed and align reprs differ
                .blocklist_type("__mingw_ldbl_type_t")
                .bitfield_enum("event_class_flag_t")
                .bitfield_enum("micro_checkpoint_flags_t")
                .bitfield_enum("access_t")
                .bitfield_enum("breakpoint_flag")
                .bitfield_enum("save_flags_t")
                .generate()?;

    bindings.write_to_file(bindings_file_path)?;

    if cfg!(feature = "link") {
        let base_dir_path = PathBuf::from(
            var(SIMICS_BASE_ENV)
                .map_err(|e| anyhow!("No environment variable {SIMICS_BASE_ENV} found: {e}"))?,
        );

        ensure!(
            base_dir_path.is_dir(),
            "{} is not a directory",
            base_dir_path.display()
        );

        let libpython_path =
            PathBuf::from(var(PYTHON3_LDFLAGS_ENV).map_err(|e| {
                anyhow!("No environment variable {PYTHON3_LDFLAGS_ENV} found: {e}")
            }).and_then(|v| if v.is_empty() { bail!("Environment variable {PYTHON3_LDFLAGS_ENV} is empty") } else { Ok(v) }).or_else(|e| {
                println!("cargo:warning=No environment variable {INCLUDE_PATHS_ENV} set. Using default include paths: {e}");
                base_dir_path
                    .join(HOST_DIRNAME)
                    .join("sys")
                    .join("lib")
                    .join("libpython3.so")
                    .to_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("Could not convert path to string"))
            })?);

        let libpython_dir = libpython_path
            .parent()
            .ok_or_else(|| anyhow!("libpython path {} has no parent", libpython_path.display()))?
            .to_path_buf();

        let link_search_paths = var(LDFLAGS_ENV)
            .or_else(|e| {
                println!("cargo:warning=No environment variable {LDFLAGS_ENV} set. Using default include paths: {e}");
                base_dir_path
                    .join(HOST_DIRNAME)
                    .join("bin")
                    .to_str()
                    .map(|s| format!("-L{}", s))
                    .ok_or_else(|| anyhow!("Could not convert path to string"))
            })?
            .split_whitespace()
            .filter_map(|s| s.starts_with("-L").then_some(s.replace("-L", "")))
            .map(PathBuf::from)
            .chain(once(libpython_dir))
            .collect::<Vec<_>>();

        #[cfg(not(windows))]
        let libs = var(LIBS_ENV)
            .unwrap_or_else(|e| {
                println!("cargo:warning=No environment variable {LIBS_ENV} set. Using default include paths: {e}");
                "-lsimics-common -lvtutils".to_string()
            })
            .split_whitespace()
            .filter_map(|s| s.starts_with("-l").then_some(s.replace("-l", "")))
            .collect::<Vec<_>>();

        #[cfg(windows)]
        let libs = "-lsimics-common -lvtutils"
            .split_whitespace()
            .filter_map(|s| s.starts_with("-l").then_some(s.replace("-l", "")))
            .collect::<Vec<_>>();

        link_search_paths.iter().for_each(|p| {
            // NOTE: These are removed by cargo, which is why we use absolute paths.
            // We emit them just for posterity
            println!("cargo:rustc-link-search=native={}", p.display());
            // NOTE: This is needed for the recursive linking step against libpython from
            // libsimics-common.so
            println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}", p.display());
            // NOTE: This is needed so we can locate the shared libraries at runtime, because
            // unlike the `simics` script in each project, we don't get to write an absolute
            // path
            // println!("cargo-rustc-link-arg=-Wl,-rpath,{}", p.display());
            // println!("cargo-rustc-cdylib-link-arg=-Wl,-rpath,{}", p.display());
            // println!("cargo-rustc-link-arg=-Wl,-rpath={}", p.display());
            // println!("cargo-rustc-cdylib-link-arg=-Wl,-rpath={}", p.display());
        });

        libs.iter()
            .for_each(|l| println!("cargo:rustc-link-lib=dylib={}", l));

        let library_search_paths = link_search_paths
            .iter()
            .map(|p| {
                p.to_str()
                    .ok_or_else(|| anyhow!("Could not convert path {} to string", p.display()))
            })
            .collect::<Result<Vec<_>>>()?
            .join(":");

        // NOTE: This enables running binaries linked with this one when running with `cargo run`
        println!("cargo:rustc-env=LD_LIBRARY_PATH={}", library_search_paths);

        // NOTE:
        // EVEN with all of the above, a binary built using `cargo build` will not be able to find
        // libsimics-common.so. Instead, when we build a binary that transitively depends on this
        // -sys crate, we compile it with `cargo rustc`, passing the `-rpath` link argument like
        // so. Note `--disable-new-dtags`, otherwise `libsimics-common.so` cannot find
        // `libpython3.9.so.1.0` because it will be missing the recursive rpath lookup.

        // SIMICS_BASE=/home/rhart/simics-public/simics-6.0.174
        // PYTHON3_INCLUDE=-I/home/rhart/simics-public/simics-6.0.174/linux64/include/python3.9
        // INCLUDE_PATHS=/home/rhart/simics-public/simics-6.0.174/src/include
        // PYTHON3_LDFLAGS=/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/libpython3.so
        // LDFLAGS="-L/home/rhart/simics-public/simics-6.0.174/linux64/bin -z
        // noexecstack -z relro -z now" LIBS="-lsimics-common -lvtutils" cargo rustc
        // --features=auto,link --example simple-simics -- -C
        // link-args="-Wl,--disable-new-dtags
        // -Wl,-rpath,/home/rhart/simics-public/simics-6.0.174/linux64/bin;/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/"

        // This command (the environment variables can be left out) can be auto-generated in the
        // SIMICS makefile build system.
    }

    Ok(())
}
