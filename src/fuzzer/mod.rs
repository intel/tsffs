// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Fuzzing engine implementation, configure and run LibAFL on a separate thread

use crate::{
    fuzzer::{
        executors::inprocess::InProcessExecutor, feedbacks::ReportingMapFeedback,
        messages::FuzzerMessage,
    },
    Tsffs,
};
use anyhow::{anyhow, Result};
use libafl::{
    feedback_or, feedback_or_fast,
    inputs::{HasBytesVec, Input},
    prelude::{
        havoc_mutations, ondisk::OnDiskMetadataFormat, tokens_mutations, AFLppRedQueen, BytesInput,
        CachedOnDiskCorpus, Corpus, CrashFeedback, ExitKind, HasCurrentCorpusIdx, HasTargetBytes,
        HitcountsMapObserver, I2SRandReplace, MaxMapFeedback, OnDiskCorpus, RandBytesGenerator,
        SimpleEventManager, SimpleMonitor, StdCmpValuesObserver, StdMOptMutator, StdMapObserver,
        StdScheduledMutator, TimeFeedback, TimeObserver, Tokens,
    },
    schedulers::{
        powersched::PowerSchedule, IndexesLenTimeMinimizerScheduler, StdWeightedScheduler,
    },
    stages::{
        mutational::MultiMutationalStage, CalibrationStage, ColorizationStage, DumpToDiskStage,
        GeneralizationStage, IfStage, StdMutationalStage, StdPowerMutationalStage,
        SyncFromDiskStage, TracingStage,
    },
    state::{HasCorpus, HasMetadata, StdState},
    Fuzzer, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    prelude::{OwnedMutSlice, OwnedRefMut},
    rands::StdRand,
    tuples::{tuple_list, Merge},
    AsMutSlice, AsSlice,
};
use libafl_targets::{AFLppCmpLogObserver, AFLppCmplogTracingStage};
use simics::{api::AsConfObject, debug, trace, warn};
use std::{
    cell::RefCell, fmt::Debug, fs::write, io::stderr, slice::from_raw_parts_mut,
    sync::mpsc::channel, thread::spawn,
};
use tokenize::{tokenize_executable_file, tokenize_src_file};
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{
    filter::filter_fn, fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, Layer,
};

pub mod executors;
pub mod feedbacks;
pub mod messages;
pub mod tokenize;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Testcase {
    pub testcase: BytesInput,
    pub cmplog: bool,
}

impl Debug for Testcase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Testcase")
            .field(
                "testcase",
                &format!(
                    "{:?}{} ({} bytes)",
                    &self.testcase.bytes()[..(if self.testcase.bytes().len() < 32 {
                        self.testcase.bytes().len()
                    } else {
                        32
                    })],
                    if self.testcase.bytes().len() >= 32 {
                        "..."
                    } else {
                        ""
                    },
                    self.testcase.bytes().len()
                ),
            )
            .field("cmplog", &self.cmplog)
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShutdownMessage {}

impl Tsffs {
    const EDGES_OBSERVER_NAME: &'static str = "coverage";
    const AFLPP_CMP_OBSERVER_NAME: &'static str = "aflpp_cmplog";
    const CMPLOG_OBSERVER_NAME: &'static str = "cmplog";
    const TIME_OBSERVER_NAME: &'static str = "time";
    const TIMEOUT_FEEDBACK_NAME: &'static str = "time";
    const CORPUS_CACHE_SIZE: usize = 4096;

    /// Start the fuzzing thread.
    pub fn start_fuzzer_thread(&mut self) -> Result<()> {
        if self.fuzz_thread.get().is_some() {
            warn!(self.as_conf_object(), "Fuzz thread already started but start_fuzzer_thread called. Returning without error.");
            // We can only start the thread once
            return Ok(());
        }

        debug!(self.as_conf_object_mut(), "Starting fuzzer thread");

        let (tx, orx) = channel::<ExitKind>();
        let (otx, rx) = channel::<Testcase>();
        let (stx, srx) = channel::<ShutdownMessage>();
        let (mtx, mrx) = channel::<FuzzerMessage>();

        self.fuzzer_tx
            .set(tx)
            .map_err(|_| anyhow!("Fuzzer sender already set"))?;
        self.fuzzer_rx
            .set(rx)
            .map_err(|_| anyhow!("Fuzzer receiver already set"))?;
        self.fuzzer_shutdown
            .set(stx)
            .map_err(|_| anyhow!("Fuzzer shutdown sender already set"))?;
        self.fuzzer_messages
            .set(mrx)
            .map_err(|_| anyhow!("Fuzzer messages receiver already set"))?;

        let client = RefCell::new((otx, orx));

        let coverage_map = unsafe {
            from_raw_parts_mut(
                self.coverage_map
                    .get_mut()
                    .ok_or_else(|| anyhow!("Coverage map not set"))?
                    .as_mut_slice()
                    .as_mut_ptr(),
                Self::COVERAGE_MAP_SIZE,
            )
        };

        let aflpp_cmp_map = Box::leak(unsafe {
            Box::from_raw(
                *self
                    .aflpp_cmp_map_ptr
                    .get()
                    .ok_or_else(|| anyhow!("Comparison map pointer not set"))?,
            )
        });

        let aflpp_cmp_map_dup = Box::leak(unsafe {
            Box::from_raw(
                *self
                    .aflpp_cmp_map_ptr
                    .get()
                    .ok_or_else(|| anyhow!("Comparison map pointer not set"))?,
            )
        });

        let cmplog_enabled = self.cmplog;
        let corpus_directory = self.corpus_directory.clone();
        let solutions_directory = self.solutions_directory.clone();
        let executable_tokens = self
            .token_executables
            .iter()
            .map(tokenize_executable_file)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let src_file_tokens = self
            .token_src_files
            .iter()
            .map(|f| {
                tokenize_src_file(f)
                    .map(|t| t.iter().map(|s| s.as_bytes().to_vec()).collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let token_files = self.token_files.clone();
        let input_tokens = self.tokens.clone();
        let generate_random_corpus = self.generate_random_corpus;
        let initial_random_corpus_size = self.initial_random_corpus_size;
        let debug_log_libafl = self.debug_log_libafl;
        let initial_contents = self
            .use_initial_as_corpus
            .then(|| {
                self.start_info
                    .get()
                    .map(|si| BytesInput::new(si.contents.clone()))
            })
            .flatten();

        // NOTE: We do *not* use `run_in_thread` because it causes the fuzzer to block when HAPs arrive
        // which prevents forward progress.
        self.fuzz_thread
            .set(spawn(move || -> Result<()> {
                if debug_log_libafl {
                    let reg = registry().with({
                        fmt::layer()
                            .compact()
                            .with_thread_ids(true)
                            .with_thread_names(true)
                            .with_writer(stderr)
                            .with_filter(LevelFilter::TRACE)
                            .with_filter(filter_fn(|metadata| {
                                // LLMP absolutely spams the log when tracing
                                !(metadata.target() == "libafl_bolts::llmp"
                                    && matches!(metadata.level(), &Level::TRACE))
                            }))
                    });

                    reg.try_init()
                        .map_err(|e| {
                            eprintln!("Could not install tracing subscriber: {}", e);
                            e
                        })
                        .ok();
                }

                let mut harness = |input: &BytesInput| {
                    let testcase = BytesInput::new(input.target_bytes().as_slice().to_vec());
                    client
                        .borrow_mut()
                        .0
                        .send(Testcase {
                            testcase,
                            cmplog: false,
                        })
                        .expect("Failed to send testcase message");

                    let status = match client.borrow_mut().1.recv() {
                        Err(e) => panic!("Error receiving status: {e}"),
                        Ok(m) => m,
                    };

                    status
                };

                let mut aflpp_cmp_harness = |input: &BytesInput| {
                    let testcase = BytesInput::new(input.target_bytes().as_slice().to_vec());
                    client
                        .borrow_mut()
                        .0
                        .send(Testcase {
                            testcase,
                            cmplog: true,
                        })
                        .expect("Failed to send testcase message");

                    let status = match client.borrow_mut().1.recv() {
                        Err(e) => panic!("Error receiving status: {e}"),
                        Ok(m) => m,
                    };

                    status
                };

                let mut tracing_harness = aflpp_cmp_harness;

                let edges_observer = HitcountsMapObserver::new(StdMapObserver::from_mut_slice(
                    Self::EDGES_OBSERVER_NAME,
                    OwnedMutSlice::from(coverage_map),
                ));

                let aflpp_cmp_observer = AFLppCmpLogObserver::new(
                    Self::AFLPP_CMP_OBSERVER_NAME,
                    OwnedRefMut::Ref(aflpp_cmp_map),
                    true,
                );

                let cmplog_observer = StdCmpValuesObserver::new(
                    Self::CMPLOG_OBSERVER_NAME,
                    OwnedRefMut::Ref(aflpp_cmp_map_dup),
                    true,
                );
                let time_observer = TimeObserver::new(Self::TIME_OBSERVER_NAME);

                let map_feedback = ReportingMapFeedback::new(
                    MaxMapFeedback::tracking(&edges_observer, true, true),
                    mtx.clone(),
                );
                let time_feedback = TimeFeedback::with_observer(&time_observer);

                let crash_feedback = CrashFeedback::new();
                let timeout_feedback = TimeFeedback::new(Self::TIMEOUT_FEEDBACK_NAME);

                let solutions = OnDiskCorpus::with_meta_format(
                    solutions_directory.clone(),
                    OnDiskMetadataFormat::JsonPretty,
                )
                .map_err(|e| {
                    eprintln!("Failed to initialize solutions corpus: {e}");
                    anyhow!("Failed to initialize solutions corpus: {e}")
                })?;

                let corpus = CachedOnDiskCorpus::with_meta_format(
                    corpus_directory.clone(),
                    Self::CORPUS_CACHE_SIZE,
                    Some(OnDiskMetadataFormat::Json),
                )
                .map_err(|e| {
                    eprintln!("Failed to initialize corpus: {e}");
                    anyhow!("Failed to initialize corpus: {e}")
                })?;

                // NOTE: Initialize these here before we move the feedbacks
                let calibration_stage = CalibrationStage::new(&map_feedback);
                let colorization_stage = ColorizationStage::new(&edges_observer);
                let generalization_stage = GeneralizationStage::new(&edges_observer);

                let mut feedback = feedback_or!(map_feedback, time_feedback);
                let mut objective = feedback_or_fast!(crash_feedback, timeout_feedback);

                let mut state = StdState::new(
                    StdRand::with_seed(current_nanos()),
                    corpus,
                    solutions,
                    &mut feedback,
                    &mut objective,
                )
                .map_err(|e| {
                    eprintln!("Couldn't initialize fuzzer state: {e}");
                    anyhow!("Couldn't initialize state: {e}")
                })?;

                let mut tokens = Tokens::default().add_from_files(token_files)?;

                tokens.add_tokens(executable_tokens);
                tokens.add_tokens(src_file_tokens);
                tokens.add_tokens(input_tokens);

                state.add_metadata(tokens);

                let scheduler =
                    IndexesLenTimeMinimizerScheduler::new(StdWeightedScheduler::with_schedule(
                        &mut state,
                        &edges_observer,
                        Some(PowerSchedule::EXPLORE),
                    ));

                let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

                let monitor = {
                    let mtx = mtx.clone();
                    SimpleMonitor::new(move |s| {
                        mtx.send(FuzzerMessage::String(s.to_string()))
                            .expect("Failed to send monitor message");
                    })
                };

                let mut manager = SimpleEventManager::new(monitor);

                let mut executor = InProcessExecutor::new(
                    &mut harness,
                    tuple_list!(edges_observer, time_observer),
                    &mut fuzzer,
                    &mut manager,
                )
                .map_err(|e| {
                    eprintln!("Couldn't initialize fuzzer executor: {e}");
                    anyhow!("Couldn't initialize fuzzer executor: {e}")
                })?;

                let aflpp_cmp_executor = InProcessExecutor::new(
                    &mut aflpp_cmp_harness,
                    tuple_list!(aflpp_cmp_observer),
                    &mut fuzzer,
                    &mut manager,
                )
                .map_err(|e| {
                    eprintln!("Couldn't initialize fuzzer AFL++ cmplog executor: {e}");
                    anyhow!("Couldn't initialize fuzzer AFL++ cmplog executor: {e}")
                })?;

                let tracing_executor = InProcessExecutor::new(
                    &mut tracing_harness,
                    tuple_list!(cmplog_observer),
                    &mut fuzzer,
                    &mut manager,
                )
                .map_err(|e| {
                    eprintln!("Couldn't initialize fuzzer AFL++ cmplog executor: {e}");
                    anyhow!("Couldn't initialize fuzzer AFL++ cmplog executor: {e}")
                })?;

                let input_to_state_stage = StdMutationalStage::new(StdScheduledMutator::new(
                    tuple_list!(I2SRandReplace::new()),
                ));
                let havoc_mutational_stage = StdPowerMutationalStage::new(
                    StdScheduledMutator::new(havoc_mutations().merge(tokens_mutations())),
                );
                let mopt_mutational_stage = StdPowerMutationalStage::new(
                    StdMOptMutator::new(
                        &mut state,
                        havoc_mutations().merge(tokens_mutations()),
                        7,
                        5,
                    )
                    .map_err(|e| {
                        eprintln!("Couldn't initialize fuzzer MOpt mutator: {e}");
                        anyhow!("Couldn't initialize fuzzer MOpt mutator: {e}")
                    })?,
                );
                let redqueen_mutational_stage =
                    MultiMutationalStage::new(AFLppRedQueen::with_cmplog_options(true, true));
                let aflpp_tracing_stage = AFLppCmplogTracingStage::with_cmplog_observer_name(
                    aflpp_cmp_executor,
                    Self::AFLPP_CMP_OBSERVER_NAME,
                );
                let tracing_stage = TracingStage::new(tracing_executor);
                let synchronize_corpus_stage =
                    SyncFromDiskStage::with_from_file(corpus_directory.clone());
                let dump_corpus_stage = DumpToDiskStage::new(
                    |input: &BytesInput, _state: &_| input.target_bytes().as_slice().to_vec(),
                    corpus_directory.clone(),
                    solutions_directory.clone(),
                )
                .map_err(|e| {
                    eprintln!("Couldn't initialize fuzzer dump to disk stage: {e}");
                    anyhow!("Couldn't initialize fuzzer dump to disk stage: {e}")
                })?;

                if let Some(contents) = initial_contents {
                    write(
                        corpus_directory.join(contents.generate_name(0)),
                        contents.bytes(),
                    )?;
                }

                if state.must_load_initial_inputs() {
                    state
                        .load_initial_inputs(
                            &mut fuzzer,
                            &mut executor,
                            &mut manager,
                            &[corpus_directory.clone()],
                        )
                        .map_err(|e| {
                            eprintln!(
                                "Error loading initial inputs from {corpus_directory:?}: {e}"
                            );
                            anyhow!("Error loading initial inputs from {corpus_directory:?}: {e}")
                        })?;

                    if state.corpus().count() < 1 && generate_random_corpus {
                        let mut generator = RandBytesGenerator::new(64);
                        state
                            .generate_initial_inputs(
                                &mut fuzzer,
                                &mut executor,
                                &mut generator,
                                &mut manager,
                                initial_random_corpus_size,
                            )
                            .map_err(|e| {
                                eprintln!("Error generating random inputs: {e}");
                                anyhow!("Error generating random inputs: {e}")
                            })?;
                    }
                }

                if state.corpus().count() < 1 {
                    panic!(
                        "No interesting cases found from inputs! This may mean \
                            your harness is incorrect (check your arguments), your inputs \
                            are not triggering new code paths, or all inputs are causing \
                            crashes.",
                    );
                }

                let mut stages = tuple_list!(
                    calibration_stage,
                    generalization_stage,
                    IfStage::new(
                        |_fuzzer: &mut _,
                         _executor: &mut _,
                         state: &mut StdState<_, CachedOnDiskCorpus<_>, _, _>,
                         _event_manager: &mut _|
                         -> Result<bool, libafl::Error> {
                            Ok(cmplog_enabled
                                && state
                                    .corpus()
                                    .get(
                                        state
                                            .current_corpus_idx()
                                            .map_err(|e| {
                                                eprintln!(
                                                    "Error getting current corpus index: {e}"
                                                );
                                                // libafl::Error::unkown(format!(
                                                //     "Error getting current corpus index: {e}"
                                                // ))
                                                e
                                            })?
                                            .ok_or_else(|| {
                                                eprintln!("No current corpus index");

                                                libafl::Error::unknown("No current corpus index")
                                            })?,
                                    )
                                    .map_err(|e| {
                                        eprintln!("Error getting current corpus entry: {e}");
                                        e
                                    })?
                                    .borrow()
                                    .scheduled_count()
                                    == 1)
                        },
                        tuple_list!(
                            colorization_stage,
                            aflpp_tracing_stage,
                            redqueen_mutational_stage
                        )
                    ),
                    IfStage::new(
                        |_fuzzer: &mut _,
                         _executor: &mut _,
                         _state: &mut StdState<_, CachedOnDiskCorpus<_>, _, _>,
                         _event_manager: &mut _|
                         -> Result<bool, libafl::Error> {
                            Ok(cmplog_enabled)
                        },
                        tuple_list!(tracing_stage, input_to_state_stage)
                    ),
                    havoc_mutational_stage,
                    mopt_mutational_stage,
                    dump_corpus_stage,
                    synchronize_corpus_stage,
                );

                loop {
                    // Check if we have a message to shut down, and if so, exit.
                    if let Ok(_msg) = srx.try_recv() {
                        break;
                    }

                    fuzzer
                        .fuzz_one(&mut stages, &mut executor, &mut state, &mut manager)
                        .map_err(|e| {
                            eprintln!("Error running iteration of fuzzing loop: {e}");
                            anyhow!("Error running iteration of fuzzing loop: {e}")
                        })?;
                }

                println!("Fuzzing loop exited.");

                Ok(())
            }))
            .map_err(|_| anyhow!("Fuzzer thread already set"))?;

        Ok(())
    }

    pub fn send_shutdown(&mut self) -> Result<()> {
        if let Some(stx) = self.fuzzer_shutdown.get_mut() {
            stx.send(ShutdownMessage::default())?;
        }

        Ok(())
    }

    pub fn get_testcase(&mut self) -> Result<Testcase> {
        let testcase = if let Some(testcase) = self.repro_testcase.as_ref() {
            debug!(self.as_conf_object(), "Using repro testcase");
            Testcase {
                testcase: BytesInput::new(testcase.clone()),
                cmplog: false,
            }
        } else {
            self.fuzzer_rx
                .get_mut()
                .ok_or_else(|| anyhow!("Fuzzer receiver not set"))?
                .recv()
                .map_err(|e| anyhow!("Error receiving from fuzzer: {e}"))?
        };

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

        debug!(self.as_conf_object(), "Testcase: {testcase:?}");

        Ok(testcase)
    }
}
