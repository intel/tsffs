use anyhow::Result;
use ipc_channel::ipc::channel;
use ipc_shm::IpcShm;

#[test]
fn test_serialize_send_shm() -> Result<()> {
    let shm = IpcShm::default();

    let (tx, rx) = channel()?;

    tx.send(shm)?;

    let _shm_received = rx.recv()?;

    Ok(())
}

#[test]
fn test_unidirectional_write_read() -> Result<()> {
    let mut shm = IpcShm::default();

    let mut writer = shm.writer()?;

    writer.write(b"Hello, world!")?;
    println!("Writing");

    let (tx, rx) = channel()?;

    tx.send(shm)?;

    let shm_received = rx.recv()?;
    println!("Got shm");

    let reader = shm_received.reader()?;
    println!("Got reader");

    let res = reader.read(13)?;

    println!("Read");

    assert_eq!(&res, b"Hello, world!", "Not equal");

    Ok(())
}

#[test]
fn test_unidirectional_write_multi_read() -> Result<()> {
    let mut shm = IpcShm::default();

    let mut writer = shm.writer()?;

    writer.write(b"Hello, world!")?;
    println!("Writing");

    let (tx, rx) = channel()?;

    tx.send(shm.try_clone()?)?;
    tx.send(shm.try_clone()?)?;

    let shm_received = rx.recv()?;
    println!("Got shm");

    let shm_received2 = rx.recv()?;
    println!("Got shm");

    let reader = shm_received.reader()?;
    println!("Got reader");

    let reader2 = shm_received2.reader()?;
    println!("Got reader");

    let res = reader.read(13)?;

    println!("Read");

    let res2 = reader2.read(13)?;

    println!("Read");

    assert_eq!(&res, b"Hello, world!", "Not equal");
    assert_eq!(&res2, b"Hello, world!", "Not equal");

    Ok(())
}
