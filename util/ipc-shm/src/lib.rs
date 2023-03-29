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

impl IpcShm {
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IpcShmSerializable {
    pub size: usize,
    pub backing_file: OwnedDescriptor,
}

impl From<&IpcShm> for IpcShmSerializable {
    fn from(val: &IpcShm) -> Self {
        let backing_file = val
            .memfd
            .as_file()
            .try_clone()
            .expect("Could not clone file");

        IpcShmSerializable {
            size: val.size,
            backing_file: backing_file.into(),
        }
    }
}

impl From<IpcShmSerializable> for IpcShm {
    fn from(val: IpcShmSerializable) -> Self {
        let memfd = Memfd::try_from_file(val.backing_file.into()).expect("Could not create memfd");

        IpcShm {
            size: val.size,
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

    /// Obtain a unique writer to the shared memory. If there is already a `writer` obtained
    /// for this memory, it will still be able to write, so take care when using this method
    /// that `unique_writer` is called to obtain a first and *only* writable mapping if this is
    /// desired.
    pub fn unique_writer(&mut self) -> Result<IpcShmWriter> {
        let mmap = unsafe { MmapOptions::new().map_mut(&self.memfd)? };

        self.memfd.add_seal(FileSeal::SealFutureWrite)?;
        self.memfd.add_seal(FileSeal::SealSeal)?;

        Ok(IpcShmWriter {
            size: self.size,
            mmap,
        })
    }

    /// Obtain a non-unique writer to the shared memory. Other writers may be created before and
    /// after this one, all of which may write to the mapped memory. Using this method,
    /// synchronization should be used to ensure ordered mutable access
    pub fn writer(&mut self) -> Result<IpcShmWriter> {
        let mmap = unsafe { MmapOptions::new().map_mut(&self.memfd)? };

        // Make it so we can't apply any more seals, specifically SealFutureWrite to ensure
        // this mapping cannot be made write-sealed
        if self.memfd.seals()?.contains(&FileSeal::SealSeal) {
            self.memfd.add_seal(FileSeal::SealSeal)?;
        }

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
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

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

    pub fn write_byte(&mut self, data: u8, offset: usize) -> Result<()> {
        ensure!(offset < self.size, "Offset out of bounds");

        let writer = self.mmap.as_mut();
        writer[offset] = data;

        Ok(())
    }

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

    pub fn read_byte(&self, offset: usize) -> Result<u8> {
        ensure!(offset < self.size, "Read length exceeds size.");

        let reader = self.mmap.as_ref();
        let data = reader[offset];

        Ok(data)
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }
}

#[derive(Debug)]
pub struct IpcShmReader {
    size: usize,
    mmap: Mmap,
}

impl IpcShmReader {
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

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

    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.mmap.as_ptr() as *mut u8
    }
}
