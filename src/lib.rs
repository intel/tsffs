// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! TFFS Module for SIMICS
//!
//! # Overview
//!
//! This crate provides a client and module loadable by SIMICS to enable fuzzing on the SIMICS
//! platform. The client is intended to be used by the `simics-fuzz` crate, but it can be used
//! manually to enable additional use cases.
//!
//! # Capabilities
//!
//! The Module can:
//!
//! - Trace branch hits during an execution of a target on an x86_64 processor. These branches
//!   are traced into shared memory in the format understood by the AFL family of tools.
//! - Catch exception/fault events registered in an initial configuration or dynamically using
//!   a SIMICS Python script
//! - Catch timeout events registered in an initial configuration or dynamically using a SIMICS
//!   Python script
//! - Manage the state of a target under test by taking and restoring a snapshot of its state for
//!   deterministic snapshot fuzzing

#![deny(clippy::all)]
// NOTE: We have to do this a lot, and it sucks to have all these functions be unsafe
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![deny(clippy::unwrap_used)]
#![warn(missing_docs)]

use crate::interfaces::{config::config, fuzz::fuzz};
use crate::state::{Solution, SolutionKind};
#[cfg(not(simics_deprecated_api_rev_exec))]
use crate::util::Utils;
use anyhow::{anyhow, Result};
use arch::{Architecture, ArchitectureHint, ArchitectureOperations};
use fuzzer::{messages::FuzzerMessage, ShutdownMessage, Testcase};
use indoc::indoc;
use libafl::{
    inputs::{HasBytesVec, Input},
    prelude::ExitKind,
};
use libafl_bolts::prelude::OwnedMutSlice;
use libafl_targets::AFLppCmpLogMap;
use serde::{Deserialize, Serialize};
use simics::{
    break_simulation, class, error, free_attribute, get_class, get_interface, get_processor_number,
    info, lookup_file, object_clock, run_command, run_python, simics_init, trace, AsConfObject,
    BreakpointId, ClassCreate, ClassObjectsFinalize, ConfObject, CoreBreakpointMemopHap,
    CoreExceptionHap, CoreMagicInstructionHap, CoreSimulationStoppedHap,
    CpuInstrumentationSubscribeInterface, Event, EventClassFlag, FromConfObject, HapHandle,
    Interface, IntoAttrValueDict,
};
#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
// NOTE: save_snapshot used because it is a stable alias for both save_snapshot and take_snapshot
// which is necessary because this module is compatible with base versions which cross the
// deprecation boundary
use simics::{
    debug, restore_snapshot, save_snapshot, sys::save_flags_t, write_configuration_to_file,
};
#[cfg(not(simics_deprecated_api_rev_exec))]
use simics::{
    discard_future, restore_micro_checkpoint, save_micro_checkpoint, MicroCheckpointFlags,
};
use state::StopReason;
#[cfg(any(
    simics_experimental_api_snapshots,
    simics_experimental_api_snapshots_v2,
    simics_stable_api_snapshots
))]
use std::fs::remove_dir_all;
use std::{
    alloc::{alloc_zeroed, Layout},
    cell::OnceCell,
    collections::{hash_map::Entry, BTreeSet, HashMap, HashSet},
    fs::{write, File},
    path::PathBuf,
    ptr::null_mut,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
    time::SystemTime,
};
use tracer::tsffs::{on_instruction_after, on_instruction_before};
use typed_builder::TypedBuilder;

pub(crate) mod arch;
pub(crate) mod fuzzer;
pub(crate) mod haps;
pub(crate) mod interfaces;
pub(crate) mod log;
pub(crate) mod state;
pub(crate) mod tracer;
pub(crate) mod traits;
pub(crate) mod util;

/// The class name used for all operations interfacing with SIMICS

pub const CLASS_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(TypedBuilder, Serialize, Deserialize, Clone, Debug)]
pub(crate) struct StartBuffer {
    /// The physical address of the buffer. Must be physical, if the input address was
    /// virtual, it should be pre-translated
    pub physical_address: u64,
    /// Whether the address that translated to this physical address was virtual
    /// this should not be used or checked, it's simply informational
    pub virt: bool,
}

#[derive(TypedBuilder, Serialize, Deserialize, Clone, Debug)]
pub(crate) struct StartSize {
    #[builder(default, setter(into, strip_option))]
    /// The address of the magic start size value, and whether the address that translated
    /// to this physical address was virtual. The address must be physical.
    pub physical_address: Option<(u64, bool)>,
    #[builder(default, setter(into, strip_option))]
    // NOTE: There is no need to save the size fo the size, it must be pointer-sized.
    /// The initial size of the magic start size
    pub initial_size: Option<u64>,
}

#[class(name = "tsffs", skip_objects_finalize, attr_value)]
#[derive(AsConfObject, FromConfObject, Default, IntoAttrValueDict)]
/// The main module class for the TSFFS fuzzer, stores state and configuration information
pub(crate) struct Tsffs {
    #[class(attribute(optional, default = false))]
    /// Whether all breakpoints are treated as solutions. When set to `True`, any breakpoint
    /// which triggers a `Core_Breakpoint_Memop` HAP will be treated as a solution. This allows
    /// setting memory breakpoints on specific memory locations to trigger a solution when the
    /// memory is read, written, or executed. Not all breakpoints cause this HAP to occur.
    ///
    /// For example, to set an execution breakpoint on the address $addr:
    ///
    /// $addr = 0x100000
    /// $bp = (bp.memory.break -x $addr)
    /// @tsffs.all_breakpoints_are_solutions = True
    ///
    /// Tsffs will treat the breakpoint as a solution (along with all other
    /// breakpoints), and the fuzzer will stop when the breakpoint is hit.
    pub all_breakpoints_are_solutions: bool,
    #[class(attribute(optional, default = false))]
    /// Whether all exceptions are treated as solutions. When set to `True`, any CPU exception
    /// or interrupt which triggers a `Core_Exception` HAP will be treated as a solution. This
    /// can be useful when enabled in a callback after which any exception is considered a
    /// solution and is typically not useful when enabled during the start-up process because
    /// most processors will generate exceptions during start-up and during normal operation.
    pub all_exceptions_are_solutions: bool,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// The set of exceptions which are treated as solutions. For example on x86_64, setting:
    ///
    /// @tsffs.exceptions = [14]
    ///
    /// would treat any page fault as a solution.
    pub exceptions: BTreeSet<i64>,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// The set of breakpoints which are treated as solutions. For example, to set a solution
    /// breakpoint on the address $addr (note the breakpoint set from the Simics command is
    /// accessed through the simenv namespace):
    ///
    /// $addr = 0x100000
    /// $bp = (bp.memory.break -x $addr)
    /// @tsffs.breakpoints = [simenv.bp]
    pub breakpoints: BTreeSet<BreakpointId>,
    #[class(attribute(optional, default = 5.0))]
    /// The timeout in seconds of virtual time for each iteration of the fuzzer. If the virtual
    /// time timeout is exceeded for a single iteration, the iteration is stopped and the testcase
    /// is saved as a solution.
    pub timeout: f64,
    #[class(attribute(optional, default = 60))]
    /// The timeout in seconds of virtual time for each iteration of the fuzzer. If the virtual
    /// time timeout is exceeded for a single iteration, the iteration is stopped and the testcase
    /// is saved as a solution.
    pub executor_timeout: u64,
    #[class(attribute(optional, default = true))]
    /// Whether the fuzzer should start on compiled-in harnesses. If set to `True`, the fuzzer
    /// will start fuzzing when a harness macro is executed.
    pub start_on_harness: bool,
    #[class(attribute(optional, default = true))]
    /// Whether the fuzzer should stop on compiled-in harnesses. If set to `True`, the fuzzer
    /// will start fuzzing when a harness macro is executed.
    pub stop_on_harness: bool,
    #[class(attribute(optional, default = true))]
    /// Whether snapshots should be used. Snapshots are introduced as of Simics 6.0.173 and
    /// replace rev-exec micro checkpoints as the only method of taking full simulation
    /// snapshots as of Simics 7.0.0. If set to `True`, the fuzzer will use snapshots to
    /// restore the state of the simulation to a known state before each iteration. If set to
    /// `False` the fuzzer will use rev-exec micro checkpoints to restore the state of the
    /// simulation to a known state before each iteration. If snapshots are not supported by
    /// the version of SIMICS being used, the fuzzer will quit with an error message when this
    /// option is set.
    pub use_snapshots: bool,
    #[class(attribute(optional, default = 1))]
    /// The magic number `n` which is passed to the platform-specific magic instruction HAP
    /// by a compiled-in harness to signal that the fuzzer should start the fuzzing loop.
    ///
    /// This option is useful when fuzzing a target which has multiple start harnesses compiled
    /// into it, and the fuzzer should start on a specific harness.
    pub magic_start: i64,
    #[class(attribute(optional, default = 2))]
    /// The magic number `n` which is passed to the platform-specific magic instruction HAP
    /// by a compiled-in harness to signal that the fuzzer should stop execution of the current
    /// iteration.
    ///
    /// This option is useful when fuzzing a target which has multiple stop harnesses compiled
    /// into it, and the fuzzer should stop on a specific harness.
    pub magic_stop: i64,
    #[class(attribute(optional, default = 3))]
    /// The magic number `n` which is passed to the platform-specific magic instruction HAP
    /// by a compiled-in harness to signal that the fuzzer should stop execution of the current
    /// iteration and save the testcase as a solution.
    pub magic_assert: i64,
    #[class(attribute(optional))]
    /// The limit on the number of fuzzing iterations to execute. If set to 0, the fuzzer will
    /// run indefinitely. If set to a positive integer, the fuzzer will run until the limit is
    /// reached.
    pub iteration_limit: usize,
    #[class(attribute(optional, default = 8))]
    /// The size of the corpus to generate randomly. If `generate_random_corpus` is set to
    /// `True`, the fuzzer will generate a random corpus of this size before starting the
    /// fuzzing loop.
    pub initial_random_corpus_size: usize,
    #[class(attribute(optional, default = lookup_file("%simics%")?.join("corpus")))]
    #[attr_value(fallible)]
    /// The directory to load the corpus from and save new corpus items to. This directory
    /// may be a SIMICS relative path prefixed with "%simics%". It is an error to provide no
    /// corpus directory when `set_generate_random_corpus(True)` has not been called prior to
    /// fuzzer startup. It is also an error to provide an *empty* corpus directory without
    /// calling `set_generate_random_corpus(True)`.  If not provided, "%simics%/corpus" will
    /// be used by default.
    pub corpus_directory: PathBuf,
    #[class(attribute(optional, default = lookup_file("%simics%")?.join("solutions")))]
    #[attr_value(fallible)]
    /// The directory to save solutions to. This directory may be a SIMICS relative path
    /// prefixed with "%simics%". If not provided, "%simics%/solutions" will be used by
    /// default.
    pub solutions_directory: PathBuf,
    #[class(attribute(optional, default = false))]
    /// Whether to generate a random corpus before starting the fuzzing loop. If set to `True`,
    /// the fuzzer will generate a random corpus of size `initial_random_corpus_size` before
    /// starting the fuzzing loop. This should generally be used only for debugging and testing
    /// purposes, and is not recommended for use in production. A real corpus representative of
    /// both valid and invalid inputs should be used in production.
    pub generate_random_corpus: bool,
    #[class(attribute(optional, default = true))]
    /// Whether comparison logging should be used during fuzzing to enable value-driven
    /// mutations. If set to `True`, the fuzzer will use comparison logging to enable
    /// value-driven mutations. This should always be enabled unless the target is known to
    /// not benefit from value-driven mutations or run too slowly when solving for comparison
    /// values.
    pub cmplog: bool,
    #[class(attribute(optional, default = true))]
    /// Whether coverage reporting should be enabled. When enabled, new edge addresses will
    /// be logged.
    pub coverage_reporting: bool,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// A set of executable files to tokenize. Tokens will be extracted from these files and
    /// used to drive token mutations of testcases.
    pub token_executables: Vec<PathBuf>,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// A set of source files to tokenize. Tokens will be extracted from these files and used
    /// to drive token mutations of testcases. C source files are expected, and strings and
    /// tokens will be extracted from strings in the source files.
    pub token_src_files: Vec<PathBuf>,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// Files in the format of:
    ///
    /// x = "hello"
    /// y = "foo\x41bar"
    ///
    /// which will be used to drive token mutations of testcases.
    pub token_files: Vec<PathBuf>,
    #[class(attribute(optional))]
    #[attr_value(fallible)]
    /// Sets of tokens to use to drive token mutations of testcases. Each token set is a
    /// bytes which will be randomically inserted into testcases.
    pub tokens: Vec<Vec<u8>>,
    #[attr_value(skip)]
    /// A mapping of architecture hints from CPU index to architecture hint. This architecture
    /// hint overrides the detected architecture of the CPU core. This is useful when the
    /// architecture of the CPU core is not detected correctly, or when the architecture of the
    /// CPU core is not known at the time the fuzzer is started. Specifically, x86 cores which
    /// report their architecture as x86_64 can be overridden to x86.
    pub architecture_hints: HashMap<i32, ArchitectureHint>,
    #[class(attribute(optional, default = lookup_file("%simics%")?.join("checkpoint.ckpt")))]
    #[attr_value(fallible)]
    /// The path to the checkpoint saved prior to fuzzing when using snapshots
    pub checkpoint_path: PathBuf,
    #[class(attribute(optional, default = true))]
    pub pre_snapshot_checkpoint: bool,
    #[class(attribute(optional, default = lookup_file("%simics%")?.join("log.json")))]
    #[attr_value(fallible)]
    /// The path to the log file which will be used to log the fuzzer's output statistics
    pub log_path: PathBuf,
    #[class(attribute(optional, default = true))]
    pub log_to_file: bool,
    #[class(attribute(optional, default = false))]
    pub keep_all_corpus: bool,

    #[attr_value(skip)]
    /// Handle for the core simulation stopped hap
    stop_hap_handle: HapHandle,
    #[attr_value(skip)]
    /// Handle for the core breakpoint memop hap
    breakpoint_memop_hap_handle: HapHandle,
    #[attr_value(skip)]
    /// Handle for exception HAP
    exception_hap_handle: HapHandle,
    #[attr_value(skip)]
    /// The handle for the registered magic HAP, used to
    /// listen for magic start and stop if `start_on_harness`
    /// or `stop_on_harness` are set.
    magic_hap_handle: HapHandle,

    // Threads and message channels
    #[attr_value(skip)]
    /// Fuzzer thread
    fuzz_thread: OnceCell<JoinHandle<Result<()>>>,
    #[attr_value(skip)]
    /// Message sender to the fuzzer thread. TSFFS sends exit kinds to the fuzzer thread to
    /// report whether testcases resulted in normal exit, timeout, or solutions.
    fuzzer_tx: OnceCell<Sender<ExitKind>>,
    #[attr_value(skip)]
    /// Message receiver from the fuzzer thread. TSFFS receives new testcases and run configuration
    /// from the fuzzer thread.
    fuzzer_rx: OnceCell<Receiver<Testcase>>,
    #[attr_value(skip)]
    /// A message sender to inform the fuzzer thread that it should exit.
    fuzzer_shutdown: OnceCell<Sender<ShutdownMessage>>,
    #[attr_value(skip)]
    /// Reciever from the fuzzer thread to receive messages from the fuzzer thread
    /// including status messages and structured introspection data like new edge findings.
    fuzzer_messages: OnceCell<Receiver<FuzzerMessage>>,

    // Fuzzer coverage maps
    #[attr_value(skip)]
    /// The coverage map
    coverage_map: OnceCell<OwnedMutSlice<'static, u8>>,
    #[attr_value(skip)]
    /// A pointer to the AFL++ comparison map
    aflpp_cmp_map_ptr: OnceCell<*mut AFLppCmpLogMap>,
    #[attr_value(skip)]
    /// The owned AFL++ comparison map
    aflpp_cmp_map: OnceCell<&'static mut AFLppCmpLogMap>,
    #[attr_value(skip)]
    /// The previous location for coverage for calculating the hash of edges.
    coverage_prev_loc: u64,
    #[attr_value(skip)]
    /// The registered timeout event which is registered and used to detect timeouts in
    /// virtual time
    timeout_event: OnceCell<Event>,
    #[attr_value(skip)]
    /// The set of edges which have been seen at least once.
    edges_seen: HashSet<u64>,
    #[attr_value(skip)]
    /// A map of the new edges to their AFL indices seen since the last time the fuzzer
    /// provided an update
    edges_seen_since_last: HashMap<u64, u64>,

    #[attr_value(skip)]
    /// The name of the fuzz snapshot, if saved
    snapshot_name: OnceCell<String>,
    #[attr_value(skip)]
    /// The index of the micro checkpoint saved for the fuzzer. Only present if not using
    /// snapshots.
    micro_checkpoint_index: OnceCell<i32>,

    #[attr_value(skip)]
    /// The reason the current stop occurred
    stop_reason: Option<StopReason>,
    #[attr_value(skip)]
    /// The buffer and size information, if saved
    start_buffer: OnceCell<StartBuffer>,
    #[attr_value(skip)]
    start_size: OnceCell<StartSize>,

    #[attr_value(skip)]
    // #[builder(default = SystemTime::now())]
    /// The time the fuzzer was started at
    start_time: OnceCell<SystemTime>,

    #[attr_value(skip)]
    log: OnceCell<File>,

    #[attr_value(skip)]
    /// Whether cmplog is currently enabled
    coverage_enabled: bool,
    #[attr_value(skip)]
    /// Whether cmplog is currently enabled
    cmplog_enabled: bool,
    #[attr_value(skip)]
    /// The number of the processor which starts the fuzzing loop (via magic or manual methods)
    start_processor_number: OnceCell<i32>,
    #[attr_value(skip)]
    /// Tracked processors. This always includes the start processor, and may include
    /// additional processors that are manually added by the user
    processors: HashMap<i32, Architecture>,
    #[attr_value(skip)]
    /// A testcase to use for repro
    repro_testcase: Option<Vec<u8>>,
    #[attr_value(skip)]
    /// Whether a bookmark has been set for repro mode
    repro_bookmark_set: bool,
    #[attr_value(skip)]
    /// Whether the fuzzer is currently stopped in repro mode
    stopped_for_repro: bool,
    #[attr_value(skip)]
    /// The number of iterations which have been executed so far
    iterations: usize,
}

impl ClassObjectsFinalize for Tsffs {
    unsafe fn objects_finalized(instance: *mut ConfObject) -> simics::Result<()> {
        let tsffs: &'static mut Tsffs = instance.into();
        tsffs.stop_hap_handle = CoreSimulationStoppedHap::add_callback(
            // NOTE: Core_Simulation_Stopped is called with an object, exception and
            // error string, but the exception is always
            // SimException::SimExc_No_Exception and the error string is always
            // null_mut.
            move |_, _, _| {
                // On stops, call the module's stop callback method, which will in turn call the
                // stop callback methods on each of the module's components. The stop reason will
                // be retrieved from the module, if one is set. It is an error for the module to
                // stop itself without setting a reason
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_simulation_stopped()
                    .expect("Error calling simulation stopped callback");
            },
        )?;
        tsffs.breakpoint_memop_hap_handle =
            CoreBreakpointMemopHap::add_callback(move |trigger_obj, breakpoint_number, memop| {
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_breakpoint_memop(trigger_obj, breakpoint_number, memop)
                    .expect("Error calling breakpoint memop callback");
            })?;
        tsffs.exception_hap_handle =
            CoreExceptionHap::add_callback(move |trigger_obj, exception_number| {
                let tsffs: &'static mut Tsffs = instance.into();
                tsffs
                    .on_exception(trigger_obj, exception_number)
                    .expect("Error calling breakpoint memop callback");
            })?;
        tsffs.magic_hap_handle =
            CoreMagicInstructionHap::add_callback(move |trigger_obj, magic_number| {
                let tsffs: &'static mut Tsffs = instance.into();

                tsffs
                    .on_magic_instruction(trigger_obj, magic_number)
                    .expect("Error calling magic instruction callback");
            })?;
        tsffs
            .coverage_map
            .set(OwnedMutSlice::from(vec![0; Tsffs::COVERAGE_MAP_SIZE]))
            .map_err(|_e| anyhow!("Value already set"))?;

        tsffs
            .aflpp_cmp_map_ptr
            .set(unsafe { alloc_zeroed(Layout::new::<AFLppCmpLogMap>()) as *mut _ })
            .map_err(|_e| anyhow!("Value already set"))?;

        tsffs
            .aflpp_cmp_map
            .set(unsafe {
                &mut **tsffs
                    .aflpp_cmp_map_ptr
                    .get()
                    .expect("Value just set and known to be valid")
            })
            .map_err(|_e| anyhow!("Value already set"))?;

        tsffs
            .timeout_event
            .set(
                Event::builder()
                    .name(Tsffs::TIMEOUT_EVENT_NAME)
                    .cls(get_class(CLASS_NAME).expect("Error getting class"))
                    .flags(EventClassFlag::Sim_EC_No_Flags)
                    .build(),
            )
            .map_err(|_e| anyhow!("Value already set"))?;

        Ok(())
    }
}

impl Tsffs {
    /// The size of the coverage map in bytes
    pub const COVERAGE_MAP_SIZE: usize = 128 * 1024;
    /// The name of the registered timeout event
    pub const TIMEOUT_EVENT_NAME: &'static str = "detector_timeout_event";
    /// The name of the initial snapshot
    pub const SNAPSHOT_NAME: &'static str = "tsffs-origin-snapshot";
}

/// Implementations for controlling the simulation
impl Tsffs {
    /// Stop the simulation with a reason
    pub fn stop_simulation(&mut self, reason: StopReason) -> Result<()> {
        let break_string = reason.to_string();
        self.stop_reason = Some(reason);
        break_simulation(break_string)?;

        Ok(())
    }
}

/// Implementations for common functionality
impl Tsffs {
    /// Add a monitored processor to the simulation and whether the processor is the
    /// "start processor" which is the processor running when the fuzzing loop begins
    pub fn add_processor(&mut self, cpu: *mut ConfObject, is_start: bool) -> Result<()> {
        let cpu_number = get_processor_number(cpu)?;

        if let Entry::Vacant(e) = self.processors.entry(cpu_number) {
            let architecture = if let Some(hint) = self.architecture_hints.get(&cpu_number) {
                hint.architecture(cpu)?
            } else {
                Architecture::new(cpu)?
            };
            e.insert(architecture);
            let mut cpu_interface: CpuInstrumentationSubscribeInterface = get_interface(cpu)?;
            cpu_interface.register_instruction_after_cb(
                null_mut(),
                Some(on_instruction_after),
                self as *mut Self as *mut _,
            )?;
            cpu_interface.register_instruction_before_cb(
                null_mut(),
                Some(on_instruction_before),
                self as *mut Self as *mut _,
            )?;
        }

        if is_start {
            self.start_processor_number
                .set(cpu_number)
                .map_err(|_| anyhow!("Start processor number already set"))?;
        }

        Ok(())
    }

    /// Return a reference to the saved "start processor" if there is one. There will be no
    /// "start processor" before a start harness (manual or magic) is executed.
    pub fn start_processor(&mut self) -> Option<&mut Architecture> {
        self.start_processor_number
            .get()
            .and_then(|n| self.processors.get_mut(n))
    }
}

impl Tsffs {
    /// Save the initial snapshot using the configured method (either rev-exec micro checkpoints
    /// or snapshots)
    pub fn save_initial_snapshot(&mut self) -> Result<()> {
        if self.use_snapshots && self.snapshot_name.get().is_none() {
            #[cfg(any(
                simics_experimental_api_snapshots,
                simics_experimental_api_snapshots_v2,
                simics_stable_api_snapshots
            ))]
            {
                if self.checkpoint_path.exists() {
                    remove_dir_all(&self.checkpoint_path)?;
                }

                debug!(
                    self.as_conf_object(),
                    "Saving checkpoint to {}",
                    self.checkpoint_path.display()
                );

                if self.pre_snapshot_checkpoint {
                    write_configuration_to_file(&self.checkpoint_path, save_flags_t(0))?;
                }

                save_snapshot(Self::SNAPSHOT_NAME)?;
                self.snapshot_name
                    .set(Self::SNAPSHOT_NAME.to_string())
                    .map_err(|_| anyhow!("Snapshot name already set"))?;
            }
            #[cfg(not(any(
                simics_experimental_api_snapshots,
                simics_experimental_api_snapshots_v2,
                simics_stable_api_snapshots
            )))]
            panic!("Snapshots cannot be used without SIMICS support from recent SIMICS versions.");
        } else if !self.use_snapshots
            && self.snapshot_name.get().is_none()
            && self.micro_checkpoint_index.get().is_none()
        {
            #[cfg(not(simics_deprecated_api_rev_exec))]
            {
                save_micro_checkpoint(
                    Self::SNAPSHOT_NAME,
                    MicroCheckpointFlags::Sim_MC_ID_User | MicroCheckpointFlags::Sim_MC_Persistent,
                )?;

                self.snapshot_name
                    .set(Self::SNAPSHOT_NAME.to_string())
                    .map_err(|_| anyhow!("Snapshot name already set"))?;

                self.micro_checkpoint_index
                    .set(
                        Utils::get_micro_checkpoints()?
                            .iter()
                            .enumerate()
                            .find_map(|(i, c)| (c.name == Self::SNAPSHOT_NAME).then_some(i as i32))
                            .ok_or_else(|| {
                                anyhow!("No micro checkpoint with just-registered name found")
                            })?,
                    )
                    .map_err(|_| anyhow!("Micro checkpoint index already set"))?;
            }
            #[cfg(simics_deprecated_api_rev_exec)]
            panic!("Micro checkpoints are deprecated in SIMICS >=7.0.0 and cannot be used. Set `use_snapshots` to `true` to use snapshots instead.");
        }

        Ok(())
    }

    /// Restore the initial snapshot using the configured method (either rev-exec micro checkpoints
    /// or snapshots)
    pub fn restore_initial_snapshot(&mut self) -> Result<()> {
        if self.use_snapshots {
            #[cfg(any(
                simics_experimental_api_snapshots,
                simics_experimental_api_snapshots_v2,
                simics_stable_api_snapshots
            ))]
            restore_snapshot(Self::SNAPSHOT_NAME)?;
            #[cfg(not(any(
                simics_experimental_api_snapshots,
                simics_experimental_api_snapshots_v2,
                simics_stable_api_snapshots
            )))]
            panic!("Snapshots cannot be used without SIMICS support from recent SIMICS versions.");
        } else {
            #[cfg(not(simics_deprecated_api_rev_exec))]
            {
                restore_micro_checkpoint(*self.micro_checkpoint_index.get().ok_or_else(|| {
                    anyhow!("Not using snapshots and no micro checkpoint index present")
                })?)?;
                discard_future()?;
            }

            #[cfg(simics_deprecated_api_rev_exec)]
            panic!("Micro checkpoints are deprecated in SIMICS >=7.0.0 and cannot be used. Set `use_snapshots` to `true` to use snapshots instead.");
        }

        Ok(())
    }

    /// Whether an initial snapshot has been saved
    pub fn have_initial_snapshot(&self) -> bool {
        (self.snapshot_name.get().is_some() && self.use_snapshots)
            || (self.snapshot_name.get().is_some()
                && self.micro_checkpoint_index.get().is_some()
                && !self.use_snapshots)
    }

    /// Save a repro bookmark if one is needed
    pub fn save_repro_bookmark_if_needed(&mut self) -> Result<()> {
        if self.repro_testcase.is_some() && !self.repro_bookmark_set {
            free_attribute(run_command("set-bookmark start")?)?;
            self.repro_bookmark_set = true;
        }

        Ok(())
    }
}

impl Tsffs {
    /// Get a testcase from the fuzzer and write it to memory along with, optionally, a size
    pub fn get_and_write_testcase(&mut self) -> Result<()> {
        let testcase = self.get_testcase()?;

        if self.keep_all_corpus {
            let testcase_name = testcase.testcase.generate_name(0);
            trace!(
                self.as_conf_object(),
                "Writing testcase {}.testcase to corpus directory: {}",
                &testcase_name,
                self.corpus_directory.display()
            );

            write(
                self.corpus_directory
                    .join(format!("{}.testcase", &testcase_name)),
                testcase.testcase.bytes(),
            )?;
        }

        self.cmplog_enabled = testcase.cmplog;

        // TODO: Fix cloning - refcell?
        let start_buffer = self
            .start_buffer
            .get()
            .ok_or_else(|| anyhow!("No start buffer"))?
            .clone();

        let start_size = self
            .start_size
            .get()
            .ok_or_else(|| anyhow!("No start size"))?
            .clone();

        let start_processor = self
            .start_processor()
            .ok_or_else(|| anyhow!("No start processor"))?;

        start_processor.write_start(testcase.testcase.bytes(), &start_buffer, &start_size)?;

        Ok(())
    }

    /// Post a new timeout event on the start processor with the configured timeout in
    /// seconds
    pub fn post_timeout_event(&mut self) -> Result<()> {
        let tsffs_ptr = self.as_conf_object_mut();
        let start_processor = self
            .start_processor()
            .ok_or_else(|| anyhow!("No start processor"))?;
        let start_processor_time = start_processor.cycle().get_time()?;
        let start_processor_cpu = start_processor.cpu();
        let start_processor_clock = object_clock(start_processor_cpu)?;
        let timeout_time = self.timeout + start_processor_time;
        trace!(
            self.as_conf_object(),
            "Posting event on processor at time {} for {}s (time {})",
            start_processor_time,
            self.timeout,
            timeout_time
        );
        self.timeout_event
            .get_mut()
            .ok_or_else(|| anyhow!("No timeout event set"))?
            .post_time(
                start_processor_cpu,
                start_processor_clock,
                self.timeout,
                move |obj| {
                    let tsffs: &'static mut Tsffs = tsffs_ptr.into();
                    info!(tsffs.as_conf_object_mut(), "timeout({:#x})", obj as usize);
                    tsffs
                        .stop_simulation(StopReason::Solution(
                            Solution::builder().kind(SolutionKind::Timeout).build(),
                        ))
                        .expect("Error calling timeout callback");
                },
            )?;

        Ok(())
    }

    /// Cancel a pending timeout event, if there is one. Used when execution reaches a
    /// solution or normal stop condition before a timeout occurs.
    pub fn cancel_timeout_event(&mut self) -> Result<()> {
        if let Some(start_processor) = self.start_processor() {
            let start_processor_time = start_processor.cycle().get_time()?;
            let start_processor_cpu = start_processor.cpu();
            let start_processor_clock = object_clock(start_processor_cpu)?;
            match self
                .timeout_event
                .get()
                .ok_or_else(|| anyhow!("No timeout event set"))?
                .find_next_time(start_processor_clock, start_processor_cpu)
            {
                Ok(next_time) => trace!(
                    self.as_conf_object(),
                    "Cancelling event with next time {} (current time {})",
                    next_time,
                    start_processor_time
                ),
                // NOTE: This is not an error, it almost always means we did not find a next
                // time, which always happens if the timeout goes off.
                Err(e) => trace!(
                    self.as_conf_object(),
                    "Not cancelling event with next time due to error: {e}"
                ),
            }
            self.timeout_event
                .get()
                .ok_or_else(|| anyhow!("No timeout event set"))?
                .cancel_time(start_processor_cpu, start_processor_clock)?;
        }
        Ok(())
    }
}

#[simics_init(name = "tsffs", class = "tsffs")]
/// Initialize TSFFS
fn init() {
    let tsffs = Tsffs::create().expect("Failed to create class tsffs");
    config::register(tsffs).expect("Failed to register config interface for tsffs");
    fuzz::register(tsffs).expect("Failed to register config interface for tsffs");
    run_python(indoc! {r#"
        def init_tsffs_cmd():
            try:
                global tsffs
                tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
            except Exception as e:
                raise CliError(f"Failed to create tsffs: {e}")
            
            print("TSFFS initialized. Configure and use it as @tsffs.")
    "#})
    .expect("Failed to run python");
    run_python(indoc! {r#"
        new_command(
            "init-tsffs",
            init_tsffs_cmd,
            [],
            type = ["Fuzzing"],
            see_also = [],
            short = "Initialize the TSFFS fuzzer",
            doc = "Initialize the TSFFS fuzzer"
        )
    "#})
    .map_err(|e| {
        error!("{e}");
        e
    })
    .expect("Failed to run python");
}