// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, ensure, Result};
use common::{
    emit_link_info, SimicsBindings, VersionDeclaration, AUTO_BINDINGS_FILENAME,
    AUTO_BINDINGS_VERSION_FILENAME, OUT_DIR_ENV, SIMICS_BASE_ENV,
};
use ispm_wrapper::ispm::{self, GlobalOptions};
use std::{env::var, path::PathBuf};

/// Common utilities for the simics-api-sys build and update. Copy this mod directly into
/// `scripts/gen-simics-api-items.rs` on update.
pub mod common {
    use anyhow::{anyhow, bail, ensure, Result};
    use bindgen::{
        callbacks::{MacroParsingBehavior, ParseCallbacks},
        AliasVariation, Bindings, Builder, EnumVariation, FieldVisibilityKind, MacroTypeVariation,
        NonCopyUnionStyle,
    };
    use ispm_wrapper::ispm::{self, GlobalOptions};
    use scraper::{Html, Selector};
    use std::{
        collections::HashSet,
        env::var,
        ffi::OsStr,
        fmt::{self, Display, Formatter, Write},
        fs::{read, read_dir, write},
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
                                        println!(
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
                                    println!(
                                        "cargo:warning=Failed to canonicalize path {}: {}",
                                        p.path().display(),
                                        e
                                    );
                                    None
                                }
                            }
                        } else {
                            println!(
                                "cargo:warning=Ignoring path {}, no '.h' extension",
                                p.path().display()
                            );
                            None
                        }
                    }
                    None => {
                        println!(
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

            let include_paths_env = simics_base_directory
                .as_ref()
                .join("src")
                .join("include")
                .to_str()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("Could not convert path to string"))?;

            let include_path = PathBuf::from(&include_paths_env)
                .canonicalize()
                .map_err(|e| {
                    anyhow!(
                        "Include path from include path env {:?}: {}",
                        include_paths_env,
                        e
                    )
                })?;

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
            write(path.as_ref(), &self.code).map_err(|e| {
                anyhow!(
                    "Failed to write include wrapper to path {:?}: {}",
                    path.as_ref(),
                    e
                )
            })?;
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
            write(path.as_ref(), &self.0).map_err(|e| {
                anyhow!(
                    "Failed to write version declaration to path {:?}: {}",
                    path.as_ref(),
                    e
                )
            })?;
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

            let bindings = Builder::default()
                .clang_arg(
                    subdir(base_dir_path.as_ref().join(HOST_DIRNAME).join("include")).and_then(
                        |p| {
                            p.to_str()
                                .map(|s| format!("-I{}", s))
                                .ok_or_else(|| anyhow!("Could not convert path to string"))
                        },
                    )?,
                )
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
                // Blocklisted because approximate values of mathematical constants trigger
                // clippy
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
        #[cfg(unix)]
        const HOST_DIRNAME: &str = "linux64";

        #[cfg(not(unix))]
        const HOST_DIRNAME: &'static str = "win64";

        let base_dir_path = if let Ok(simics_base) = var(SIMICS_BASE_ENV) {
            PathBuf::from(simics_base)
        } else {
            println!("cargo:warning=No SIMICS_BASE environment variable found, using ispm to find installed packages and using latest base version");
            let mut packages = ispm::packages::list(&GlobalOptions::default())?;
            packages.sort();
            let Some(installed) = packages.installed_packages.as_ref() else {
                bail!("No SIMICS_BASE variable set and did not get any installed packages");
            };
            let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
                bail!(
                "No SIMICS_BASE variable set and did not find a package with package number 1000"
            );
            };
            println!("cargo:warning=Using Simics base version {}", base.version);
            base.paths
                .first()
                .ok_or_else(|| anyhow!("No paths found for package with package number 1000"))?
                .clone()
        };

        #[cfg(unix)]
        {
            // Link `libsimics-common.so`, `libvtutils.so`, and `libpythonX.XX.so.X.X` if they exist
            let bin_dir = base_dir_path
                .join(HOST_DIRNAME)
                .join("bin")
                .canonicalize()?;
            let libsimics_common = bin_dir.join("libsimics-common.so").canonicalize()?;

            let libvtutils = bin_dir.join("libvtutils.so").canonicalize()?;

            let sys_lib_dir = base_dir_path
                .join(HOST_DIRNAME)
                .join("sys")
                .join("lib")
                .canonicalize()?;

            let libpython = sys_lib_dir.join(
                read_dir(&sys_lib_dir)?
                    .filter_map(|p| p.ok())
                    .filter(|p| p.path().is_file())
                    .filter(|p| {
                        let path = p.path();

                        let Some(file_name) = path.file_name() else {
                            return false;
                        };

                        let Some(file_name) = file_name.to_str() else {
                            return false;
                        };

                        file_name.starts_with("libpython")
                            && file_name.contains(".so")
                            && file_name != "libpython3.so"
                    })
                    .map(|p| p.path())
                    .next()
                    .ok_or_else(|| {
                        anyhow!("No libpythonX.XX.so.X.X found in {}", sys_lib_dir.display())
                    })?,
            );

            println!(
                "cargo:rustc-link-lib=dylib:+verbatim={}",
                libsimics_common
                    .file_name()
                    .ok_or_else(|| anyhow!(
                        "No file name found for {}",
                        libsimics_common.display()
                    ))?
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?
            );
            println!(
                "cargo:rustc-link-lib=dylib:+verbatim={}",
                libvtutils
                    .file_name()
                    .ok_or_else(|| anyhow!("No file name found for {}", libvtutils.display()))?
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?
            );
            println!(
                "cargo:rustc-link-lib=dylib:+verbatim={}",
                libpython
                    .file_name()
                    .ok_or_else(|| anyhow!("No file name found for {}", libpython.display()))?
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?
            );
            println!(
                "cargo:rustc-link-search=native={}",
                bin_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?
            );
            println!(
                "cargo:rustc-link-search=native={}",
                sys_lib_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?
            );
            let ld_library_path = [
                bin_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?,
                sys_lib_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not convert path to string"))?,
            ]
            .join(":");

            println!("cargo:rustc-env=LD_LIBRARY_PATH={}", ld_library_path);
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
        fn blocklist_items(self, blocklist: &[&str]) -> Self;
        fn allowlist_items(self, allowlist: &[&str]) -> Self;
    }

    impl WithLists for Builder {
        fn blocklist_items(self, blocklist: &[&str]) -> Self {
            blocklist.iter().fold(self, |b, f| b.blocklist_item(f))
        }

        fn allowlist_items(self, allowlist: &[&str]) -> Self {
            allowlist.iter().fold(self, |b, f| b.allowlist_item(f))
        }
    }
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={SIMICS_BASE_ENV}");

    let base_dir_path = if let Ok(simics_base) = var(SIMICS_BASE_ENV) {
        PathBuf::from(simics_base)
    } else {
        println!("cargo:warning=No SIMICS_BASE environment variable found, using ispm to find installed packages and using latest base version");
        let mut packages = ispm::packages::list(&GlobalOptions::default())?;
        packages.sort();
        let Some(installed) = packages.installed_packages.as_ref() else {
            bail!("No SIMICS_BASE variable set and did not get any installed packages");
        };
        let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
            bail!(
                "No SIMICS_BASE variable set and did not find a package with package number 1000"
            );
        };
        println!("cargo:warning=Using Simics base version {}", base.version);
        base.paths
            .first()
            .ok_or_else(|| anyhow!("No paths found for package with package number 1000"))?
            .clone()
    };

    ensure!(
        base_dir_path.is_dir(),
        "{} is not a directory",
        base_dir_path.display()
    );

    let out_dir_path = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    let bindings_file_path = out_dir_path.join(AUTO_BINDINGS_FILENAME);
    let version_file_path = out_dir_path.join(AUTO_BINDINGS_VERSION_FILENAME);

    let version_declaration = VersionDeclaration::new(&base_dir_path).map_err(|e| {
        anyhow!(
            "Failed to create version declaration from path {:?}: {}",
            base_dir_path,
            e
        )
    })?;
    let mut allowlist = SIMICS_API_ITEMS.to_vec();

    // Explicitly allow hap definitions which we added ourselves
    allowlist.push(".*_HAP_NAME");
    allowlist.push(".*_hap_callback");

    let bindings = if var("SIMICS_BINDINGS_CLEAN").is_ok() {
        // If SIMICS_BINDINGS_CLEAN is set, we use the allowlist. This is set when publishing
        // so that documentation is tidy.
        SimicsBindings::new(&base_dir_path, &[], &allowlist)?
    } else {
        // If SIMICS_BINDINGS_CLEAN is not set, we do not use the allowlist. This improves
        // build times significantly.
        SimicsBindings::new(&base_dir_path, &[], &[])?
    };

    version_declaration.write(&version_file_path).map_err(|e| {
        anyhow!(
            "Failed to write version declaration to file {:?}: {}",
            version_file_path,
            e
        )
    })?;
    bindings.write(&bindings_file_path).map_err(|e| {
        anyhow!(
            "Failed to write bindings to file {:?}: {}",
            bindings_file_path,
            e
        )
    })?;

    if cfg!(feature = "link") {
        emit_link_info()?;
    }

    Ok(())
}

// NOTE: This constant is generated by running:
// ```
// ./scripts/gen-simics-api-items.rs \
//     -s ~/simics/simics-6.0.163 \
//     -s ~/simics/simics-6.0.164 \
//     -s ~/simics/simics-6.0.165 \
//     -s ~/simics/simics-6.0.166 \
//     -s ~/simics/simics-6.0.167 \
//     -s ~/simics/simics-6.0.168 \
//     -s ~/simics/simics-6.0.169 \
//     -s ~/simics/simics-6.0.170 \
//     -s ~/simics/simics-6.0.171 \
//     -s ~/simics/simics-6.0.172 \
//     -s ~/simics/simics-6.0.173 \
//     -s ~/simics/simics-6.0.174 \
//     -s ~/simics/simics-6.0.175 \
//     -s ~/simics/simics-6.0.176 \
//     -s ~/simics/simics-6.0.177 \
//     -s ~/simics/simics-6.0.178 \
//     -s ~/simics/simics-6.0.179 \
//     -s ~/simics/simics-6.0.180 \
//     -s ~/simics/simics-6.0.181 \
//     -s ~/simics/simics-6.0.182 \
//     -s ~/simics/simics-6.0.183 \
//     -s ~/simics/simics-6.0.184 \
//     -o simics-api-sys/simics_api_items.rs
// ```
//
// This list needs to be updated on each SIMICS Base version release
include!("simics_api_items.rs");
