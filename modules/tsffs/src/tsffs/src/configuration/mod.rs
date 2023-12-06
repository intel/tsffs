use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use getters2::Getters;
use simics::api::{lookup_file, BreakpointId};
use simics_macro::TryIntoAttrValueTypeDict;
use typed_builder::TypedBuilder;

use crate::arch::ArchitectureHint;

impl Configuration {
    /// The timeout runs in virtual time, so a typical 5 second timeout is acceptable
    pub const DEFAULT_TIMEOUT_SECONDS: f64 = 5.0;
    /// The default start magic mnumber the fuzzer expects to be triggered, either
    /// via an in-target macro or another means.
    pub const DEFAULT_MAGIC_START: i64 = 1;
    /// The default stop magic mnumber the fuzzer expects to be triggered, either
    /// via an in-target macro or another means.
    pub const DEFAULT_MAGIC_STOP: i64 = 2;
    /// The default assert magic mnumber the fuzzer expects to be triggered, either
    /// via an in-target macro or another means.
    pub const DEFAULT_MAGIC_ASSERT: i64 = 3;
    pub const DEFAULT_CORPUS_DIRECTORY_NAME: &'static str = "corpus";
    pub const DEFAULT_SOLUTIONS_DIRECTORY_NAME: &'static str = "solutions";
    pub const DEFAULT_EXECUTOR_TIMEOUT: u64 = 60;
    pub const DEFAULT_INITIAL_RANDOM_CORPUS_SIZE: usize = 8;
    #[cfg(simics_experimental_api_snapshots)]
    pub const DEFAULT_USE_SNAPSHOTS: bool = true;
    #[cfg(not(simics_experimental_api_snapshots))]
    pub const DEFAULT_USE_SNAPSHOTS: bool = false;
}

#[derive(TypedBuilder, Getters, Debug, Clone, TryIntoAttrValueTypeDict)]
#[getters(deref, mutable)]
pub struct Configuration {
    #[builder(default = false)]
    /// Whether any breakpoint that occurs during fuzzing is treated as a fault
    all_breakpoints_are_solutions: bool,
    #[builder(default = false)]
    /// Whether any CPU exception that occurs during fuzzing is treated as a solution
    all_exceptions_are_solutions: bool,
    #[builder(default)]
    #[getters(skip_deref, clone)]
    /// The set of specific exception numbers that are treated as a solution
    exceptions: BTreeSet<i64>,
    #[builder(default)]
    #[getters(skip_deref, clone)]
    /// The set of breakpoints to treat as solutions
    breakpoints: BTreeSet<BreakpointId>,
    #[builder(default = Configuration::DEFAULT_TIMEOUT_SECONDS)]
    /// The amount of time in seconds before a testcase execution is considered "timed
    /// out" and will be treated as a solution
    timeout: f64,
    #[builder(default = false)]
    start_on_harness: bool,
    #[builder(default = false)]
    stop_on_harness: bool,
    #[builder(default = Configuration::DEFAULT_USE_SNAPSHOTS)]
    use_snapshots: bool,
    #[builder(default = Configuration::DEFAULT_MAGIC_START)]
    magic_start: i64,
    #[builder(default = Configuration::DEFAULT_MAGIC_STOP)]
    magic_stop: i64,
    #[builder(default = Configuration::DEFAULT_MAGIC_ASSERT)]
    magic_assert: i64,
    #[builder(default, setter(strip_option))]
    iterations: Option<usize>,
    #[builder(default)]
    #[getters(skip_deref, clone)]
    tokens: Vec<Vec<u8>>,
    #[builder(default = lookup_file("%simics%").expect("No simics project root found").join(Configuration::DEFAULT_CORPUS_DIRECTORY_NAME))]
    #[getters(skip_deref, clone)]
    corpus_directory: PathBuf,
    #[builder(default = lookup_file("%simics%").expect("No simics project root found").join(Configuration::DEFAULT_SOLUTIONS_DIRECTORY_NAME))]
    #[getters(skip_deref, clone)]
    solutions_directory: PathBuf,
    #[builder(default = false)]
    generate_random_corpus: bool,
    #[builder(default)]
    #[getters(skip_deref, clone)]
    token_files: Vec<PathBuf>,
    #[builder(default = Configuration::DEFAULT_EXECUTOR_TIMEOUT)]
    /// The executor timeout in seconds
    executor_timeout: u64,
    #[builder(default = Configuration::DEFAULT_INITIAL_RANDOM_CORPUS_SIZE)]
    initial_random_corpus_size: usize,
    #[builder(default = true)]
    cmplog: bool,
    #[builder(default)]
    #[getters(skip_deref, clone)]
    architecture_hints: HashMap<i32, ArchitectureHint>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self::builder().build()
    }
}
