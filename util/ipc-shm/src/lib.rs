use anyhow::{ensure, Result};
use ipc_channel::descriptor::OwnedDescriptor;
use memfd::{FileSeal, Memfd, MemfdOptions};
use memmap2::{Mmap, MmapMut, MmapOptions};
use serde::{Deserialize, Serialize};

const AFL_MAP_SIZE: usize = 1024 * 64;
const AFL_MAP_NAME: &str = "afl_map";

#[derive(Debug)]
pub struct IpcShm {
    pub size: usize,
    pub memfd: Memfd,
}

#[derive(Debug, Serialize, Deserialize)]
struct IpcShmSerializable {
    pub size: usize,
    pub backing_file: OwnedDescriptor,
}

impl Into<IpcShmSerializable> for &IpcShm {
    fn into(self) -> IpcShmSerializable {
        let backing_file = self
            .memfd
            .as_file()
            .try_clone()
            .expect("Could not clone file");

        IpcShmSerializable {
            size: self.size,
            backing_file: backing_file.into(),
        }
    }
}

impl Into<IpcShm> for IpcShmSerializable {
    fn into(self) -> IpcShm {
        let memfd = Memfd::try_from_file(self.backing_file.into()).expect("Could not create memfd");

        IpcShm {
            size: self.size,
            memfd,
        }
    }
}

impl Default for IpcShm {
    fn default() -> Self {
        Self::try_new(AFL_MAP_NAME, AFL_MAP_SIZE).expect("Could not create IpcShmWriter.")
    }
}

impl IpcShm {
    pub fn try_new<S: AsRef<str>>(name: S, size: usize) -> Result<Self> {
        let opts = MemfdOptions::default().allow_sealing(true);
        let memfd = opts.create(name)?;

        // Set size and seal against size changes
        memfd.as_file().set_len(size as u64)?;
        memfd.add_seal(FileSeal::SealShrink)?;
        memfd.add_seal(FileSeal::SealGrow)?;

        // let mut_map = unsafe { MmapOptions::new().map_mut(&memfd)? };

        // memfd.add_seal(FileSeal::SealFutureWrite)?;
        // memfd.add_seal(FileSeal::SealSeal)?;

        Ok(Self { size, memfd })
    }

    pub fn try_clone(&self) -> Result<Self> {
        let backing_file = self.memfd.as_file().try_clone()?;
        let memfd = Memfd::try_from_file(backing_file).expect("Could not create memfd from file");

        Ok(Self {
            size: self.size,
            memfd,
        })
    }

    pub fn writer(&mut self) -> Result<IpcShmWriter> {
        let mmap = unsafe { MmapOptions::new().map_mut(&self.memfd)? };

        self.memfd.add_seal(FileSeal::SealFutureWrite)?;
        self.memfd.add_seal(FileSeal::SealSeal)?;

        Ok(IpcShmWriter {
            size: self.size,
            mmap,
        })
    }

    pub fn reader(&self) -> Result<IpcShmReader> {
        let mmap = unsafe { MmapOptions::new().map(&self.memfd)? };
        Ok(IpcShmReader {
            size: self.size,
            mmap,
        })
    }
}

impl Serialize for IpcShm {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serializable: IpcShmSerializable = self.into();
        serializable.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IpcShm {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serializable = IpcShmSerializable::deserialize(deserializer)?;
        Ok(serializable.into())
    }
}

pub struct IpcShmWriter {
    size: usize,
    mmap: MmapMut,
}

impl IpcShmWriter {
    pub fn len(&self) -> usize {
        self.size
    }
}

impl IpcShmWriter {
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        ensure!(data.len() <= self.size, "Data too large.");

        let writer = self.mmap.as_mut();
        writer[..data.len()].copy_from_slice(data);

        Ok(())
    }

    pub fn write_at(&mut self, data: &[u8], offset: usize) -> Result<()> {
        ensure!(data.len() + offset <= self.size, "Data too large.");

        let writer = self.mmap.as_mut();
        writer[offset..offset + data.len()].copy_from_slice(data);

        Ok(())
    }
}

pub struct IpcShmReader {
    size: usize,
    mmap: Mmap,
}

impl IpcShmReader {
    pub fn len(&self) -> usize {
        self.size
    }
}

impl IpcShmReader {
    pub fn read_all(&self) -> Result<Vec<u8>> {
        let reader = self.mmap.as_ref();
        let data = reader[..].to_vec();

        Ok(data)
    }

    pub fn read(&self, len: usize) -> Result<Vec<u8>> {
        ensure!(len <= self.size, "Read length exceeds size.");

        let reader = self.mmap.as_ref();
        let data = reader[..len].to_vec();

        Ok(data)
    }

    pub fn read_at(&self, len: usize, offset: usize) -> Result<Vec<u8>> {
        ensure!(len + offset <= self.size, "Read length exceeds size.");

        let reader = self.mmap.as_ref();
        let data = reader[offset..offset + len].to_vec();

        Ok(data)
    }
}
