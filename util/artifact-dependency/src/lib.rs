use anyhow::{anyhow, bail, ensure, Result};
use cargo_metadata::MetadataCommand;
use derive_builder::Builder;
use std::{
    env::var,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Copy)]
pub enum CrateType {
    Executable,
    CDynamicLibrary,
    Dylib,
    StaticLibrary,
    RustLibrary,
    ProcMacro,
    // NOTE: Doesn't include raw-dylib, which allows DLL linking without import libraries:
    // https://rust-lang.github.io/rfcs/2627-raw-dylib-kind.html
}

#[derive(Clone, Debug)]
pub enum Profile {
    Release,
    Dev,
    Other(String),
}

#[derive(Builder)]
#[builder(setter(strip_option))]
#[builder(build_fn(skip))]
pub struct ArtifactDependency {
    /// Workspace root to search for an artifact dependency in. Defaults to the current workspace
    /// if one is not provided.
    pub workspace_root: Option<PathBuf>,
    /// Crate name to search for an artifact dependency for.
    pub crate_name: String,
    /// Type of artifact to search for
    pub artifact_type: CrateType,
    /// Profile, defaults to the current profile
    pub profile: Option<Profile>,
    /// Build the artifact if it is missing
    pub build_missing: bool,
    #[builder(setter(each = "feature"))]
    pub features: Vec<String>,
}

// NOTE: Artifact naming is not very easy to discern, we have to dig hard into rustc.
// Windows dll import lib: https://github.com/rust-lang/rust/blob/b2b34bd83192c3d16c88655158f7d8d612513e88/compiler/rustc_codegen_llvm/src/back/archive.rs#L129
// Others by crate type: https://github.com/rust-lang/rust/blob/b2b34bd83192c3d16c88655158f7d8d612513e88/compiler/rustc_session/src/output.rs#L141
// The default settings: https://github.com/rust-lang/rust/blob/db9d1b20bba1968c1ec1fc49616d4742c1725b4b/compiler/rustc_target/src/spec/mod.rs#L1422-L1529
//
// | Platform Spec   | DLL Prefix | DLL Suffix | EXE Suffix | Staticlib Prefix | Staticlib Suffix |
// | Default         | lib (d)    | .so (d)    |            | lib (d)          | .a (d)           |
// | MSVC            |            | .dll       | .exe       |                  | .lib             |
// | Windows GNU     |            | .dll       | .exe       | lib (d)          | .a (d)           |
// | WASM            | lib (d)    | .wasm      | .wasm      | lib (d)          | .a (d)           |
// | AIX             | lib (d)    | .a         |            | lib (d)          | .a (d)           |
// | Apple           | lib (d)    | .dylib     |            | lib (d)          | .a (d,framework?)|
// | NVPTX           |            | .ptx       | .ptx       | lib (d)          | .a (d)           |
// | Windows GNULLVM |            | .dll       | .exe       | lib (d)          | .a (d)           |

#[cfg(target_family = "unix")]
const ARTIFACT_NAMEPARTS: (&str, &str, &str, &str, &str) = ("lib", ".so", "lib", ".a", "");
#[cfg(target_family = "darwin")]
const ARTIFACT_NAMEPARTS: (&str, &str, &str, &str, &str) = ("lib", ".dylib", "lib", ".a", "");
#[cfg(any(
    target = "x86_64_pc-windows-msvc",
    target = "aarch64-pc-windows-msvc",
    target = "i586-pc-windows-msvc",
    target = "i686-pc-windows-msvc"
))]
const ARTIFACT_NAMEPARTS: (&str, &str, &str, &str, &str) = ("", ".dll", "", ".lib", ".exe");
#[cfg(any(
    target = "x86_64_pc-windows-gnu",
    target = "i586-pc-windows-gnu",
    target = "i686-pc-windows-gnu"
))]
const ARTIFACT_NAMEPARTS: (&str, &str, &str, &str, &str) = ("", ".dll", "lib", ".a", ".exe");

#[cfg(debug_assertions)]
const PROFILE: Profile = Profile::Dev;
#[cfg(not(debug_assertions))]
const PROFILE: Profile = Profile::Release;

impl ArtifactDependency {
    pub fn search(&mut self) -> Result<PathBuf> {
        let workspace_root = self.workspace_root.clone().unwrap_or(
            MetadataCommand::new()
                .no_deps()
                .exec()?
                .workspace_root
                .into(),
        );

        let metadata = MetadataCommand::new()
            .no_deps()
            .manifest_path(workspace_root.join("Cargo.toml"))
            .exec()?;

        let package = metadata
            .packages
            .iter()
            .find(|p| p.name == self.crate_name)
            .ok_or_else(|| {
                anyhow!(
                    "No package matching name {} found in workspace at {}",
                    self.crate_name,
                    workspace_root.display()
                )
            })?;

        let package_name = package.name.clone();

        let (dll_prefix, dll_suffix, staticlib_prefix, staticlib_suffix, exe_suffix) =
            ARTIFACT_NAMEPARTS;

        let profile = self.profile.clone().unwrap_or(PROFILE);

        let profile_target_path = metadata.target_directory.join(match &profile {
            Profile::Release => "release".to_string(),
            Profile::Dev => "debug".to_string(),
            Profile::Other(o) => o.clone(),
        });

        let artifact_path = match self.artifact_type {
            CrateType::Executable => {
                profile_target_path.join(format!("{}{}", &package.name, exe_suffix))
            }
            CrateType::CDynamicLibrary => {
                profile_target_path.join(format!("{}{}{}", dll_prefix, &package.name, dll_suffix))
            }
            CrateType::StaticLibrary => profile_target_path.join(format!(
                "{}{}{}",
                staticlib_prefix, package.name, staticlib_suffix
            )),
            _ => bail!(
                "Crate type {:?} is not supported as an artifact dependency source",
                self.artifact_type
            ),
        };

        let artifact_path = if !artifact_path.exists() {
            if self.build_missing {
                let cargo = var("CARGO")?;
                let mut cargo_command = Command::new(cargo);
                cargo_command
                    .arg("--build")
                    .arg("--manifest-path")
                    .arg(workspace_root.join("Cargo.toml"))
                    .arg("--package")
                    .arg(&package.name);

                match &profile {
                    Profile::Release => {
                        cargo_command.arg("--release");
                    }
                    Profile::Other(o) => {
                        cargo_command.args(vec!["--profile".to_string(), o.clone()]);
                    }
                    _ => {}
                }

                cargo_command.arg(format!("--features={}", self.features.join(",")));

                let output = cargo_command
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output()?;

                if !output.status.success() {
                    bail!(
                        "Failed to build artifact crate:\nstdout: {}\nstderr: {}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                let artifact_path: PathBuf = artifact_path.into();
                ensure!(
                    artifact_path.exists(),
                    "Artifact build succeeded, but artifact not found in {}",
                    artifact_path.display()
                );
                artifact_path
            } else {
                let artifact_path: PathBuf = artifact_path.into();
                bail!(
                    "Artifact not found at {} and not set to build missing artifacts.",
                    artifact_path.display()
                );
            }
        } else {
            artifact_path.into()
        };

        Ok(artifact_path)
    }
}
