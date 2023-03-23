use anyhow::{bail, Result};
use confuse_simics_manifest::{PackageNumber, PublicPackageNumber};
use confuse_simics_project::SimicsProject;
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use ipc_test_module::messages::{FuzzerEvent, Message, SimicsEvent};
use ipc_test_module::{BOOTSTRAP_SOCKNAME, CRATE_NAME};
use log::info;
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

    let simics_project = SimicsProject::try_new()?
        .try_with_package_latest(PublicPackageNumber::QspX86)?
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

    tx.send(Message::FuzzerEvent(FuzzerEvent::Initialize))?;

    info!("Receiving ipc shm");

    let shm = match rx.recv()? {
        Message::SimicsEvent(SimicsEvent::SharedMem(shm)) => shm,
        _ => bail!("Unexpected message received"),
    };

    let reader = shm.reader()?;

    let res = reader.read_all()?;

    for (i, itm) in res.iter().enumerate() {
        assert_eq!(
            *itm,
            (i % u8::MAX as usize) as u8,
            "Unexpected value in map"
        );
    }

    let status = simics_process.wait()?;

    assert!(status.success(), "Simics failed");

    Ok(())
}
