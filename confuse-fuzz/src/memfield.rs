use anyhow::{bail, ensure, Result};
use memfd::{FileSeal, Memfd, MemfdOptions};
use memmap2::{Mmap, MmapMut, MmapOptions};
use unix_ipc::Handle;

const AFL_MAP_SIZE: u64 = 1024 * 64;
const AFL_MAP_NAME: &str = "afl_map";

#[derive(Debug)]
pub struct MemFieldReader {
    pub size: u64,
    pub memfd: Memfd,
    reader: Option<Mmap>,
}

#[derive(Debug)]
pub struct MemFieldWriter {
    pub size: u64,
    pub memfd: Memfd,
    writer: Option<MmapMut>,
}

impl Default for MemFieldWriter {
    fn default() -> Self {
        Self::try_new(AFL_MAP_NAME, AFL_MAP_SIZE).expect("Could not create MemFieldWriter.")
    }
}

impl MemFieldWriter {
    pub fn try_new<S: AsRef<str>>(name: S, size: u64) -> Result<Self> {
        let opts = MemfdOptions::default().allow_sealing(true);
        let memfd = opts.create(name).expect("Could not create memfd.");
        memfd
            .as_file()
            .set_len(size)
            .expect("Could not set memfd size.");
        memfd
            .add_seal(FileSeal::SealShrink)
            .expect("Could not seal memfd.");
        memfd
            .add_seal(FileSeal::SealGrow)
            .expect("Could not seal memfd.");

        let mut writer = Self {
            size: AFL_MAP_SIZE,
            memfd,
            writer: None,
        };

        ensure!(writer.writer.is_none(), "Writer already exists.");
        ensure!(
            writer.size == writer.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            writer.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            writer.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );

        let writer_map = unsafe { MmapOptions::new().map_mut(&writer.memfd)? };

        writer.writer = Some(writer_map);

        writer.memfd.add_seal(FileSeal::SealFutureWrite)?;
        writer.memfd.add_seal(FileSeal::SealSeal)?;

        Ok(writer)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        ensure!(data.len() <= self.size as usize, "Data too large.");
        ensure!(self.writer.is_some(), "Writer does not exist.");
        ensure!(
            self.size == self.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let writer = self.writer.as_mut().unwrap();
        writer[..data.len()].copy_from_slice(data);

        Ok(())
    }

    pub fn write_at(&mut self, data: &[u8], offset: u64) -> Result<()> {
        ensure!(
            data.len() + offset as usize <= self.size as usize,
            "Data too large."
        );
        ensure!(self.writer.is_some(), "Writer does not exist.");
        ensure!(
            self.size == self.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let writer = self.writer.as_mut().unwrap();
        writer[offset as usize..offset as usize + data.len()].copy_from_slice(data);

        Ok(())
    }

    pub fn try_into_handle(&self) -> Result<Handle<Memfd>> {
        let memfd = match Memfd::try_from_file(self.memfd.as_file().try_clone()?) {
            Ok(memfd) => memfd,
            Err(_) => {
                bail!("Could not clone memfd.");
            }
        };
        Ok(Handle::new(memfd))
    }
}

impl TryFrom<Handle<Memfd>> for MemFieldReader {
    type Error = anyhow::Error;
    fn try_from(value: Handle<Memfd>) -> std::result::Result<Self, Self::Error> {
        let memfd = value.into_inner();
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader_map = unsafe { MmapOptions::new().map(&memfd)? };

        Ok(Self {
            size: memfd.as_file().metadata()?.len(),
            memfd,
            reader: Some(reader_map),
        })
    }
}

impl TryFrom<&MemFieldWriter> for MemFieldReader {
    type Error = anyhow::Error;
    fn try_from(value: &MemFieldWriter) -> std::result::Result<Self, Self::Error> {
        // Don't check whether the writer field exists -- specifically if we deserialized the writer
        // we won't have it which is OK for initializing a reader (of course we won't be able to write
        // but that's the desired behavior)
        ensure!(
            value.size == value.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            value.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            value.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            value.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            value.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let memfd = Memfd::try_from_file(value.memfd.as_file().try_clone()?)
            .expect("Could not create memfd.");
        let reader_map = unsafe { MmapOptions::new().map(&memfd)? };

        Ok(Self {
            size: value.size,
            memfd,
            reader: Some(reader_map),
        })
    }
}

impl TryFrom<(u64, Memfd)> for MemFieldReader {
    type Error = anyhow::Error;
    fn try_from(value: (u64, Memfd)) -> std::result::Result<Self, Self::Error> {
        let (size, memfd) = value;
        ensure!(size == memfd.as_file().metadata()?.len(), "Size mismatch.");
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader_map = unsafe { MmapOptions::new().map(&memfd)? };

        Ok(Self {
            size,
            memfd,
            reader: Some(reader_map),
        })
    }
}

impl TryFrom<i32> for MemFieldReader {
    type Error = anyhow::Error;
    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        let memfd = Memfd::try_from_fd(value).expect("Could not create memfd.");
        let size = memfd.as_file().metadata()?.len();
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader = unsafe { MmapOptions::new().map(&memfd)? };

        Ok(Self {
            size,
            memfd,
            reader: Some(reader),
        })
    }
}

impl MemFieldReader {
    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        ensure!(self.reader.is_some(), "Reader does not exist.");
        ensure!(
            self.size == self.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader = self.reader.as_ref().unwrap();
        let data = reader[..].to_vec();

        Ok(data)
    }

    pub fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        ensure!(len <= self.size as usize, "Read length exceeds size.");
        ensure!(self.reader.is_some(), "Reader does not exist.");
        ensure!(
            self.size == self.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader = self.reader.as_ref().unwrap();
        let data = reader[..len].to_vec();

        Ok(data)
    }

    pub fn read_at(&mut self, len: usize, offset: u64) -> Result<Vec<u8>> {
        ensure!(
            len + offset as usize <= self.size as usize,
            "Read length exceeds size."
        );
        ensure!(self.reader.is_some(), "Reader does not exist.");
        ensure!(
            self.size == self.memfd.as_file().metadata()?.len(),
            "Size mismatch."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealShrink),
            "Seal mismatch. Expected SealShrink."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealGrow),
            "Seal mismatch. Expected SealGrow."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealFutureWrite),
            "Seal mismatch. Expected SealFutureWrite."
        );
        ensure!(
            self.memfd.seals()?.contains(&FileSeal::SealSeal),
            "Seal mismatch. Expected SealSeal."
        );

        let reader = self.reader.as_ref().unwrap();
        let data = reader[offset as usize..offset as usize + len].to_vec();

        Ok(data)
    }
}
