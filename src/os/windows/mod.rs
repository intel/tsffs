use anyhow::{anyhow, ensure, Result};
use debug_info::{Module, Process, SymbolInfo};
use ffi::ffi;
use intervaltree::IntervalTree;
use kernel::{find_kernel_with_idt, KernelInfo};
use raw_cstr::AsRawCstr;
use simics::{
    get_interface, get_object, get_processor_number, info, sys::cpu_cb_handle_t, ConfObject,
    CpuInstrumentationSubscribeInterface, IntRegisterInterface, ProcessorInfoV2Interface,
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    ffi::c_void,
    path::{Path, PathBuf},
};
use structs::WindowsKpcr;
use util::read_virtual;

use vergilius::bindings::*;

use crate::Tsffs;

pub mod debug_info;
pub mod idt;
pub mod kernel;
pub mod paging;
pub mod pdb;
pub mod structs;
pub mod util;

const KUSER_SHARED_DATA_ADDRESS_X86_64: u64 = 0xFFFFF78000000000;

#[derive(Debug)]
pub struct CpuInstrumentationCbHandle(usize);

impl From<*mut cpu_cb_handle_t> for CpuInstrumentationCbHandle {
    fn from(value: *mut cpu_cb_handle_t) -> Self {
        Self(value as usize)
    }
}

impl From<CpuInstrumentationCbHandle> for *mut cpu_cb_handle_t {
    fn from(value: CpuInstrumentationCbHandle) -> Self {
        value.0 as *mut _
    }
}

#[derive(Debug, Default)]
pub struct WindowsOsInfo {
    /// Kernel info
    pub kernel_info: Option<KernelInfo>,
    /// Per-CPU process list, there may be overlap between them.
    pub processes: HashMap<i32, Vec<Process>>,
    /// Per-CPU kernel module list, there may be overlap between them.
    pub modules: HashMap<i32, Vec<Module>>,
    /// Per-CPU Symbol lookup trees
    pub symbol_lookup_trees: HashMap<i32, IntervalTree<u64, SymbolInfo>>,
    /// Cache of full names of both processes and kernel modules which are not found from
    /// the pdb server
    pub not_found_full_name_cache: HashSet<String>,
    /// Callbacks on instruction to do coverage lookups
    pub instruction_callbacks: HashMap<i32, CpuInstrumentationCbHandle>,
}

impl WindowsOsInfo {
    pub fn collect<P>(
        &mut self,
        processor: *mut ConfObject,
        download_directory: P,
        guess_pdb_function_size: bool,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        info!(get_object("tsffs")?, "Collecting Windows OS information");
        let processor_nr = get_processor_number(processor)?;
        let mut processor_info_v2: ProcessorInfoV2Interface = get_interface(processor)?;

        if self.kernel_info.is_none() {
            info!(get_object("tsffs")?, "Collecting kernel information");
            // Make sure we're running 64-bit Windows
            ensure!(
                processor_info_v2.get_logical_address_width()? == 64,
                "Only 64-bit Windows is supported"
            );

            let kuser_shared_data = read_virtual::<windows_10_0_22631_2428_x64::_KUSER_SHARED_DATA>(
                processor,
                KUSER_SHARED_DATA_ADDRESS_X86_64,
            )?;

            let (maj, min, build) = (
                kuser_shared_data.NtMajorVersion,
                kuser_shared_data.NtMinorVersion,
                kuser_shared_data.NtBuildNumber,
            );

            ensure!(maj == 10, "Only Windows 10/11 is supported");

            // Initialize the KPCR to make sure we have a valid one at gs_base
            let _ = WindowsKpcr::new(processor, maj, min, build)?;
            let kernel_base = find_kernel_with_idt(processor, build)?;

            info!(get_object("tsffs")?, "Found kernel base {kernel_base:#x}");

            self.kernel_info = Some(KernelInfo::new(
                processor,
                "ntoskrnl.exe",
                kernel_base,
                download_directory.as_ref(),
                &mut self.not_found_full_name_cache,
                user_debug_info,
            )?);
        }

        info!(get_object("tsffs")?, "Collecting process list");

        self.processes.insert(
            processor_nr,
            self.kernel_info
                .as_mut()
                .expect("Kernel Info must be set at this point")
                .process_list(
                    processor,
                    download_directory.as_ref(),
                    &mut self.not_found_full_name_cache,
                    user_debug_info,
                )?,
        );

        info!(get_object("tsffs")?, "Collecting module list");

        self.modules.insert(
            processor_nr,
            self.kernel_info
                .as_mut()
                .expect("Kernel Info must be set at this point")
                .loaded_module_list(
                    processor,
                    download_directory.as_ref(),
                    &mut self.not_found_full_name_cache,
                    user_debug_info,
                )?,
        );

        let elements = self
            .modules
            .get_mut(&processor_nr)
            .ok_or_else(|| anyhow!("No modules for processor {processor_nr}"))?
            .iter_mut()
            .map(|m| m.intervals(guess_pdb_function_size))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .chain(
                self.kernel_info
                    .as_mut()
                    .expect("Kernel Info must be set at this point")
                    .current_process(
                        processor,
                        download_directory.as_ref(),
                        &mut self.not_found_full_name_cache,
                        user_debug_info,
                    )?
                    .modules
                    .iter_mut()
                    .map(|m| m.intervals(guess_pdb_function_size))
                    .collect::<Result<Vec<_>>>()?,
            )
            .flatten()
            .collect::<Vec<_>>();

        let mut filtered_elements = HashSet::new();

        // Deduplicate elements by their range
        let elements = elements
            .into_iter()
            .filter(|e| filtered_elements.insert(e.range.clone()))
            .collect::<Vec<_>>();

        self.symbol_lookup_trees.insert(
            processor_nr,
            elements.iter().cloned().collect::<IntervalTree<_, _>>(),
        );

        Ok(())
    }
}

impl Tsffs {
    pub fn on_control_register_write_windows_symcov(
        &mut self,
        trigger_obj: *mut ConfObject,
        register_nr: i64,
        value: i64,
    ) -> Result<()> {
        let mut int_register: IntRegisterInterface = get_interface(trigger_obj)?;
        let processor_nr = get_processor_number(trigger_obj)?;

        if self.processors.contains_key(&processor_nr)
            && self.coverage_enabled
            && self.windows
            && self.symbolic_coverage
            && register_nr == int_register.get_number("cr3".as_raw_cstr()?)? as i64
            && self
                .cr3_cache
                .get(&processor_nr)
                .is_some_and(|v| *v != value)
        {
            info!(
                    get_object("tsffs")?,
                    "Got write {value:#x} to CR3 for processor {processor_nr}, refreshing kernel & process mappings"
                );

            self.windows_os_info.collect(
                trigger_obj,
                &self.debuginfo_download_directory,
                self.guess_pdb_function_size,
                &self.debug_info,
            )?;

            self.cr3_cache.insert(processor_nr, value);
        }

        Ok(())
    }
}
