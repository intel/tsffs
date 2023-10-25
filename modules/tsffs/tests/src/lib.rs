//! Runs the SIMICS tests for the project

use anyhow::Result;
use getters::Getters;
use ispm_wrapper::{
    data::ProjectPackage,
    ispm::{
        self,
        packages::{InstallOptions, UninstallOptions},
        projects::CreateOptions,
        GlobalOptions,
    },
};
use std::{
    fs::{create_dir_all, remove_dir_all, write},
    path::{Path, PathBuf},
};
use typed_builder::TypedBuilder;

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Debug)]
pub enum Architecture {
    X86,
    Riscv,
}

impl Architecture {
    fn packages(&self) -> Vec<ProjectPackage> {
        match self {
            Architecture::X86 => vec![
                ProjectPackage::builder()
                    .package_number(1000)
                    .version("latest")
                    .build(),
                // QSP-x86
                ProjectPackage::builder()
                    .package_number(2096)
                    .version("latest")
                    .build(),
                // QSP-CPU
                ProjectPackage::builder()
                    .package_number(8112)
                    .version("latest")
                    .build(),
            ],
            Architecture::Riscv => vec![
                ProjectPackage::builder()
                    .package_number(1000)
                    .version("latest")
                    .build(),
                // RISC-V-CPU
                ProjectPackage::builder()
                    .package_number(2050)
                    .version("latest")
                    .build(),
                // RISC-V-Simple
                ProjectPackage::builder()
                    .package_number(2053)
                    .version("latest")
                    .build(),
            ],
        }
    }
}

#[derive(TypedBuilder, Debug)]
pub struct TestEnvSpec {
    #[builder(setter(into))]
    cargo_manifest_dir: String,
    #[builder(setter(into))]
    cargo_target_tmpdir: String,
    #[builder(setter(into))]
    name: String,

    #[builder(default, setter(strip_option, into))]
    arch: Option<Architecture>,
    #[builder(default, setter(into))]
    extra_packages: Vec<ProjectPackage>,
    #[builder(default = true)]
    tsffs: bool,
    #[builder(default)]
    files: Vec<(String, Vec<u8>)>,
    #[builder(default)]
    simics_home: Option<PathBuf>,
}

impl TestEnvSpec {
    pub fn to_env(&self) -> Result<TestEnv> {
        TestEnv::build(self)
    }
}

#[derive(Getters)]
pub struct TestEnv {
    /// The base of the test environment, e.g. the `CARGO_TARGET_TMPDIR` directory
    test_base: PathBuf,
    /// The subdirectory in the test environment for this test
    test_dir: PathBuf,
    /// The project subdirectory in the test environment for this test
    project_dir: PathBuf,
    /// The simics home subdirectory in the test environment for this test
    simics_home_dir: PathBuf,
}

impl TestEnv {
    fn install_tsffs<P, S>(simics_home_dir: P, cargo_manifest_dir: S) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        // NOTE: Uninstall and reinstall the tsffs module (installs the latest build)
        ispm::packages::uninstall(
            &UninstallOptions::builder()
                .packages([ProjectPackage::builder()
                    .package_number(31337)
                    .version("latest")
                    .build()])
                .global(
                    GlobalOptions::builder()
                        .install_dir(simics_home_dir.as_ref())
                        .build(),
                )
                .build(),
        )
        .map_err(|e| eprintln!("Not uninstalling package: {}", e))
        .ok();

        ispm::packages::install(
            &InstallOptions::builder()
                .package_paths([PathBuf::from(cargo_manifest_dir.as_ref())
                    .join("../../../")
                    .join("linux64")
                    .join("packages")
                    .join("simics-pkg-31337-6.0.0-linux64.ispm")])
                .global(
                    GlobalOptions::builder()
                        .install_dir(simics_home_dir.as_ref())
                        .trust_insecure_packages(true)
                        .build(),
                )
                .build(),
        )?;

        Ok(())
    }

    fn install_files<P>(project_dir: P, files: &Vec<(String, Vec<u8>)>) -> Result<()>
    where
        P: AsRef<Path>,
    {
        for (name, content) in files {
            write(project_dir.as_ref().join(name), content)?;
        }

        Ok(())
    }

    fn build(spec: &TestEnvSpec) -> Result<Self> {
        let test_base = PathBuf::from(&spec.cargo_target_tmpdir);
        let test_dir = test_base.join(&spec.name);

        if test_dir.is_dir() {
            remove_dir_all(&test_dir)?;
        }

        let project_dir = test_dir.join("project");

        let simics_home_dir = if let Some(simics_home) = spec.simics_home.as_ref() {
            simics_home.clone()
        } else {
            create_dir_all(test_dir.join("simics"))?;

            test_dir.join("simics")
        };

        let mut packages = spec.extra_packages.clone();

        if spec.tsffs {
            Self::install_tsffs(&simics_home_dir, &spec.cargo_manifest_dir)?;
            packages.push(
                ProjectPackage::builder()
                    .package_number(31337)
                    .version("latest")
                    .build(),
            );
        }

        if let Some(arch) = spec.arch.as_ref() {
            packages.extend(arch.packages().clone());
        }

        ispm::projects::create(
            &CreateOptions::builder()
                .packages(packages)
                .global(
                    GlobalOptions::builder()
                        .install_dir(&simics_home_dir)
                        .trust_insecure_packages(true)
                        .build(),
                )
                .build(),
            &project_dir,
        )?;

        Self::install_files(&project_dir, &spec.files)?;

        Ok(Self {
            test_base,
            test_dir,
            project_dir,
            simics_home_dir,
        })
    }
}
