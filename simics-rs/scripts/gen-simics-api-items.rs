#!/usr/bin/env -S cargo +nightly -Z script
## [dependencies]
## anyhow = "*"
## bindgen = "*"
## clap = { version = "*", features = ["derive"] }
## futures = "*"
## prettyplease = "*"
## quote = "*"
## scraper = "*"
## syn = { version = "*", features = ["full"] }
## tokio = { version = "*", features = ["full"] }
## typed-builder = "*"
## walkdir = "*"

// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use futures::{stream::iter, StreamExt};
use prettyplease::unparse;
use common::SimicsBindings;
use std::{collections::HashSet, path::{Path, PathBuf}};
use syn::{parse_file, Item, ForeignItem};
use quote::quote;
use tokio::{process::Command, fs::write};
use typed_builder::TypedBuilder;

/// Common utilities for the simics-api-sys build and update. Copy this mod directly into
/// `scripts/gen-simics-api-items.rs` on update.
pub mod common {
    use anyhow::{anyhow, bail, ensure, Result};
    use bindgen::{
        callbacks::{MacroParsingBehavior, ParseCallbacks},
        AliasVariation, Bindings, Builder, EnumVariation, FieldVisibilityKind, MacroTypeVariation,
        NonCopyUnionStyle,
    };
    use scraper::{Html, Selector};
    use std::{
        collections::HashSet,
        env::var,
        ffi::OsStr,
        fmt::{self, Display, Formatter, Write},
        fs::{read, read_dir, write},
        iter::once,
        path::{Path, PathBuf},
    };
    use walkdir::WalkDir;

    pub const AUTO_BINDINGS_FILENAME: &str = "bindings-auto.rs";
    pub const AUTO_BINDINGS_VERSION_FILENAME: &str = "version-auto.rs";
    pub const OUT_DIR_ENV: &str = "OUT_DIR";

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
    pub const SIMICS_BASE_ENV: &str = "SIMICS_BASE";
    /// Name for the environment variable set by the SIMICS build system to the flag to
    /// include e.g.  -I SIMICS_BASE/linux64/include/python3.9/
    pub const PYTHON3_INCLUDE_ENV: &str = "PYTHON3_INCLUDE";
    /// Name for the ldflags environment variable, which will point to
    /// Name for the environment variable by the SIMICS build system to the path to the
    /// simics include directory e.g.  SIMICS_BASE/src/include/
    pub const INCLUDE_PATHS_ENV: &str = "INCLUDE_PATHS";

    /// Name for the environment variable set by the SIMICS build system to the libpython3.so library
    pub const PYTHON3_LDFLAGS_ENV: &str = "PYTHON3_LDFLAGS";
    /// Name for the LDFLAGS environment variable set by the SIMICS build system containing
    /// the link search path for the libsimics library, among other flags. e.g. -LPATH -z noexecstack
    pub const LDFLAGS_ENV: &str = "LDFLAGS";
    /// Name for the environment variable containing shared library link flags for simics common and
    /// vtutils
    pub const LIBS_ENV: &str = "LIBS";

    #[cfg(not(windows))]
    /// The name of the binary/library/object subdirectory on linux systems
    pub const HOST_DIRNAME: &str = "linux64";
    #[cfg(windows)]
    /// The name of the binary/library/object subdirectory on windows systems
    pub const HOST_DIRNAME: &str = "win64";

    /// The path in SIMICS_BASE/HOST_TYPE/ of the HTML file containing HAP documentation required
    /// for high level codegen of builtin HAPs
    pub const HAP_DOC_PATH: &str = "doc/html/rm-base/rm-haps.html";

    // https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-1312298570
    pub const IGNORE_MACROS: [&str; 20] = [
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

    pub const HDR_BLOCKLIST: [&str; 4] = [
        // Not a SIMICS library include, and conflicts with simics/simulator/follower-time.h
        "libfollower.h",
        // Deprecated (see #pragma messages in files)
        "data-structs.h",
        "global.h",
        "vtutils.h",
        // Excluded (host configuration dependent)
        // "host-info.h",
        // "module-host-config.h",
        // "base-types.h",
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

    pub struct IncludeWrapper {
        include_path: PathBuf,
        code: String,
    }

    impl IncludeWrapper {
        /// Generate include wrapper code from the include directory. This generates a top-level
        /// file which includes all the headers in the include directory, which is passed to bindgen
        /// which will expand all the includes.
        pub fn include_code_from_includes<P>(include_path: P) -> Result<String>
        where
            P: AsRef<Path>,
        {
            let mut include_paths = WalkDir::new(&include_path)
                .into_iter()
                .filter_map(|p| p.ok())
                .filter(|p| p.path().is_file())
                .filter_map(|p| match p.path().extension() {
                    Some(e) => {
                        if e == "h" {
                            match p.path().canonicalize() {
                                Ok(p) => p.strip_prefix(&include_path).map_or_else(
                                    |e| {
                                        eprintln!(
                                            "cargo:warning=Failed to strip prefix {} from {}: {}",
                                            include_path.as_ref().display(),
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
                })
                .collect::<Vec<_>>();

            let Some(python_hdr_pos) = include_paths
                .iter()
                .position(|p| p.file_name() == Some(OsStr::new("python-header.h")))
            else {
                bail!("No python-header.h in include file list.");
            };

            include_paths.swap(0, python_hdr_pos);

            HDR_BLOCKLIST.iter().for_each(|le| {
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

            let code = include_stmts.join("\n") + "\n";

            Ok(code)
        }

        /// Generate code for HAPs from the hap documentation file. HAPs are not included in
        /// the include directory like interfaces (the other code-generated component) are,
        /// so we must generate them separately. This is very much a best-effort approach, as it
        /// relies on parsing the HTML documentation.
        pub fn hap_code_from_doc<P>(hap_doc_path: P) -> Result<String>
        where
            P: AsRef<Path>,
        {
            let hap_document =
                Html::parse_document(&String::from_utf8(read(&hap_doc_path).map_err(|e| {
                    anyhow!(
                        "Error reading document path {} to extract HAP definitions: {e}",
                        hap_doc_path.as_ref().display()
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
                            hap_name_name, name, comment, code,
                        )
                        .and_then(|_| Ok(s))
                    },
                )?;

            Ok(hap_code)
        }

        /// Generate a new include wrapper for the given SIMICS base directory
        ///
        pub fn new<P>(simics_base_directory: P) -> Result<Self>
        where
            P: AsRef<Path>,
        {
            let hap_doc_path = simics_base_directory
                .as_ref()
                .join(HOST_DIRNAME)
                .join(HAP_DOC_PATH);

            let include_paths_env = var(INCLUDE_PATHS_ENV).or_else(|e| {
                println!("cargo:warning=No environment variable {INCLUDE_PATHS_ENV} set. Using default include paths: {e}");
                simics_base_directory.as_ref()
                    .join("src")
                    .join("include")
                    .to_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("Could not convert path to string"))
            })?;

            let include_path = PathBuf::from(&include_paths_env).canonicalize()?;

            let include_code = Self::include_code_from_includes(&include_path)?;
            let hap_code = Self::hap_code_from_doc(hap_doc_path)?;
            Ok(Self {
                code: include_code + &hap_code,
                include_path,
            })
        }

        /// Write the generated include wrapper to the given path
        pub fn write<P>(&self, path: P) -> Result<()>
        where
            P: AsRef<Path>,
        {
            write(path, &self.code)?;
            Ok(())
        }
    }

    impl Display for IncludeWrapper {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.code)
        }
    }

    pub struct VersionDeclaration(String);

    impl VersionDeclaration {
        pub fn new<P>(simics_base_directory: P) -> Result<Self>
        where
            P: AsRef<Path>,
        {
            let simics_base_version = simics_base_directory
            .as_ref()
            .file_name()
            .ok_or_else(|| anyhow!("No file name found in SIMICS base path"))?
            .to_str()
            .ok_or_else(|| anyhow!("Could not convert file name to string"))?
            .split('-')
            .last()
            .ok_or_else(|| anyhow!("Could not split to obtain version: SIMICS base directory may not be in the format simics-X.X.XXX"))?
            .to_string();

            Ok(Self(format!(
                r#"pub const SIMICS_VERSION: &str = "{}";"#,
                simics_base_version
            )))
        }

        pub fn write<P>(&self, path: P) -> Result<()>
        where
            P: AsRef<Path>,
        {
            write(path, &self.0)?;
            Ok(())
        }
    }

    pub struct SimicsBindings {
        pub bindings: Bindings,
    }

    impl SimicsBindings {
        pub fn new<P>(base_dir_path: P, blocklist: &[&str], allowlist: &[&str]) -> Result<Self>
        where
            P: AsRef<Path>,
        {
            let wrapper = IncludeWrapper::new(&base_dir_path)?;

            #[cfg(unix)]
            let wrapper_include_path = wrapper.include_path.display().to_string();

            #[cfg(windows)]
            let wrapper_include_path = {
                let path = wrapper.include_path.display().to_string();
                if path.starts_with(r#"\\?\"#) {
                    path[4..].to_string()
                } else {
                    path
                }
            };

            let bindings =
                Builder::default()
                    .clang_arg(var(PYTHON3_INCLUDE_ENV).or_else(|e| {
                        println!("cargo:warning=No environment variable {PYTHON3_INCLUDE_ENV} set. Using default include paths: {e}");
                        subdir(base_dir_path
                            .as_ref()
                            .join(HOST_DIRNAME)
                            .join("include"))
                            .and_then(|p| {
                                p.to_str()
                                .map(|s| format!("-I{}", s))
                                .ok_or_else(|| anyhow!("Could not convert path to string"))
                            })
                    })?)
                    .clang_arg(format!("-I{}", &wrapper_include_path))
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
                    .header_contents("wrapper.h", &wrapper.to_string())
                    // NOTE: These callbacks are required to emit `cargo:rerun-if-changed`
                    // statements, so we do not need to use them in this script. If you want to
                    // repurpose this script to use in a `build.rs`, you should re-enable this
                    // line:
                    // .parse_callbacks(Box::new(CargoCallbacks))
                    .parse_callbacks(Box::new(IgnoreMacros::new()))
                    // These functions are extraneous and only emitted due to the include wrappers
                    // eventually including system headers which provide a huge number of unused
                    // symbols.
                    // .blocklist_functions(FUNCS_BLOCKLIST)
                    .blocklist_items(blocklist)
                    .allowlist_items(allowlist)
                    // .blocklist_items(ITEMS_BLOCKLIST_128BIT)
                    // Blocklisted because packed and align reprs differ
                    .blocklist_type("__mingw_ldbl_type_t")
                    .bitfield_enum("event_class_flag_t")
                    .bitfield_enum("micro_checkpoint_flags_t")
                    .bitfield_enum("access_t")
                    .bitfield_enum("breakpoint_flag")
                    .bitfield_enum("save_flags_t")
                    // Blocklisted because use 128-bit types which are not FFI-safe
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
                    .blocklist_function("strtold")
                    .blocklist_function("qecvt")
                    .blocklist_function("qfcvt")
                    .blocklist_function("qgcvt")
                    .blocklist_function("qecvt_r")
                    .blocklist_function("qfcvt_r")
                    // Blocklisted because use 128-bit types which are not FFI-safe
                    // (Windows)
                    .blocklist_function("__mingw_strtold")
                    .blocklist_function("__mingw_wcstold")
                    .blocklist_function("sincosl")
                    .blocklist_function("_chgsignl")
                    // Blocklisted because not used and adds confusion
                    .generate()
                    .map_err(|e| {
                        println!("cargo:warning=Failed to generate bindings: {e}");
                        println!("cargo:warning=Include path: {}", &wrapper_include_path);
                        let wrapper_string = wrapper.to_string();
                        for (i, line) in wrapper_string.lines().enumerate() {
                            println!("cargo:warning={:04}: {}", i, line);
                        }
                        e
                    })?;

            Ok(Self { bindings })
        }

        pub fn write<P>(&self, path: P) -> Result<()>
        where
            P: AsRef<Path>,
        {
            self.bindings.write_to_file(path)?;
            Ok(())
        }
    }

    impl Display for SimicsBindings {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", &self.bindings.to_string())
        }
    }

    /// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
    /// exist, returns an error
    pub fn subdir<P>(dir: P) -> Result<PathBuf>
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

    pub fn emit_link_info() -> Result<()> {
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
            // NOTE: This is ostensibly needed so we can locate the shared libraries at
            // runtime, because unlike the `simics` script in each project, we don't get to
            // write an absolute path. Unfortunately, rpath is blocklisted.
            //
            // println!("cargo-rustc-link-arg=-Wl,-rpath,{}", p.display());
            // println!("cargo-rustc-cdylib-link-arg=-Wl,-rpath,{}", p.display());
            // println!("cargo-rustc-link-arg=-Wl,-rpath={}", p.display());
            // println!("cargo-rustc-cdylib-link-arg=-Wl,-rpath={}", p.display());
        });

        libs.iter()
            .for_each(|l| println!("cargo:rustc-link-lib=dylib={}", l));

        #[cfg(unix)]
        {
            let library_search_paths = link_search_paths
                .iter()
                .map(|p| {
                    p.to_str()
                        .ok_or_else(|| anyhow!("Could not convert path {} to string", p.display()))
                })
                .collect::<Result<Vec<_>>>()?
                .join(":");

            // NOTE: This enables running binaries linked with this one when running with `cargo run`
            println!(
                "cargo:warning=Setting LD_LIBRARY_PATH={}",
                library_search_paths
            );

            println!("cargo:rustc-env=LD_LIBRARY_PATH={}", library_search_paths);
        }

        // NOTE: EVEN with all of the above, a binary built using `cargo build` will not
        // be able to find libsimics-common.so. Instead, when we build a binary that
        // transitively depends on this -sys crate, we compile it with `cargo rustc`,
        // passing the `-rpath` link argument like so. Note `--disable-new-dtags`,
        // otherwise `libsimics-common.so` cannot find `libpython3.9.so.1.0` because it
        // will be missing the recursive rpath lookup.

        // SIMICS_BASE=/home/rhart/simics-public/simics-6.0.174
        // PYTHON3_INCLUDE=-I/home/rhart/simics-public/simics-6.0.174/linux64/include/python3.9
        // INCLUDE_PATHS=/home/rhart/simics-public/simics-6.0.174/src/include
        // PYTHON3_LDFLAGS=/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/libpython3.so
        // LDFLAGS="-L/home/rhart/simics-public/simics-6.0.174/linux64/bin -z noexecstack -z relro -z now" LIBS="-lsimics-common -lvtutils"
        // cargo --features=auto,link --example simple-simics -- -C link-args="-Wl,--disable-new-dtags -Wl,-rpath,/home/rhart/simics-public/simics-6.0.174/linux64/bin;/home/rhart/simics-public/simics-6.0.174/linux64/sys/lib/"
        //
        // This command (the environment variables can be left out) can be
        // auto-generated in the SIMICS makefile build system.

        Ok(())
    }

    trait WithLists {
        fn blocklist_functions(self, blocklist: &[&str]) -> Self;
        fn blocklist_items(self, blocklist: &[&str]) -> Self;
        fn blocklist_types(self, blocklist: &[&str]) -> Self;
        fn allowlist_functions(self, allowlist: &[&str]) -> Self;
        fn allowlist_items(self, allowlist: &[&str]) -> Self;
        fn allowlist_types(self, allowlist: &[&str]) -> Self;
    }

    impl WithLists for Builder {
        fn blocklist_functions(self, blocklist: &[&str]) -> Self {
            blocklist.iter().fold(self, |b, f| b.blocklist_function(f))
        }

        fn blocklist_items(self, blocklist: &[&str]) -> Self {
            blocklist.iter().fold(self, |b, f| b.blocklist_item(f))
        }

        fn blocklist_types(self, blocklist: &[&str]) -> Self {
            blocklist.iter().fold(self, |b, f| b.blocklist_type(f))
        }

        fn allowlist_functions(self, allowlist: &[&str]) -> Self {
            allowlist.iter().fold(self, |b, f| b.allowlist_function(f))
        }

        fn allowlist_items(self, allowlist: &[&str]) -> Self {
            allowlist.iter().fold(self, |b, f| b.allowlist_item(f))
        }

        fn allowlist_types(self, allowlist: &[&str]) -> Self {
            allowlist.iter().fold(self, |b, f| b.allowlist_type(f))
        }
    }
}


#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    /// The path to a SIMICS base directory (e.g. /opt/simics/simics-6.0.169)
    simics_base: Vec<PathBuf>,
    #[clap(short, long)]
    /// The path to write the generated bindings to
    output: PathBuf,
}

#[derive(TypedBuilder, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParseItem {
    pub name: String,
    #[builder(default)]
    pub is_fn: bool,
}

impl ParseItem {
    /// Return a regex for searching for this item. For functions, the pattern is
    /// searched with an open paren after the name to restrict the search to function
    /// calls and declarations.
    fn search_regex(&self) -> String {
        if self.is_fn {
            // NOTE: Some APIs are declared like `void *SIM_...` so we allow an optional
            // asterisk after a space
            format!(r#"(^{name}\(|\s+\*?{name}\()"#, name = self.name)
        } else {
            format!(r#"(^{name}|\s+{name})"#, name = self.name)
        }
    }

    pub async fn search<P>(&self, path: P) -> Option<String>
    where
        P: AsRef<Path>,
    {
        let name = self.name.clone();

        Command::new("rg")
            .arg("-q")
            .arg(&self.search_regex())
            .arg(path.as_ref())
            .status()
            .await
            .expect("Failed to execute rg. Install it with 'cargo install ripgrep'")
            .success()
            .then_some(name)
    }
}

/// Parse an item into a list of names
fn parse_item(item: &Item) -> Vec<ParseItem> {
    match item {
        Item::Const(c) => vec![
            ParseItem::builder().name(c.ident.to_string()).build()
        ],
        Item::Enum(e) => vec![
            ParseItem::builder().name(e.ident.to_string()).build()
        ],
        // ExternCrate(ItemExternCrate),
        Item::Fn(f) => vec![
            ParseItem::builder().name(f.sig.ident.to_string()).is_fn(true).build()
        ],
        Item::ForeignMod(m) => {
            m.items.iter()
                .filter_map(|item| match item {
                    ForeignItem::Fn(f) => Some(
                        ParseItem::builder()
                            .name(f.sig.ident.to_string())
                            .is_fn(true)
                            .build()
                    ),
                    ForeignItem::Static(s) => Some(
                        ParseItem::builder()
                            .name(s.ident.to_string())
                            .build()
                    ),
                    ForeignItem::Type(t) => Some(
                        ParseItem::builder()
                            .name(t.ident.to_string())
                            .build()
                    ),
                    // ForeignItem::Macro(ItemMacro),
                    // ForeignItem::Verbatim(TokenStream),
                    _ => None,
                })
                .collect()
        },
        // Impl(ItemImpl),
        // Macro(ItemMacro),
        // Mod(ItemMod),
        Item::Static(s) => vec![
            ParseItem::builder().name(s.ident.to_string()).build()
        ],
        Item::Struct(s) => vec![
            ParseItem::builder().name(s.ident.to_string()).build()
        ],
        // Trait(ItemTrait),
        // TraitAlias(ItemTraitAlias),
        Item::Type(t) => vec![
            ParseItem::builder().name(t.ident.to_string()).build()
        ],
        Item::Union(u) => vec![
            ParseItem::builder().name(u.ident.to_string()).build()
        ],
        // Use(ItemUse),
        // Verbatim(TokenStream),
        _ => vec![],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let parse_names = iter(args.simics_base)
        .map(|simics_base| async move {
            let bindings = SimicsBindings::new(&simics_base, &[], &[])
                .expect("Failed to generate bindings");

            let parsed = parse_file(&bindings.to_string())
                .expect("Failed to parse bindings");

            let parse_items = parsed.items.iter()
                .map(parse_item)
                .flatten()
                .collect::<Vec<_>>();

            // Asynchronously check if the name actually occurs in the base directory
            iter(parse_items)
                .filter_map(|parse_item| {
                    let include_dir = simics_base
                        .join("src")
                        .join("include");
                    async move { parse_item.search(&include_dir).await }
                })
                .collect::<Vec<_>>()
                .await
        })
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await;

    let mut parse_names = parse_names
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    parse_names.sort();

    let parse_names_len = parse_names.len();

    let simics_api_items = quote! {
        const SIMICS_API_ITEMS: &[&str; #parse_names_len] = &[
            #(#parse_names),*
        ];
    };

    let formatted = unparse(&parse_file(&simics_api_items.to_string())?);

    write(&args.output, &formatted).await?;

    Ok(())
}
