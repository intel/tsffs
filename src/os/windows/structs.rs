use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Result};
use raw_cstr::AsRawCstr;
use simics::{debug, get_attribute, get_interface, ConfObject, IntRegisterInterface};
use vergilius::bindings::*;
use windows::Win32::{Foundation::UNICODE_STRING, System::Kernel::LIST_ENTRY};

use crate::os::windows::{debug_info::DebugInfo, util::read_virtual};

use super::{
    debug_info::ProcessModule,
    util::{read_unicode_string, read_unicode_string_dtb, read_virtual_dtb},
};

pub enum WindowsKpcr {
    Windows10_0_10240_16384 {
        kpcr: windows_10_0_10240_16384_x64::_KPCR,
    },
    Windows10_0_10586_0 {
        kpcr: windows_10_0_10586_0_x64::_KPCR,
    },
    Windows10_0_14393_0 {
        kpcr: windows_10_0_14393_0_x64::_KPCR,
    },
    Windows10_0_15063_0 {
        kpcr: windows_10_0_15063_0_x64::_KPCR,
    },
    Windows10_0_16299_15 {
        kpcr: windows_10_0_16299_15_x64::_KPCR,
    },
    Windows10_0_17134_1 {
        kpcr: windows_10_0_17134_1_x64::_KPCR,
    },
    Windows10_0_17763_107 {
        kpcr: windows_10_0_17763_107_x64::_KPCR,
    },
    Windows10_0_18362_418 {
        kpcr: windows_10_0_18362_418_x64::_KPCR,
    },
    Windows10_0_19041_1288 {
        kpcr: windows_10_0_19041_1288_x64::_KPCR,
    },
    Windows10_0_19045_2965 {
        kpcr: windows_10_0_19045_2965_x64::_KPCR,
    },
    Windows10_0_22000_194 {
        kpcr: windows_10_0_22000_194_x64::_KPCR,
    },
    Windows10_0_22621_382 {
        kpcr: windows_10_0_22621_382_x64::_KPCR,
    },
    Windows10_0_22631_2428 {
        kpcr: windows_10_0_22631_2428_x64::_KPCR,
    },
}

impl WindowsKpcr {
    pub fn new(processor: *mut ConfObject, maj: u32, min: u32, build: u32) -> Result<Self> {
        let mut int_register = get_interface::<IntRegisterInterface>(processor)?;
        let ia32_kernel_gs_base_nr =
            int_register.get_number("ia32_kernel_gs_base".as_raw_cstr()?)?;
        let ia32_gs_base_nr = int_register.get_number("ia32_gs_base".as_raw_cstr()?)?;
        let ia32_kernel_gs_base = int_register.read(ia32_kernel_gs_base_nr)?;
        let ia32_gs_base = int_register.read(ia32_gs_base_nr)?;
        let sim_idtr_base: u64 = get_attribute(processor, "idtr_base")?.try_into()?;
        debug!("Got SIM IDTR Base {:#x}", sim_idtr_base);

        let kpcr_address = max(ia32_gs_base, ia32_kernel_gs_base);
        debug!("Got KPCR address {:#x}", kpcr_address);

        debug!("Initializing KPCR for Windows {}.{}.{}", maj, min, build);

        match (maj, min, build) {
            (10, 0, 10240) => {
                let kpcr =
                    read_virtual::<windows_10_0_10240_16384_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_10240_16384 { kpcr })
            }
            (10, 0, 10586) => {
                let kpcr =
                    read_virtual::<windows_10_0_10586_0_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_10586_0 { kpcr })
            }
            (10, 0, 14393) => {
                let kpcr =
                    read_virtual::<windows_10_0_14393_0_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_14393_0 { kpcr })
            }
            (10, 0, 15063) => {
                let kpcr =
                    read_virtual::<windows_10_0_15063_0_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_15063_0 { kpcr })
            }
            (10, 0, 16299) => {
                let kpcr =
                    read_virtual::<windows_10_0_16299_15_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_16299_15 { kpcr })
            }
            (10, 0, 17134) => {
                let kpcr =
                    read_virtual::<windows_10_0_17134_1_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_17134_1 { kpcr })
            }
            (10, 0, 17763) => {
                let kpcr =
                    read_virtual::<windows_10_0_17763_107_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_17763_107 { kpcr })
            }
            (10, 0, 18362) => {
                let kpcr =
                    read_virtual::<windows_10_0_18362_418_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_18362_418 { kpcr })
            }
            (10, 0, 19041) => {
                let kpcr =
                    read_virtual::<windows_10_0_19041_1288_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_19041_1288 { kpcr })
            }
            (10, 0, 19045) => {
                let kpcr =
                    read_virtual::<windows_10_0_19045_2965_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_19045_2965 { kpcr })
            }
            (10, 0, 22000) => {
                let kpcr =
                    read_virtual::<windows_10_0_22000_194_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_22000_194 { kpcr })
            }
            (10, 0, 22621) => {
                let kpcr =
                    read_virtual::<windows_10_0_22621_382_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_22621_382 { kpcr })
            }
            (10, 0, 22631) => {
                let kpcr =
                    read_virtual::<windows_10_0_22631_2428_x64::_KPCR>(processor, kpcr_address)?;
                ensure!(
                    unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.Self_
                        == kpcr_address as *mut _,
                    "Invalid KPCR: Self != KPCR address"
                );
                ensure!(
                    kpcr.IdtBase == sim_idtr_base as *mut _,
                    "Invalid KPCR: IdtBase != IDTR base"
                );

                Ok(WindowsKpcr::Windows10_0_22631_2428 { kpcr })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn kpcrb_address(&self) -> u64 {
        match self {
            WindowsKpcr::Windows10_0_10240_16384 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_10586_0 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_14393_0 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_15063_0 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_16299_15 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_17134_1 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_17763_107 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_18362_418 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_19041_1288 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_19045_2965 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_22000_194 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_22621_382 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
            WindowsKpcr::Windows10_0_22631_2428 { kpcr } => {
                unsafe { kpcr.__bindgen_anon_1.__bindgen_anon_1 }.CurrentPrcb as u64
            }
        }
    }
}

pub enum WindowsKprcb {
    Windows10_0_10240_16384 {
        kprcb: windows_10_0_10240_16384_x64::_KPRCB,
    },
    Windows10_0_10586_0 {
        kprcb: windows_10_0_10586_0_x64::_KPRCB,
    },
    Windows10_0_14393_0 {
        kprcb: windows_10_0_14393_0_x64::_KPRCB,
    },
    Windows10_0_15063_0 {
        kprcb: windows_10_0_15063_0_x64::_KPRCB,
    },
    Windows10_0_16299_15 {
        kprcb: windows_10_0_16299_15_x64::_KPRCB,
    },
    Windows10_0_17134_1 {
        kprcb: windows_10_0_17134_1_x64::_KPRCB,
    },
    Windows10_0_17763_107 {
        kprcb: windows_10_0_17763_107_x64::_KPRCB,
    },
    Windows10_0_18362_418 {
        kprcb: windows_10_0_18362_418_x64::_KPRCB,
    },
    Windows10_0_19041_1288 {
        kprcb: windows_10_0_19041_1288_x64::_KPRCB,
    },
    Windows10_0_19045_2965 {
        kprcb: windows_10_0_19045_2965_x64::_KPRCB,
    },
    Windows10_0_22000_194 {
        kprcb: windows_10_0_22000_194_x64::_KPRCB,
    },
    Windows10_0_22621_382 {
        kprcb: windows_10_0_22621_382_x64::_KPRCB,
    },
    Windows10_0_22631_2428 {
        kprcb: windows_10_0_22631_2428_x64::_KPRCB,
    },
}

impl WindowsKprcb {
    pub fn new(
        processor: *mut ConfObject,
        maj: u32,
        min: u32,
        build: u32,
        kpcrb_address: u64,
    ) -> Result<Self> {
        debug!("Initializing KPRCB for Windows {}.{}.{}", maj, min, build);

        match (maj, min, build) {
            (10, 0, 10240) => {
                let kprcb =
                    read_virtual::<windows_10_0_10240_16384_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_10240_16384 { kprcb })
            }
            (10, 0, 10586) => {
                let kprcb =
                    read_virtual::<windows_10_0_10586_0_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_10586_0 { kprcb })
            }
            (10, 0, 14393) => {
                let kprcb =
                    read_virtual::<windows_10_0_14393_0_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_14393_0 { kprcb })
            }
            (10, 0, 15063) => {
                let kprcb =
                    read_virtual::<windows_10_0_15063_0_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_15063_0 { kprcb })
            }
            (10, 0, 16299) => {
                let kprcb =
                    read_virtual::<windows_10_0_16299_15_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_16299_15 { kprcb })
            }
            (10, 0, 17134) => {
                let kprcb =
                    read_virtual::<windows_10_0_17134_1_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_17134_1 { kprcb })
            }
            (10, 0, 17763) => {
                let kprcb =
                    read_virtual::<windows_10_0_17763_107_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_17763_107 { kprcb })
            }
            (10, 0, 18362) => {
                let kprcb =
                    read_virtual::<windows_10_0_18362_418_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_18362_418 { kprcb })
            }
            (10, 0, 19041) => {
                let kprcb =
                    read_virtual::<windows_10_0_19041_1288_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_19041_1288 { kprcb })
            }
            (10, 0, 19045) => {
                let kprcb =
                    read_virtual::<windows_10_0_19045_2965_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_19045_2965 { kprcb })
            }
            (10, 0, 22000) => {
                let kprcb =
                    read_virtual::<windows_10_0_22000_194_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_22000_194 { kprcb })
            }
            (10, 0, 22621) => {
                let kprcb =
                    read_virtual::<windows_10_0_22621_382_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_22621_382 { kprcb })
            }
            (10, 0, 22631) => {
                let kprcb =
                    read_virtual::<windows_10_0_22631_2428_x64::_KPRCB>(processor, kpcrb_address)?;

                Ok(WindowsKprcb::Windows10_0_22631_2428 { kprcb })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn current_thread(&self) -> u64 {
        match self {
            WindowsKprcb::Windows10_0_10240_16384 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_10586_0 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_14393_0 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_15063_0 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_16299_15 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_17134_1 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_17763_107 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_18362_418 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_19041_1288 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_19045_2965 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_22000_194 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_22621_382 { kprcb } => kprcb.CurrentThread as u64,
            WindowsKprcb::Windows10_0_22631_2428 { kprcb } => kprcb.CurrentThread as u64,
        }
    }
}

pub enum WindowsLdrDataTableEntry {
    Windows10_0_10240_16384 {
        ldr_data_table_entry: windows_10_0_10240_16384_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_10586_0 {
        ldr_data_table_entry: windows_10_0_10586_0_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_14393_0 {
        ldr_data_table_entry: windows_10_0_14393_0_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_15063_0 {
        ldr_data_table_entry: windows_10_0_15063_0_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_16299_15 {
        ldr_data_table_entry: windows_10_0_16299_15_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_17134_1 {
        ldr_data_table_entry: windows_10_0_17134_1_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_17763_107 {
        ldr_data_table_entry: windows_10_0_17763_107_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_18362_418 {
        ldr_data_table_entry: windows_10_0_18362_418_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_19041_1288 {
        ldr_data_table_entry: windows_10_0_19041_1288_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_19045_2965 {
        ldr_data_table_entry: windows_10_0_19045_2965_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_22000_194 {
        ldr_data_table_entry: windows_10_0_22000_194_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_22621_382 {
        ldr_data_table_entry: windows_10_0_22621_382_x64::_LDR_DATA_TABLE_ENTRY,
    },
    Windows10_0_22631_2428 {
        ldr_data_table_entry: windows_10_0_22631_2428_x64::_LDR_DATA_TABLE_ENTRY,
    },
}

impl WindowsLdrDataTableEntry {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        ldr_data_table_entry_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_10240_16384_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 10586) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_10586_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 14393) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_14393_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 15063) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_15063_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 16299) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_16299_15_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 17134) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_17134_1_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 17763) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_17763_107_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 18362) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_18362_418_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 19041) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_19041_1288_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 19045) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_19045_2965_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22000) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_22000_194_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22621) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_22621_382_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22631) => {
                let ldr_data_table_entry = read_virtual::<
                    windows_10_0_22631_2428_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, ldr_data_table_entry_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                    ldr_data_table_entry,
                })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn new_dtb(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        directory_table_base: u64,
        virtual_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_10240_16384_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 10586) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_10586_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 14393) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_14393_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 15063) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_15063_0_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 16299) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_16299_15_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 17134) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_17134_1_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 17763) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_17763_107_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 18362) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_18362_418_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 19041) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_19041_1288_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 19045) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_19045_2965_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22000) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_22000_194_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22621) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_22621_382_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                    ldr_data_table_entry,
                })
            }
            (10, 0, 22631) => {
                let ldr_data_table_entry = read_virtual_dtb::<
                    windows_10_0_22631_2428_x64::_LDR_DATA_TABLE_ENTRY,
                >(
                    processor, directory_table_base, virtual_address
                )?;
                Ok(WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                    ldr_data_table_entry,
                })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn new_from_in_memory_order_links(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        in_memory_order_links_address: u64,
    ) -> Result<Self> {
        let in_memory_order_links_offset = match (major, minor, build) {
            (10, 0, 10240) => {
                std::mem::offset_of!(
                    windows_10_0_10240_16384_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 10586) => {
                std::mem::offset_of!(
                    windows_10_0_10586_0_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 14393) => {
                std::mem::offset_of!(
                    windows_10_0_14393_0_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 15063) => {
                std::mem::offset_of!(
                    windows_10_0_15063_0_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 16299) => {
                std::mem::offset_of!(
                    windows_10_0_16299_15_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 17134) => {
                std::mem::offset_of!(
                    windows_10_0_17134_1_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 17763) => {
                std::mem::offset_of!(
                    windows_10_0_17763_107_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 18362) => {
                std::mem::offset_of!(
                    windows_10_0_18362_418_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 19041) => {
                std::mem::offset_of!(
                    windows_10_0_19041_1288_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 19045) => {
                std::mem::offset_of!(
                    windows_10_0_19045_2965_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 22000) => {
                std::mem::offset_of!(
                    windows_10_0_22000_194_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 22621) => {
                std::mem::offset_of!(
                    windows_10_0_22621_382_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (10, 0, 22631) => {
                std::mem::offset_of!(
                    windows_10_0_22631_2428_x64::_LDR_DATA_TABLE_ENTRY,
                    InMemoryOrderLinks
                )
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        };

        let ldr_data_table_entry_address =
            in_memory_order_links_address - in_memory_order_links_offset as u64;

        Self::new(processor, major, minor, build, ldr_data_table_entry_address)
    }

    pub fn dll_base(&self) -> u64 {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.DllBase as u64,
        }
    }

    pub fn entry_point(&self) -> u64 {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.EntryPoint as u64,
        }
    }

    pub fn size_of_image(&self) -> u64 {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => ldr_data_table_entry.SizeOfImage as u64,
        }
    }

    pub fn full_name(&self, processor: *mut ConfObject) -> Result<String> {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
            ),
        }
    }

    pub fn full_name_dtb(
        &self,
        processor: *mut ConfObject,
        directory_table_base: u64,
    ) -> Result<String> {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.FullDllName.Length as usize,
                ldr_data_table_entry.FullDllName.Buffer,
                directory_table_base,
            ),
        }
    }

    pub fn base_name(&self, processor: *mut ConfObject) -> Result<String> {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => read_unicode_string(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
            ),
        }
    }

    pub fn base_name_dtb(
        &self,
        processor: *mut ConfObject,
        directory_table_base: u64,
    ) -> Result<String> {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => read_unicode_string_dtb(
                processor,
                ldr_data_table_entry.BaseDllName.Length as usize,
                ldr_data_table_entry.BaseDllName.Buffer,
                directory_table_base,
            ),
        }
    }

    pub fn in_load_order_links(&self) -> LIST_ENTRY {
        match self {
            WindowsLdrDataTableEntry::Windows10_0_10240_16384 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10240_16384_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_10586_0 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10586_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_14393_0 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_14393_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_15063_0 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_15063_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_16299_15 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_16299_15_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_17134_1 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17134_1_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_17763_107 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17763_107_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_18362_418 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_18362_418_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_19041_1288 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19041_1288_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_19045_2965 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19045_2965_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_22000_194 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22000_194_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_22621_382 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22621_382_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
            WindowsLdrDataTableEntry::Windows10_0_22631_2428 {
                ldr_data_table_entry,
            } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22631_2428_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data_table_entry.InLoadOrderLinks)
            },
        }
    }
}

pub enum WindowsPebLdrData {
    Windows10_0_10240_16384 {
        ldr_data: windows_10_0_10240_16384_x64::_PEB_LDR_DATA,
    },
    Windows10_0_10586_0 {
        ldr_data: windows_10_0_10586_0_x64::_PEB_LDR_DATA,
    },
    Windows10_0_14393_0 {
        ldr_data: windows_10_0_14393_0_x64::_PEB_LDR_DATA,
    },
    Windows10_0_15063_0 {
        ldr_data: windows_10_0_15063_0_x64::_PEB_LDR_DATA,
    },
    Windows10_0_16299_15 {
        ldr_data: windows_10_0_16299_15_x64::_PEB_LDR_DATA,
    },
    Windows10_0_17134_1 {
        ldr_data: windows_10_0_17134_1_x64::_PEB_LDR_DATA,
    },
    Windows10_0_17763_107 {
        ldr_data: windows_10_0_17763_107_x64::_PEB_LDR_DATA,
    },
    Windows10_0_18362_418 {
        ldr_data: windows_10_0_18362_418_x64::_PEB_LDR_DATA,
    },
    Windows10_0_19041_1288 {
        ldr_data: windows_10_0_19041_1288_x64::_PEB_LDR_DATA,
    },
    Windows10_0_19045_2965 {
        ldr_data: windows_10_0_19045_2965_x64::_PEB_LDR_DATA,
    },
    Windows10_0_22000_194 {
        ldr_data: windows_10_0_22000_194_x64::_PEB_LDR_DATA,
    },
    Windows10_0_22621_382 {
        ldr_data: windows_10_0_22621_382_x64::_PEB_LDR_DATA,
    },
    Windows10_0_22631_2428 {
        ldr_data: windows_10_0_22631_2428_x64::_PEB_LDR_DATA,
    },
}

impl WindowsPebLdrData {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        ldr_data_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let ldr_data = read_virtual::<windows_10_0_10240_16384_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_10240_16384 { ldr_data })
            }
            (10, 0, 10586) => {
                let ldr_data = read_virtual::<windows_10_0_10586_0_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_10586_0 { ldr_data })
            }
            (10, 0, 14393) => {
                let ldr_data = read_virtual::<windows_10_0_14393_0_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_14393_0 { ldr_data })
            }
            (10, 0, 15063) => {
                let ldr_data = read_virtual::<windows_10_0_15063_0_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_15063_0 { ldr_data })
            }
            (10, 0, 16299) => {
                let ldr_data = read_virtual::<windows_10_0_16299_15_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_16299_15 { ldr_data })
            }
            (10, 0, 17134) => {
                let ldr_data = read_virtual::<windows_10_0_17134_1_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_17134_1 { ldr_data })
            }
            (10, 0, 17763) => {
                let ldr_data = read_virtual::<windows_10_0_17763_107_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_17763_107 { ldr_data })
            }
            (10, 0, 18362) => {
                let ldr_data = read_virtual::<windows_10_0_18362_418_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_18362_418 { ldr_data })
            }
            (10, 0, 19041) => {
                let ldr_data = read_virtual::<windows_10_0_19041_1288_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_19041_1288 { ldr_data })
            }
            (10, 0, 19045) => {
                let ldr_data = read_virtual::<windows_10_0_19045_2965_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_19045_2965 { ldr_data })
            }
            (10, 0, 22000) => {
                let ldr_data = read_virtual::<windows_10_0_22000_194_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22000_194 { ldr_data })
            }
            (10, 0, 22621) => {
                let ldr_data = read_virtual::<windows_10_0_22621_382_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22621_382 { ldr_data })
            }
            (10, 0, 22631) => {
                let ldr_data = read_virtual::<windows_10_0_22631_2428_x64::_PEB_LDR_DATA>(
                    processor,
                    ldr_data_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22631_2428 { ldr_data })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn new_dtb(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        directory_table_base: u64,
        virtual_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_10240_16384_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_10240_16384 { ldr_data })
            }
            (10, 0, 10586) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_10586_0_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_10586_0 { ldr_data })
            }
            (10, 0, 14393) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_14393_0_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_14393_0 { ldr_data })
            }
            (10, 0, 15063) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_15063_0_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_15063_0 { ldr_data })
            }
            (10, 0, 16299) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_16299_15_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_16299_15 { ldr_data })
            }
            (10, 0, 17134) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_17134_1_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_17134_1 { ldr_data })
            }
            (10, 0, 17763) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_17763_107_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_17763_107 { ldr_data })
            }
            (10, 0, 18362) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_18362_418_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_18362_418 { ldr_data })
            }
            (10, 0, 19041) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_19041_1288_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_19041_1288 { ldr_data })
            }
            (10, 0, 19045) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_19045_2965_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_19045_2965 { ldr_data })
            }
            (10, 0, 22000) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_22000_194_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22000_194 { ldr_data })
            }
            (10, 0, 22621) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_22621_382_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22621_382 { ldr_data })
            }
            (10, 0, 22631) => {
                let ldr_data = read_virtual_dtb::<windows_10_0_22631_2428_x64::_PEB_LDR_DATA>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPebLdrData::Windows10_0_22631_2428 { ldr_data })
            }
            (_, _, _) => bail!("Unsupported Windows version"),
        }
    }

    pub fn length(&self) -> usize {
        match self {
            WindowsPebLdrData::Windows10_0_10240_16384 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_10586_0 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_14393_0 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_15063_0 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_16299_15 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_17134_1 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_17763_107 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_18362_418 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_19041_1288 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_19045_2965 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_22000_194 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_22621_382 { ldr_data } => ldr_data.Length as usize,
            WindowsPebLdrData::Windows10_0_22631_2428 { ldr_data } => ldr_data.Length as usize,
        }
    }

    pub fn in_load_order_module_list(&self) -> LIST_ENTRY {
        match self {
            WindowsPebLdrData::Windows10_0_10240_16384 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10240_16384_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_10586_0 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10586_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_14393_0 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_14393_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_15063_0 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_15063_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_16299_15 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_16299_15_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_17134_1 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17134_1_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_17763_107 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17763_107_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_18362_418 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_18362_418_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_19041_1288 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19041_1288_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_19045_2965 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19045_2965_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_22000_194 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22000_194_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_22621_382 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22621_382_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
            WindowsPebLdrData::Windows10_0_22631_2428 { ldr_data } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22631_2428_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(ldr_data.InLoadOrderModuleList)
            },
        }
    }
}

pub enum WindowsPeb {
    Windows10_0_10240_16384 {
        peb: windows_10_0_10240_16384_x64::_PEB,
    },
    Windows10_0_10586_0 {
        peb: windows_10_0_10586_0_x64::_PEB,
    },
    Windows10_0_14393_0 {
        peb: windows_10_0_14393_0_x64::_PEB,
    },
    Windows10_0_15063_0 {
        peb: windows_10_0_15063_0_x64::_PEB,
    },
    Windows10_0_16299_15 {
        peb: windows_10_0_16299_15_x64::_PEB,
    },
    Windows10_0_17134_1 {
        peb: windows_10_0_17134_1_x64::_PEB,
    },
    Windows10_0_17763_107 {
        peb: windows_10_0_17763_107_x64::_PEB,
    },
    Windows10_0_18362_418 {
        peb: windows_10_0_18362_418_x64::_PEB,
    },
    Windows10_0_19041_1288 {
        peb: windows_10_0_19041_1288_x64::_PEB,
    },
    Windows10_0_19045_2965 {
        peb: windows_10_0_19045_2965_x64::_PEB,
    },
    Windows10_0_22000_194 {
        peb: windows_10_0_22000_194_x64::_PEB,
    },
    Windows10_0_22621_382 {
        peb: windows_10_0_22621_382_x64::_PEB,
    },
    Windows10_0_22631_2428 {
        peb: windows_10_0_22631_2428_x64::_PEB,
    },
}

impl WindowsPeb {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        peb_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let peb =
                    read_virtual::<windows_10_0_10240_16384_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_10240_16384 { peb })
            }
            (10, 0, 10586) => {
                let peb = read_virtual::<windows_10_0_10586_0_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_10586_0 { peb })
            }
            (10, 0, 14393) => {
                let peb = read_virtual::<windows_10_0_14393_0_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_14393_0 { peb })
            }
            (10, 0, 15063) => {
                let peb = read_virtual::<windows_10_0_15063_0_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_15063_0 { peb })
            }
            (10, 0, 16299) => {
                let peb = read_virtual::<windows_10_0_16299_15_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_16299_15 { peb })
            }
            (10, 0, 17134) => {
                let peb = read_virtual::<windows_10_0_17134_1_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_17134_1 { peb })
            }
            (10, 0, 17763) => {
                let peb = read_virtual::<windows_10_0_17763_107_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_17763_107 { peb })
            }
            (10, 0, 18362) => {
                let peb = read_virtual::<windows_10_0_18362_418_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_18362_418 { peb })
            }
            (10, 0, 19041) => {
                let peb =
                    read_virtual::<windows_10_0_19041_1288_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_19041_1288 { peb })
            }
            (10, 0, 19045) => {
                let peb =
                    read_virtual::<windows_10_0_19045_2965_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_19045_2965 { peb })
            }
            (10, 0, 22000) => {
                let peb = read_virtual::<windows_10_0_22000_194_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_22000_194 { peb })
            }
            (10, 0, 22621) => {
                let peb = read_virtual::<windows_10_0_22621_382_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_22621_382 { peb })
            }
            (10, 0, 22631) => {
                let peb =
                    read_virtual::<windows_10_0_22631_2428_x64::_PEB>(processor, peb_address)?;
                Ok(WindowsPeb::Windows10_0_22631_2428 { peb })
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        }
    }

    pub fn new_dtb(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        directory_table_base: u64,
        virtual_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let peb = read_virtual_dtb::<windows_10_0_10240_16384_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_10240_16384 { peb })
            }
            (10, 0, 10586) => {
                let peb = read_virtual_dtb::<windows_10_0_10586_0_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_10586_0 { peb })
            }
            (10, 0, 14393) => {
                let peb = read_virtual_dtb::<windows_10_0_14393_0_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_14393_0 { peb })
            }
            (10, 0, 15063) => {
                let peb = read_virtual_dtb::<windows_10_0_15063_0_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_15063_0 { peb })
            }
            (10, 0, 16299) => {
                let peb = read_virtual_dtb::<windows_10_0_16299_15_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_16299_15 { peb })
            }
            (10, 0, 17134) => {
                let peb = read_virtual_dtb::<windows_10_0_17134_1_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_17134_1 { peb })
            }
            (10, 0, 17763) => {
                let peb = read_virtual_dtb::<windows_10_0_17763_107_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_17763_107 { peb })
            }
            (10, 0, 18362) => {
                let peb = read_virtual_dtb::<windows_10_0_18362_418_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_18362_418 { peb })
            }
            (10, 0, 19041) => {
                let peb = read_virtual_dtb::<windows_10_0_19041_1288_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_19041_1288 { peb })
            }
            (10, 0, 19045) => {
                let peb = read_virtual_dtb::<windows_10_0_19045_2965_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_19045_2965 { peb })
            }
            (10, 0, 22000) => {
                let peb = read_virtual_dtb::<windows_10_0_22000_194_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_22000_194 { peb })
            }
            (10, 0, 22621) => {
                let peb = read_virtual_dtb::<windows_10_0_22621_382_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_22621_382 { peb })
            }
            (10, 0, 22631) => {
                let peb = read_virtual_dtb::<windows_10_0_22631_2428_x64::_PEB>(
                    processor,
                    directory_table_base,
                    virtual_address,
                )?;
                Ok(WindowsPeb::Windows10_0_22631_2428 { peb })
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        }
    }

    pub fn base(&self) -> u64 {
        match self {
            WindowsPeb::Windows10_0_10240_16384 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_10586_0 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_14393_0 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_15063_0 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_16299_15 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_17134_1 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_17763_107 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_18362_418 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_19041_1288 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_19045_2965 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_22000_194 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_22621_382 { peb } => peb.ImageBaseAddress as u64,
            WindowsPeb::Windows10_0_22631_2428 { peb } => peb.ImageBaseAddress as u64,
        }
    }

    pub fn ldr_address(&self) -> u64 {
        match self {
            WindowsPeb::Windows10_0_10240_16384 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_10586_0 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_14393_0 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_15063_0 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_16299_15 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_17134_1 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_17763_107 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_18362_418 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_19041_1288 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_19045_2965 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_22000_194 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_22621_382 { peb } => peb.Ldr as u64,
            WindowsPeb::Windows10_0_22631_2428 { peb } => peb.Ldr as u64,
        }
    }
}

pub enum WindowsTeb {
    Windows10_0_10240_16384 {
        teb: windows_10_0_10240_16384_x64::_TEB,
    },
    Windows10_0_10586_0 {
        teb: windows_10_0_10586_0_x64::_TEB,
    },
    Windows10_0_14393_0 {
        teb: windows_10_0_14393_0_x64::_TEB,
    },
    Windows10_0_15063_0 {
        teb: windows_10_0_15063_0_x64::_TEB,
    },
    Windows10_0_16299_15 {
        teb: windows_10_0_16299_15_x64::_TEB,
    },
    Windows10_0_17134_1 {
        teb: windows_10_0_17134_1_x64::_TEB,
    },
    Windows10_0_17763_107 {
        teb: windows_10_0_17763_107_x64::_TEB,
    },
    Windows10_0_18362_418 {
        teb: windows_10_0_18362_418_x64::_TEB,
    },
    Windows10_0_19041_1288 {
        teb: windows_10_0_19041_1288_x64::_TEB,
    },
    Windows10_0_19045_2965 {
        teb: windows_10_0_19045_2965_x64::_TEB,
    },
    Windows10_0_22000_194 {
        teb: windows_10_0_22000_194_x64::_TEB,
    },
    Windows10_0_22621_382 {
        teb: windows_10_0_22621_382_x64::_TEB,
    },
    Windows10_0_22631_2428 {
        teb: windows_10_0_22631_2428_x64::_TEB,
    },
}

impl WindowsTeb {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        teb_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let teb =
                    read_virtual::<windows_10_0_10240_16384_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_10240_16384 { teb })
            }
            (10, 0, 10586) => {
                let teb = read_virtual::<windows_10_0_10586_0_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_10586_0 { teb })
            }
            (10, 0, 14393) => {
                let teb = read_virtual::<windows_10_0_14393_0_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_14393_0 { teb })
            }
            (10, 0, 15063) => {
                let teb = read_virtual::<windows_10_0_15063_0_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_15063_0 { teb })
            }
            (10, 0, 16299) => {
                let teb = read_virtual::<windows_10_0_16299_15_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_16299_15 { teb })
            }
            (10, 0, 17134) => {
                let teb = read_virtual::<windows_10_0_17134_1_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_17134_1 { teb })
            }
            (10, 0, 17763) => {
                let teb = read_virtual::<windows_10_0_17763_107_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_17763_107 { teb })
            }
            (10, 0, 18362) => {
                let teb = read_virtual::<windows_10_0_18362_418_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_18362_418 { teb })
            }
            (10, 0, 19041) => {
                let teb =
                    read_virtual::<windows_10_0_19041_1288_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_19041_1288 { teb })
            }
            (10, 0, 19045) => {
                let teb =
                    read_virtual::<windows_10_0_19045_2965_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_19045_2965 { teb })
            }
            (10, 0, 22000) => {
                let teb = read_virtual::<windows_10_0_22000_194_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_22000_194 { teb })
            }
            (10, 0, 22621) => {
                let teb = read_virtual::<windows_10_0_22621_382_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_22621_382 { teb })
            }
            (10, 0, 22631) => {
                let teb =
                    read_virtual::<windows_10_0_22631_2428_x64::_TEB>(processor, teb_address)?;
                Ok(WindowsTeb::Windows10_0_22631_2428 { teb })
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        }
    }

    pub fn peb(
        &self,
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
    ) -> Result<WindowsPeb> {
        let peb_address = match self {
            WindowsTeb::Windows10_0_10240_16384 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_10586_0 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_14393_0 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_15063_0 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_16299_15 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_17134_1 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_17763_107 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_18362_418 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_19041_1288 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_19045_2965 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_22000_194 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_22621_382 { teb } => teb.ProcessEnvironmentBlock as u64,
            WindowsTeb::Windows10_0_22631_2428 { teb } => teb.ProcessEnvironmentBlock as u64,
        };
        debug!("peb_address: {:#x}", peb_address);
        WindowsPeb::new(processor, major, minor, build, peb_address)
    }
}

pub enum WindowsEProcess {
    Windows10_0_10240_16384 {
        eprocess: windows_10_0_10240_16384_x64::_EPROCESS,
    },
    Windows10_0_10586_0 {
        eprocess: windows_10_0_10586_0_x64::_EPROCESS,
    },
    Windows10_0_14393_0 {
        eprocess: windows_10_0_14393_0_x64::_EPROCESS,
    },
    Windows10_0_15063_0 {
        eprocess: windows_10_0_15063_0_x64::_EPROCESS,
    },
    Windows10_0_16299_15 {
        eprocess: windows_10_0_16299_15_x64::_EPROCESS,
    },
    Windows10_0_17134_1 {
        eprocess: windows_10_0_17134_1_x64::_EPROCESS,
    },
    Windows10_0_17763_107 {
        eprocess: windows_10_0_17763_107_x64::_EPROCESS,
    },
    Windows10_0_18362_418 {
        eprocess: windows_10_0_18362_418_x64::_EPROCESS,
    },
    Windows10_0_19041_1288 {
        eprocess: windows_10_0_19041_1288_x64::_EPROCESS,
    },
    Windows10_0_19045_2965 {
        eprocess: windows_10_0_19045_2965_x64::_EPROCESS,
    },
    Windows10_0_22000_194 {
        eprocess: windows_10_0_22000_194_x64::_EPROCESS,
    },
    Windows10_0_22621_382 {
        eprocess: windows_10_0_22621_382_x64::_EPROCESS,
    },
    Windows10_0_22631_2428 {
        eprocess: windows_10_0_22631_2428_x64::_EPROCESS,
    },
}

impl WindowsEProcess {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        eprocess_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let eprocess = read_virtual::<windows_10_0_10240_16384_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_10240_16384 { eprocess })
            }
            (10, 0, 10586) => {
                let eprocess = read_virtual::<windows_10_0_10586_0_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_10586_0 { eprocess })
            }
            (10, 0, 14393) => {
                let eprocess = read_virtual::<windows_10_0_14393_0_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_14393_0 { eprocess })
            }
            (10, 0, 15063) => {
                let eprocess = read_virtual::<windows_10_0_15063_0_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_15063_0 { eprocess })
            }
            (10, 0, 16299) => {
                let eprocess = read_virtual::<windows_10_0_16299_15_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_16299_15 { eprocess })
            }
            (10, 0, 17134) => {
                let eprocess = read_virtual::<windows_10_0_17134_1_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_17134_1 { eprocess })
            }
            (10, 0, 17763) => {
                let eprocess = read_virtual::<windows_10_0_17763_107_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_17763_107 { eprocess })
            }
            (10, 0, 18362) => {
                let eprocess = read_virtual::<windows_10_0_18362_418_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_18362_418 { eprocess })
            }
            (10, 0, 19041) => {
                let eprocess = read_virtual::<windows_10_0_19041_1288_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_19041_1288 { eprocess })
            }
            (10, 0, 19045) => {
                let eprocess = read_virtual::<windows_10_0_19045_2965_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_19045_2965 { eprocess })
            }
            (10, 0, 22000) => {
                let eprocess = read_virtual::<windows_10_0_22000_194_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_22000_194 { eprocess })
            }
            (10, 0, 22621) => {
                let eprocess = read_virtual::<windows_10_0_22621_382_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_22621_382 { eprocess })
            }
            (10, 0, 22631) => {
                let eprocess = read_virtual::<windows_10_0_22631_2428_x64::_EPROCESS>(
                    processor,
                    eprocess_address,
                )?;
                Ok(WindowsEProcess::Windows10_0_22631_2428 { eprocess })
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        }
    }

    pub fn new_from_active_process_links_address(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        active_process_links_address: u64,
    ) -> Result<Self> {
        let active_process_links_offset = match (major, minor, build) {
            (10, 0, 10240) => {
                std::mem::offset_of!(windows_10_0_10240_16384_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 10586) => {
                std::mem::offset_of!(windows_10_0_10586_0_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 14393) => {
                std::mem::offset_of!(windows_10_0_14393_0_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 15063) => {
                std::mem::offset_of!(windows_10_0_15063_0_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 16299) => {
                std::mem::offset_of!(windows_10_0_16299_15_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 17134) => {
                std::mem::offset_of!(windows_10_0_17134_1_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 17763) => {
                std::mem::offset_of!(windows_10_0_17763_107_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 18362) => {
                std::mem::offset_of!(windows_10_0_18362_418_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 19041) => {
                std::mem::offset_of!(windows_10_0_19041_1288_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 19045) => {
                std::mem::offset_of!(windows_10_0_19045_2965_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 22000) => {
                std::mem::offset_of!(windows_10_0_22000_194_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 22621) => {
                std::mem::offset_of!(windows_10_0_22621_382_x64::_EPROCESS, ActiveProcessLinks)
            }
            (10, 0, 22631) => {
                std::mem::offset_of!(windows_10_0_22631_2428_x64::_EPROCESS, ActiveProcessLinks)
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        };
        let eprocess_address = active_process_links_address - active_process_links_offset as u64;

        Self::new(processor, major, minor, build, eprocess_address)
    }

    pub fn active_process_links(&self) -> LIST_ENTRY {
        match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10240_16384_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_10586_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_14393_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_15063_0_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_16299_15_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17134_1_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_17763_107_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_18362_418_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19041_1288_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_19045_2965_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22000_194_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22621_382_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => unsafe {
                std::mem::transmute::<
                    vergilius::windows_10_0_22631_2428_x64::_LIST_ENTRY,
                    windows::Win32::System::Kernel::LIST_ENTRY,
                >(eprocess.ActiveProcessLinks)
            },
        }
    }

    pub fn pid(&self) -> u64 {
        match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => {
                eprocess.UniqueProcessId as u64
            }
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => eprocess.UniqueProcessId as u64,
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => eprocess.UniqueProcessId as u64,
        }
    }

    pub fn file_name(&self, processor: *mut ConfObject) -> Result<String> {
        // 1. Read _EPROCESS.SeAuditProcessCreationInfo.ImageFileName
        let object_name_information_addr = match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => {
                eprocess.SeAuditProcessCreationInfo.ImageFileName as u64
            }
        };
        let object_name_information =
            read_virtual::<UNICODE_STRING>(processor, object_name_information_addr)?;
        read_unicode_string(
            processor,
            object_name_information.Length as usize,
            object_name_information.Buffer.0,
        )
    }

    pub fn base_address(
        &self,
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
    ) -> Result<u64> {
        let peb_address = match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => eprocess.Peb as u64,
        };
        let peb = WindowsPeb::new(processor, major, minor, build, peb_address)?;
        Ok(peb.base())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modules<P>(
        &self,
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        download_directory: P,
        not_found_full_name_cache: &mut HashSet<String>,
        user_debug_info: &HashMap<String, Vec<PathBuf>>,
    ) -> Result<Vec<ProcessModule>>
    where
        P: AsRef<Path>,
    {
        let peb_address = match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => eprocess.Peb as u64,
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => eprocess.Peb as u64,
        };
        let mut directory_table_base = match self {
            WindowsEProcess::Windows10_0_10240_16384 { eprocess } => {
                eprocess.Pcb.DirectoryTableBase
            }
            WindowsEProcess::Windows10_0_10586_0 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_14393_0 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_15063_0 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_16299_15 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_17134_1 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_17763_107 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_18362_418 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_19041_1288 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_19045_2965 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_22000_194 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_22621_382 { eprocess } => eprocess.Pcb.DirectoryTableBase,
            WindowsEProcess::Windows10_0_22631_2428 { eprocess } => eprocess.Pcb.DirectoryTableBase,
        };
        debug!("Using directory table base {:#x}", directory_table_base);

        if directory_table_base == 0 {
            directory_table_base = match self {
                WindowsEProcess::Windows10_0_10240_16384 { .. } => {
                    bail!("No UserDirectoryTableBase before 1803");
                }
                WindowsEProcess::Windows10_0_10586_0 { .. } => {
                    bail!("No UserDirectoryTableBase before 1803")
                }
                WindowsEProcess::Windows10_0_14393_0 { .. } => {
                    bail!("No UserDirectoryTableBase before 1803")
                }
                WindowsEProcess::Windows10_0_15063_0 { .. } => {
                    bail!("No UserDirectoryTableBase before 1803")
                }
                WindowsEProcess::Windows10_0_16299_15 { .. } => {
                    bail!("No UserDirectoryTableBase before 1803")
                }
                WindowsEProcess::Windows10_0_17134_1 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_17763_107 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_18362_418 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_19041_1288 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_19045_2965 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_22000_194 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_22621_382 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
                WindowsEProcess::Windows10_0_22631_2428 { eprocess } => {
                    eprocess.Pcb.UserDirectoryTableBase
                }
            };
            debug!("Invalid DTB, using user DTB: {:#x}", directory_table_base);
        }

        let mut modules = Vec::new();

        debug!("PEB ADDRESS: {:x}", peb_address);
        if peb_address != 0 {
            let peb = WindowsPeb::new_dtb(
                processor,
                major,
                minor,
                build,
                directory_table_base,
                peb_address,
            )?;
            let ldr_address = peb.ldr_address();
            let ldr = WindowsPebLdrData::new_dtb(
                processor,
                major,
                minor,
                build,
                directory_table_base,
                ldr_address,
            )?;
            let mut list_entry = ldr.in_load_order_module_list();
            let last_entry = list_entry.Blink;

            while !list_entry.Flink.is_null() {
                let ldr_data_entry = WindowsLdrDataTableEntry::new_dtb(
                    processor,
                    major,
                    minor,
                    build,
                    directory_table_base,
                    list_entry.Flink as u64,
                )?;

                let base = ldr_data_entry.dll_base();
                let size = ldr_data_entry.size_of_image();
                let full_name = ldr_data_entry.full_name_dtb(processor, directory_table_base)?;
                let base_name = ldr_data_entry.base_name_dtb(processor, directory_table_base)?;
                let debug_info = full_name
                    .split('\\')
                    .last()
                    .ok_or_else(|| anyhow!("Failed to get file name"))
                    .and_then(|fname| {
                        // No need for DTB version because kernel is always mapped
                        DebugInfo::new(
                            processor,
                            fname,
                            base,
                            download_directory.as_ref(),
                            not_found_full_name_cache,
                            user_debug_info,
                        )
                    })
                    .ok();

                modules.push(ProcessModule {
                    base,
                    size,
                    full_name,
                    base_name,
                    debug_info,
                });

                list_entry = ldr_data_entry.in_load_order_links();

                if list_entry.Flink == last_entry {
                    break;
                }
            }
        }

        Ok(modules)
    }
}

pub enum WindowsKThread {
    Windows10_0_10240_16384 {
        kthread: windows_10_0_10240_16384_x64::_KTHREAD,
    },
    Windows10_0_10586_0 {
        kthread: windows_10_0_10586_0_x64::_KTHREAD,
    },
    Windows10_0_14393_0 {
        kthread: windows_10_0_14393_0_x64::_KTHREAD,
    },
    Windows10_0_15063_0 {
        kthread: windows_10_0_15063_0_x64::_KTHREAD,
    },
    Windows10_0_16299_15 {
        kthread: windows_10_0_16299_15_x64::_KTHREAD,
    },
    Windows10_0_17134_1 {
        kthread: windows_10_0_17134_1_x64::_KTHREAD,
    },
    Windows10_0_17763_107 {
        kthread: windows_10_0_17763_107_x64::_KTHREAD,
    },
    Windows10_0_18362_418 {
        kthread: windows_10_0_18362_418_x64::_KTHREAD,
    },
    Windows10_0_19041_1288 {
        kthread: windows_10_0_19041_1288_x64::_KTHREAD,
    },
    Windows10_0_19045_2965 {
        kthread: windows_10_0_19045_2965_x64::_KTHREAD,
    },
    Windows10_0_22000_194 {
        kthread: windows_10_0_22000_194_x64::_KTHREAD,
    },
    Windows10_0_22621_382 {
        kthread: windows_10_0_22621_382_x64::_KTHREAD,
    },
    Windows10_0_22631_2428 {
        kthread: windows_10_0_22631_2428_x64::_KTHREAD,
    },
}

impl WindowsKThread {
    pub fn new(
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
        kthread_address: u64,
    ) -> Result<Self> {
        match (major, minor, build) {
            (10, 0, 10240) => {
                let kthread = read_virtual::<windows_10_0_10240_16384_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_10240_16384 { kthread })
            }
            (10, 0, 10586) => {
                let kthread =
                    read_virtual::<windows_10_0_10586_0_x64::_KTHREAD>(processor, kthread_address)?;
                Ok(WindowsKThread::Windows10_0_10586_0 { kthread })
            }
            (10, 0, 14393) => {
                let kthread =
                    read_virtual::<windows_10_0_14393_0_x64::_KTHREAD>(processor, kthread_address)?;
                Ok(WindowsKThread::Windows10_0_14393_0 { kthread })
            }
            (10, 0, 15063) => {
                let kthread =
                    read_virtual::<windows_10_0_15063_0_x64::_KTHREAD>(processor, kthread_address)?;
                Ok(WindowsKThread::Windows10_0_15063_0 { kthread })
            }
            (10, 0, 16299) => {
                let kthread = read_virtual::<windows_10_0_16299_15_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_16299_15 { kthread })
            }
            (10, 0, 17134) => {
                let kthread =
                    read_virtual::<windows_10_0_17134_1_x64::_KTHREAD>(processor, kthread_address)?;
                Ok(WindowsKThread::Windows10_0_17134_1 { kthread })
            }
            (10, 0, 17763) => {
                let kthread = read_virtual::<windows_10_0_17763_107_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_17763_107 { kthread })
            }
            (10, 0, 18362) => {
                let kthread = read_virtual::<windows_10_0_18362_418_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_18362_418 { kthread })
            }
            (10, 0, 19041) => {
                let kthread = read_virtual::<windows_10_0_19041_1288_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_19041_1288 { kthread })
            }
            (10, 0, 19045) => {
                let kthread = read_virtual::<windows_10_0_19045_2965_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_19045_2965 { kthread })
            }
            (10, 0, 22000) => {
                let kthread = read_virtual::<windows_10_0_22000_194_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_22000_194 { kthread })
            }
            (10, 0, 22621) => {
                let kthread = read_virtual::<windows_10_0_22621_382_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_22621_382 { kthread })
            }
            (10, 0, 22631) => {
                let kthread = read_virtual::<windows_10_0_22631_2428_x64::_KTHREAD>(
                    processor,
                    kthread_address,
                )?;
                Ok(WindowsKThread::Windows10_0_22631_2428 { kthread })
            }
            (_, _, _) => {
                bail!("Unsupported Windows version")
            }
        }
    }

    pub fn process(
        &self,
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
    ) -> Result<WindowsEProcess> {
        let process_address = match self {
            WindowsKThread::Windows10_0_10240_16384 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_10586_0 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_14393_0 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_15063_0 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_16299_15 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_17134_1 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_17763_107 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_18362_418 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_19041_1288 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_19045_2965 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_22000_194 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_22621_382 { kthread } => kthread.Process as u64,
            WindowsKThread::Windows10_0_22631_2428 { kthread } => kthread.Process as u64,
        };
        WindowsEProcess::new(processor, major, minor, build, process_address)
    }

    pub fn teb(
        &self,
        processor: *mut ConfObject,
        major: u32,
        minor: u32,
        build: u32,
    ) -> Result<WindowsTeb> {
        let teb_address = match self {
            WindowsKThread::Windows10_0_10240_16384 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_10586_0 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_14393_0 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_15063_0 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_16299_15 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_17134_1 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_17763_107 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_18362_418 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_19041_1288 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_19045_2965 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_22000_194 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_22621_382 { kthread } => kthread.Teb as u64,
            WindowsKThread::Windows10_0_22631_2428 { kthread } => kthread.Teb as u64,
        };
        WindowsTeb::new(processor, major, minor, build, teb_address)
    }
}
