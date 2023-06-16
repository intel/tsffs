//! Standalone simics linking functionality. This copies from the `simics` crate and should be updated
//! if the linking strategy there changes.

extern crate num_traits;
#[macro_use]
extern crate num_derive;

use anyhow::{bail, ensure, Context, Result};
use dotenvy_macro::dotenv;
use itertools::Itertools;
use num::{FromPrimitive, ToPrimitive};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use version_tools::VersionConstraint;
use versions::Versioning;
use walkdir::WalkDir;

type PackageVersion = String;
type PackageNumber = i64;

#[derive(Hash, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, FromPrimitive, ToPrimitive)]
#[repr(i64)]
/// Numbers for public SIMICS packages. These numbers can be used to conveniently specify package
/// numbers
enum PublicPackageNumber {
    QspClearLinux = 4094,
    QspCpu = 8112,
    QspIsim = 8144,
    DoceaBase = 7801,
    OssSources = 1020,
    Training = 6010,
    Viewer = 8126,
    QspX86 = 2096,
    Base = 1000,
    Error = -1,
}

impl From<i64> for PublicPackageNumber {
    fn from(value: i64) -> Self {
        FromPrimitive::from_i64(value).unwrap_or(PublicPackageNumber::Error)
    }
}

impl From<PublicPackageNumber> for i64 {
    fn from(val: PublicPackageNumber) -> Self {
        ToPrimitive::to_i64(&val).expect("Invalid conversion to i64")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Information about a package. This package info is located in the packageinfo subdirectory of
/// a simics package, for example SIMICS_HOME/simics-6.0.157/packageinfo/Simics-Base-linux64
/// and is not *quite* YAML but is close.
struct PackageInfo {
    /// The package name
    pub name: String,
    /// The package description
    pub description: String,
    /// The version string for the package
    pub version: String,
    #[serde(rename = "extra-version")]
    /// The extra version string for the package, usually blank
    pub extra_version: String,
    //// Host type, e.g. `linux64`
    pub host: String,
    /// Whether the package is public or private
    pub confidentiality: String,
    #[serde(rename = "package-name")]
    /// The name of the package, again (this field is typically the same as `name`)
    pub package_name: String,
    #[serde(rename = "package-number")]
    /// The package number
    pub package_number: PackageNumber,
    #[serde(rename = "build-id")]
    /// A monotonically increasing build ID for the package number
    pub build_id: u64,
    #[serde(rename = "build-id-namespace")]
    /// Namespace for build IDs, `simics` for public/official packages
    pub build_id_namespace: String,
    #[serde(rename = "type")]
    /// The type of package, typically either `base` or `addon`
    pub typ: String,
    #[serde(rename = "package-name-full")]
    /// Long package name
    pub package_name_full: String,
    /// Complete list of files in the package
    pub files: Vec<String>,
}

impl Default for PackageInfo {
    /// A default, blank, package info structure
    fn default() -> Self {
        Self {
            name: "".to_string(),
            description: "".to_string(),
            version: "".to_string(),
            extra_version: "".to_string(),
            host: "".to_string(),
            confidentiality: "".to_string(),
            package_name: "".to_string(),
            package_number: -1,
            build_id: 0,
            build_id_namespace: "".to_string(),
            typ: "".to_string(),
            package_name_full: "".to_string(),
            files: vec![],
        }
    }
}

impl PackageInfo {
    /// Get the path to a package relative to the `simics_home` installation directory
    fn get_package_path<P: AsRef<Path>>(&self, simics_home: P) -> Result<PathBuf> {
        Ok(simics_home.as_ref().to_path_buf().join(
            self.files
                .iter()
                .take(1)
                .next()
                .context("No files in package.")?
                .split('/')
                .take(1)
                .next()
                .context("No base path.")?,
        ))
    }
}

/// Get all the package information of all packages in the `simics_home` installation directory as
/// a mapping between the package number and a nested mapping of package version to the package
/// info for the package
fn package_infos<P: AsRef<Path>>(
    simics_home: P,
) -> Result<HashMap<PackageNumber, HashMap<PackageVersion, PackageInfo>>> {
    let infos: Vec<PackageInfo> = read_dir(&simics_home)?
        .filter_map(|d| {
            d.map_err(|e| eprintln!("Could not read directory entry: {}", e))
                .ok()
        })
        .filter_map(|d| match d.path().join("packageinfo").is_dir() {
            true => Some(d.path().join("packageinfo")),
            false => {
                eprintln!(
                    "Package info path {:?} is not a directory",
                    d.path().join("packageinfo")
                );
                None
            }
        })
        .filter_map(|pid| match read_dir(&pid) {
            Ok(rd) => rd.into_iter().take(1).next().or_else(|| {
                eprintln!("No contents of packageinfo directory {:?}", pid);
                None
            }),
            Err(_) => None,
        })
        .filter_map(|pi| {
            pi.map_err(|e| {
                eprintln!("Could not get directory entry: {}", e);
                e
            })
            .ok()
        })
        .filter_map(|pi| {
            read_to_string(pi.path())
                .map_err(|e| {
                    eprintln!("Could not read file {:?} to string: {}", pi.path(), e);
                    e
                })
                .ok()
        })
        .map(|pis| {
            // TODO: This should be worked out with a real parser if possible
            // We're parsing it bespoke because...it's not yaml! yay
            let mut package_info = PackageInfo::default();
            pis.lines().for_each(|l| {
                if l.trim_start() != l {
                    // There is some whitespace at the front
                    package_info.files.push(l.trim().to_string());
                } else {
                    let kv: Vec<&str> = l.split(':').map(|lp| lp.trim()).collect();
                    if let Some(k) = kv.first() {
                        if let Some(v) = kv.get(1) {
                            match k.to_string().as_str() {
                                "name" => package_info.name = v.to_string(),
                                "description" => package_info.description = v.to_string(),
                                "version" => package_info.version = v.to_string(),
                                "extra-version" => package_info.extra_version = v.to_string(),
                                "host" => package_info.host = v.to_string(),
                                "confidentiality" => package_info.confidentiality = v.to_string(),
                                "package-name" => package_info.package_name = v.to_string(),
                                "package-number" => {
                                    package_info.package_number =
                                        v.to_string().parse().unwrap_or(0).try_into().unwrap_or(-1)
                                }
                                "build-id" => {
                                    package_info.build_id = v.to_string().parse().unwrap_or(0)
                                }
                                "build-id-namespace" => {
                                    package_info.build_id_namespace = v.to_string()
                                }
                                "type" => package_info.typ = v.to_string(),
                                "package-name-full" => {
                                    package_info.package_name_full = v.to_string()
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
            package_info
        })
        .collect();

    Ok(infos
        .iter()
        .group_by(|p| p.package_number)
        .into_iter()
        .map(|(k, g)| {
            let g: Vec<_> = g.collect();
            (
                k,
                g.iter()
                    .map(|p| (p.version.clone(), (*p).clone()))
                    .collect(),
            )
        })
        .collect())
}

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");

/// Return the SIMICS_HOME directory as a PathBuf. This depends on the SIMICS_HOME environment
/// variable being defined at compile time, and runtime changes to this variable will have no
/// effect.
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

/// Find the latest version of the Simics Base package with a particular constraint.
fn simics_base_version<P: AsRef<Path>, S: AsRef<str>>(
    simics_home: P,
    base_version_constraint: S,
) -> Result<PackageInfo> {
    let constraint: VersionConstraint = base_version_constraint.as_ref().parse()?;
    println!("Constraint: {:?}", constraint);
    let infos = package_infos(simics_home)?[&1000].clone();
    println!("Infos: {:?}", infos);
    let version = infos
        .keys()
        .filter_map(|k| Versioning::new(k))
        .filter(|v| constraint.matches(v))
        .max()
        .context("No matching version")?;

    Ok(infos
        .get(&version.to_string())
        .context(format!("No such version {}", version))?
        .clone())
}

/// Emit cargo directives to link to SIMICS given a particular version constraint
pub fn link_simics_linux<S: AsRef<str>>(version_constraint: S) -> Result<()> {
    let simics_home_dir = simics_home()?;

    let simics_base_info = simics_base_version(&simics_home_dir, &version_constraint)?;
    let simics_base_version = simics_base_info.version.clone();
    let simics_base_dir = simics_base_info.get_package_path(&simics_home_dir)?;
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

    let ld_line_pattern = Regex::new(r#"\s*([^\s]+)\s*=>\s*(.*)"#)?;
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
            let found_lib_parent = found_lib.parent().context("No parent path found")?;
            lib_search_dirs.insert(found_lib_parent.to_path_buf().canonicalize()?);
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

/// Locate a file recursively using a regex pattern in the simics base directory. If there are
/// multiple occurrences of a filename, it is undefined which will be returned.
fn find_file_in_dir<P: AsRef<Path>, S: AsRef<str>>(
    simics_base_dir: P,
    file_name_pattern: S,
) -> Result<PathBuf> {
    let file_name_regex = Regex::new(file_name_pattern.as_ref())?;
    let found_file = WalkDir::new(&simics_base_dir)
        .into_iter()
        .filter_map(|de| de.ok())
        // is_ok_and is unstable ;_;
        .filter(|de| {
            if let Ok(m) = de.metadata() {
                m.is_file()
            } else {
                false
            }
        })
        .find(|de| {
            if let Some(name) = de.path().file_name() {
                file_name_regex.is_match(&name.to_string_lossy())
            } else {
                false
            }
        })
        .context(format!(
            "Could not find {} in {}",
            file_name_pattern.as_ref(),
            simics_base_dir.as_ref().display()
        ))?
        .path()
        .to_path_buf();

    ensure!(
        found_file.is_file(),
        "No file {} found in {}",
        file_name_pattern.as_ref(),
        simics_base_dir.as_ref().display()
    );

    Ok(found_file)
}
