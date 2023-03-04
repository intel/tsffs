use anyhow::{ensure, Result};
use confuse_module::CRATE_NAME as CONFUSE_MODULE_CRATE_NAME;
use confuse_simics_manifest::PackageNumber;
use confuse_simics_module::find_module;

use confuse_simics_project::SimicsProject;
use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use log::info;
use x509_parse::X509_PARSE_EFI_MODULE;

fn main() -> Result<()> {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));

    let confuse_module = find_module(CONFUSE_MODULE_CRATE_NAME)?;

    info!("Found confuse module: {}", confuse_module.display());

    ensure!(
        confuse_module.is_file(),
        "Confuse module at path {} does not exist.",
        confuse_module.display()
    );

    let project = SimicsProject::try_new()?
        .try_with_package(PackageNumber::QuickStartPlatform)?
        .try_with_module(CONFUSE_MODULE_CRATE_NAME, &confuse_module)?
        .try_with_file_contents(X509_PARSE_EFI_MODULE, "X509Parse.efi")?;

    info!("Created project at {}", project.base_path.display());

    Ok(())
}
