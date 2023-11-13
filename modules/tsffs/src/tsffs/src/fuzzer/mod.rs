// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use crate::Tsffs;
use anyhow::{anyhow, Result};
use getters::Getters;
use libafl::{
    feedback_or, feedback_or_fast,
    prelude::{
        havoc_mutations, ondisk::OnDiskMetadataFormat, tokens_mutations, AFLppRedQueen, BytesInput,
        CachedOnDiskCorpus, Corpus, CorpusId, CrashFeedback, ExitKind, HasTargetBytes,
        HitcountsMapObserver, I2SRandReplace, InProcessExecutor, MaxMapFeedback, OnDiskCorpus,
        RandBytesGenerator, SimpleEventManager, SimpleMonitor, StdCmpValuesObserver,
        StdMOptMutator, StdMapObserver, StdScheduledMutator, TimeFeedback, TimeObserver,
        TimeoutExecutor, Tokens,
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
    prelude::OwnedMutSlice,
    rands::StdRand,
    tuples::{tuple_list, Merge},
    AsMutSlice, AsSlice,
};
use libafl_targets::{AFLppCmpLogObserver, AFLppCmplogTracingStage};
use simics::{api::AsConfObject, debug, info};
use std::{
    cell::RefCell, fmt::Debug, slice::from_raw_parts_mut, sync::mpsc::channel, thread::spawn,
    time::Duration,
};

pub mod tokenize;

#[derive(Getters, Clone, PartialEq, Eq)]
pub struct Testcase {
    testcase: Vec<u8>,
    cmplog: bool,
}

impl Debug for Testcase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Testcase")
            .field(
                "testcase",
                &&self.testcase[..(if self.testcase.len() < 8 {
                    self.testcase.len()
                } else {
                    8
                })],
            )
            .field("cmplog", &self.cmplog)
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShutdownMessage {}

impl Tsffs {
    const EDGES_OBSERVER_NAME: &'static str = "coverage";
    const AFLPP_CMP_OBSERVER_NAME: &'static str = "aflpp_cmplog";
    const CMPLOG_OBSERVER_NAME: &'static str = "cmplog";
    const TIME_OBSERVER_NAME: &'static str = "time";
    const TIMEOUT_FEEDBACK_NAME: &'static str = "time";
    const CORPUS_CACHE_SIZE: usize = 4096;

    /// Start the fuzzing thread.
    pub fn start_fuzzer_thread(&mut self) -> Result<()> {
        if self.fuzz_thread().is_some() {
            info!(self.as_conf_object(), "Fuzz thread already started but start_fuzzer_thread called. Returning without error.");
            // We can only start the thread once
            return Ok(());
        }

        debug!(self.as_conf_object_mut(), "Starting fuzzer thread");

        let (tx, orx) = channel::<ExitKind>();
        let (otx, rx) = channel::<Testcase>();
        let (stx, srx) = channel::<ShutdownMessage>();

        self.fuzzer_tx = Some(tx);
        self.fuzzer_rx = Some(rx);
        self.fuzzer_shutdown = Some(stx);

        let client = RefCell::new((otx, orx));
        let configuration = self.configuration().clone();
        let coverage_map = unsafe {
            from_raw_parts_mut(
                self.coverage_map_mut().as_mut_slice().as_mut_ptr(),
                Self::COVERAGE_MAP_SIZE,
            )
        };
        let aflpp_cmp_map = Box::leak(unsafe { Box::from_raw(*self.aflpp_cmp_map_ptr()) });
        let aflpp_cmp_map_dup = Box::leak(unsafe { Box::from_raw(*self.aflpp_cmp_map_ptr()) });
        let cmplog_enabled = *self.configuration().cmplog();

        // NOTE: We do *not* use `run_in_thread` because it causes the fuzzer to block when HAPs arrive
        // which prevents forward progress.
        *self.fuzz_thread_mut() = Some(spawn(move || -> Result<()> {
            let mut harness = |input: &BytesInput| {
                let testcase = input.target_bytes().as_slice().to_vec();
                // println!("Sending testcase {:?}", testcase);
                client
                    .borrow_mut()
                    .0
                    .send(Testcase {
                        testcase,
                        cmplog: false,
                    })
                    .expect("Failed to send testcase message");
                // println!("Sent testcase, waiting for status");
                let status = match client.borrow_mut().1.recv() {
                    Err(e) => panic!("Error receiving status: {e}"),
                    Ok(m) => m,
                };
                // println!("Got status: {:?}", status);

                status
            };

            let mut aflpp_cmp_harness = |input: &BytesInput| {
                let testcase = input.target_bytes().as_slice().to_vec();
                // println!("Sending testcase {:?}", testcase);
                client
                    .borrow_mut()
                    .0
                    .send(Testcase {
                        testcase,
                        cmplog: true,
                    })
                    .expect("Failed to send testcase message");
                // println!("Sent testcase, waiting for status");

                let status = match client.borrow_mut().1.recv() {
                    Err(e) => panic!("Error receiving status: {e}"),
                    Ok(m) => m,
                };
                // println!("Got status: {:?}", status);

                status
            };

            let mut tracing_harness = aflpp_cmp_harness;

            let edges_observer = HitcountsMapObserver::new(StdMapObserver::from_mut_slice(
                Self::EDGES_OBSERVER_NAME,
                OwnedMutSlice::from(coverage_map),
            ));
            let aflpp_cmp_observer =
                AFLppCmpLogObserver::new(Self::AFLPP_CMP_OBSERVER_NAME, aflpp_cmp_map, true);
            let cmplog_observer =
                StdCmpValuesObserver::new(Self::CMPLOG_OBSERVER_NAME, aflpp_cmp_map_dup, true);
            let time_observer = TimeObserver::new(Self::TIME_OBSERVER_NAME);

            let map_feedback = MaxMapFeedback::tracking(&edges_observer, true, true);
            let time_feedback = TimeFeedback::with_observer(&time_observer);

            let crash_feedback = CrashFeedback::new();
            let timeout_feedback = TimeFeedback::new(Self::TIMEOUT_FEEDBACK_NAME);

            let solutions = OnDiskCorpus::with_meta_format(
                configuration.solutions_directory(),
                OnDiskMetadataFormat::JsonPretty,
            )?;

            let corpus = CachedOnDiskCorpus::with_meta_format(
                configuration.corpus_directory(),
                Self::CORPUS_CACHE_SIZE,
                Some(OnDiskMetadataFormat::Json),
            )?;

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
            .map_err(|e| anyhow!("Couldn't initialize state: {e}"))?;

            let mut tokens = Tokens::default();
            configuration
                .token_files()
                .iter()
                .try_for_each(|f| tokens.add_from_file(f).map(|_| ()))?;
            tokens.add_tokens(configuration.tokens());
            state.add_metadata(tokens);

            let scheduler =
                IndexesLenTimeMinimizerScheduler::new(StdWeightedScheduler::with_schedule(
                    &mut state,
                    &edges_observer,
                    Some(PowerSchedule::EXPLORE),
                ));

            let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

            let monitor = {
                SimpleMonitor::new(move |s| {
                    println!("{}", s);
                })
            };

            let mut manager = SimpleEventManager::new(monitor);

            let mut executor = TimeoutExecutor::new(
                InProcessExecutor::new(
                    &mut harness,
                    tuple_list!(edges_observer, time_observer),
                    &mut fuzzer,
                    &mut state,
                    &mut manager,
                )?,
                Duration::from_secs(*configuration.executor_timeout()),
            );

            let aflpp_cmp_executor = TimeoutExecutor::new(
                InProcessExecutor::new(
                    &mut aflpp_cmp_harness,
                    tuple_list!(aflpp_cmp_observer),
                    &mut fuzzer,
                    &mut state,
                    &mut manager,
                )?,
                Duration::from_secs(*configuration.executor_timeout()),
            );

            let tracing_executor = TimeoutExecutor::new(
                InProcessExecutor::new(
                    &mut tracing_harness,
                    tuple_list!(cmplog_observer),
                    &mut fuzzer,
                    &mut state,
                    &mut manager,
                )?,
                Duration::from_secs(*configuration.executor_timeout()),
            );

            let input_to_state_stage = StdMutationalStage::new(StdScheduledMutator::new(
                tuple_list!(I2SRandReplace::new()),
            ));
            let havoc_mutational_stage = StdPowerMutationalStage::new(StdScheduledMutator::new(
                havoc_mutations().merge(tokens_mutations()),
            ));
            let mopt_mutational_stage = StdPowerMutationalStage::new(StdMOptMutator::new(
                &mut state,
                havoc_mutations().merge(tokens_mutations()),
                7,
                5,
            )?);
            let redqueen_mutational_stage =
                MultiMutationalStage::new(AFLppRedQueen::with_cmplog_options(true, true));
            let aflpp_tracing_stage = AFLppCmplogTracingStage::with_cmplog_observer_name(
                aflpp_cmp_executor,
                Self::AFLPP_CMP_OBSERVER_NAME,
            );
            let tracing_stage = TracingStage::new(tracing_executor);
            let synchronize_corpus_stage =
                SyncFromDiskStage::with_from_file(configuration.corpus_directory().clone());
            let dump_corpus_stage = DumpToDiskStage::new(
                |input: &BytesInput, _state: &_| input.target_bytes().as_slice().to_vec(),
                configuration.corpus_directory(),
                configuration.solutions_directory(),
            )?;

            if state.must_load_initial_inputs() {
                state.load_initial_inputs(
                    &mut fuzzer,
                    &mut executor,
                    &mut manager,
                    &[configuration.corpus_directory().clone()],
                )?;

                if state.corpus().count() < 1 && *configuration.generate_random_corpus() {
                    let mut generator = RandBytesGenerator::new(64);
                    state.generate_initial_inputs(
                        &mut fuzzer,
                        &mut executor,
                        &mut generator,
                        &mut manager,
                        *configuration.initial_random_corpus_size(),
                    )?;
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
                     _event_manager: &mut _,
                     corpus_id: CorpusId|
                     -> Result<bool, libafl::Error> {
                        Ok(cmplog_enabled
                            && state.corpus().get(corpus_id)?.borrow().scheduled_count() == 1)
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
                     _event_manager: &mut _,
                     _corpus_id: CorpusId|
                     -> Result<bool, libafl::Error> { Ok(cmplog_enabled) },
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

                fuzzer.fuzz_one(&mut stages, &mut executor, &mut state, &mut manager)?;
            }

            println!("Fuzzing loop exited.");

            Ok(())
        }));

        Ok(())
    }

    pub fn send_shutdown(&mut self) -> Result<()> {
        if let Some(stx) = self.fuzzer_shutdown_mut() {
            stx.send(ShutdownMessage::default())?;
        }

        Ok(())
    }

    pub fn get_testcase(&mut self) -> Result<Testcase> {
        Ok(if let Some(testcase) = self.repro_testcase() {
            Testcase {
                testcase: testcase.clone(),
                cmplog: false,
            }
        } else {
            self.fuzzer_rx_mut()
                .as_mut()
                .ok_or_else(|| anyhow!("Fuzzer receiver not set"))?
                .recv()
                .map_err(|e| anyhow!("Error receiving from fuzzer: {e}"))?
        })
    }
}

// impl Tsffs {
//     fn on_init(&mut self) -> Result<()> {
//         Ok(())
//     }
//
//     fn on_simulation_stopped(&mut self, reason: &StopReason) -> Result<()> {
//         info!(
//             self.as_conf_object(),
//             "Stopped in fuzzer with reason {:?}", reason
//         );
//         match reason {
//             StopReason::MagicStart(_) | StopReason::Start(_) => {
//                 if self.fuzz_thread().is_none() {
//                     self.start()?;
//                 }
//             }
//             StopReason::MagicStop(_) | StopReason::Stop(_) | StopReason::Solution(_) => {
//                 if let Some(tx) = self.tx().as_ref() {
//                     tx.send(ModuleMessage::Status(reason.clone()))
//                         .map_err(|e| anyhow!("Failed to send status message: {e}"))?;
//                 }
//             }
//         }
//         Ok(())
//     }
// }
