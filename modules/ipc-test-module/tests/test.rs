use anyhow::{bail, Result};
use confuse_fuzz::{
    message::{FuzzerEvent, Message, SimicsEvent},
    InitInfo,
};
use confuse_simics_manifest::PackageNumber;
use confuse_simics_project::SimicsProject;
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender, IpcSharedMemory};
use ipc_shm::{IpcShm, IpcShmReader};
use ipc_test_module::{BOOTSTRAP_SOCKNAME, CRATE_NAME};
use log::info;
use std::{env::var, path::PathBuf};
use test_cdylib::build_current_project;

#[test]
fn test_minimal_simics_module_exists() -> Result<()> {
    let dylib_path = build_current_project();

    assert!(dylib_path.is_file(), "No library found for module.");

    Ok(())
}

#[test]
fn test_load_ipc_test_module() -> Result<()> {
    let ipc_test_module_path = build_current_project();
    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);

    let simics_project = SimicsProject::try_new()?
        .try_with_package(PackageNumber::QuickStartPlatform)?
        .try_with_module(CRATE_NAME, &ipc_test_module_path)?;

    let (bootstrap, bootstrap_name) = IpcOneShotServer::new()?;

    let mut simics_process = simics_project
        .command()
        .arg("-batch-mode")
        .arg("-e")
        .arg("load-module ipc-test-module")
        .env(BOOTSTRAP_SOCKNAME, bootstrap_name)
        .env("RUST_LOG", "trace")
        .spawn()?;

    let (_, (tx, rx)): (_, (IpcSender<Message>, IpcReceiver<Message>)) = bootstrap.accept()?;

    info!("Sending initialize");

    tx.send(Message::FuzzerEvent(FuzzerEvent::Initialize(
        InitInfo::default(),
    )))?;

    info!("Receiving ipc shm");

    let shm = match rx.recv()? {
        Message::SimicsEvent(SimicsEvent::SharedMem(shm)) => shm,
        _ => bail!("Unexpected message received"),
    };

    let reader = shm.reader()?;

    let res = reader.read_all()?;

    for i in 0..res.len() {
        assert_eq!(
            res[i],
            (i % u8::MAX as usize) as u8,
            "Unexpected value in map"
        );
    }

    let status = simics_process.wait()?;

    assert!(status.success(), "Simics failed");

    Ok(())
}
