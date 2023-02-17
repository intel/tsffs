pub mod api;

use anyhow::{ensure, Result};
use lazy_static::lazy_static;
use memmap2::MmapMut;
use std::{fs::OpenOptions, path::PathBuf, process::id, sync::Mutex};

const DEV_SHM_DIR: &str = "/dev/shm";

pub struct Confuse {
    shm: Option<MmapMut>,
}

impl Default for Confuse {
    fn default() -> Self {
        Self { shm: None }
    }
}

impl Confuse {
    pub fn create_dio_shm(&mut self, size: usize) -> Result<()> {
        let shm_dir_path = PathBuf::from(DEV_SHM_DIR);
        ensure!(
            shm_dir.exists(),
            "Shared memory directory '{}' does not exist.",
            DEV_SHM_DIR
        );

        let shm_dev_path = shm_dir.join(format!("confuse-dio-shm-{}", id()));

        ensure!(
            !shm_dev.exists(),
            "Shared memory device '{}' already exists.",
            shm_dev.display()
        );

        let shm_dev = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&shm_dev_path)?;

        shm_dev.set_len(size)?;

        let mut mmap = unsafe { MmapMut::map_mut(&shm_dev)? };

        self.shm = Some(mmap);

        Ok(())
    }

    pub fn init(project: String, project_config: String, simics_pid: u32) -> Result<()> {}

    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

lazy_static! {
    pub static ref CONFUSE: Mutex<Confuse> = Mutex::new(Confuse::new());
}
