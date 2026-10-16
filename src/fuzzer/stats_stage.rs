// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! A LibAFL stage that derives AFL++ statistics from the fuzzer `State` and publishes them for
//! the Simics-side writer.
//!
//! It is deliberately a thin adapter: it pulls raw inputs out of `State` and hands them to the
//! pure functions in [`crate::afl_stats::derive`]. Field names and logic mirror LibAFL's own
//! `AflStatsStage` so a future LibAFL upgrade can replace this stage with upstream's. The
//! pinned revision's upstream stage only fires a JSON user stat and writes no files.

use crate::afl_stats::{
    derive::{
        cycles_without_finds, scan_queue, stability_pct, unstable_filled_count, EntryFacts,
        QueueScan,
    },
    snapshot::{FuzzerSnapshot, SharedSnapshot},
};
use libafl::{
    corpus::{testcase::SchedulerTestcaseMetadata, Corpus, HasCurrentCorpusIdx},
    feedbacks::MapFeedbackMetadata,
    schedulers::{minimizer::IsFavoredMetadata, powersched::SchedulerMetadata},
    stages::{calibrate::UnstableEntriesMetadata, Stage},
    state::{HasCorpus, HasMetadata, HasNamedMetadata, UsesState},
    Error,
};
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

/// Derives AFL++ statistics from `State` at the end of every fuzzing round
pub(crate) struct TsffsAflStatsStage<E, EM, Z> {
    /// The name of the map feedback whose history map defines Edges
    map_name: String,
    shared: SharedSnapshot,
    /// Entries in the corpus before the first fuzzing round, which were not found by fuzzing
    initial_corpus_count: u64,
    /// The scan over every corpus entry runs at most this often
    scan_interval: Duration,
    last_scan: Option<Instant>,
    /// The result of the latest scan, reused until the next one is due
    queue: QueueScan,
    /// The corpus size at the previous round, to notice growth
    last_corpus_count: usize,
    /// The queue cycle count when the corpus last grew
    cycles_at_last_find: u64,
    phantom: PhantomData<(E, EM, Z)>,
}

impl<E, EM, Z> TsffsAflStatsStage<E, EM, Z> {
    pub(crate) fn new(
        map_name: String,
        shared: SharedSnapshot,
        initial_corpus_count: u64,
        scan_interval: Duration,
    ) -> Self {
        Self {
            map_name,
            shared,
            initial_corpus_count,
            scan_interval,
            last_scan: None,
            queue: QueueScan::default(),
            last_corpus_count: initial_corpus_count as usize,
            cycles_at_last_find: 0,
            phantom: PhantomData,
        }
    }
}

impl<E, EM, Z> UsesState for TsffsAflStatsStage<E, EM, Z>
where
    E: UsesState,
{
    type State = E::State;
}

impl<E, EM, Z> Stage<E, EM, Z> for TsffsAflStatsStage<E, EM, Z>
where
    E: UsesState,
    EM: UsesState<State = E::State>,
    Z: UsesState<State = E::State>,
    E::State: HasCorpus + HasMetadata + HasNamedMetadata + HasCurrentCorpusIdx,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut E::State,
        _manager: &mut EM,
    ) -> Result<(), Error> {
        // Edges are occupied slots in the map feedback's history map
        let Some(history) = state
            .named_metadata_map()
            .get::<MapFeedbackMetadata<u8>>(&self.map_name)
            .map(|m| m.history_map.as_slice())
        else {
            return Ok(());
        };

        let is_filled = |i: usize| history.get(i).is_some_and(|&slot| slot != 0);
        let filled = history.iter().filter(|&&slot| slot != 0).count();

        let unstable_filled = state
            .metadata_map()
            .get::<UnstableEntriesMetadata>()
            .map(|m| unstable_filled_count(m.unstable_entries().iter().copied(), is_filled));

        let corpus_count = state.corpus().count();

        if self
            .last_scan
            .is_none_or(|last| last.elapsed() >= self.scan_interval)
        {
            self.queue = scan_queue(state.corpus().ids().filter_map(|id| {
                let testcase = state.corpus().get(id).ok()?.borrow();
                Some(EntryFacts {
                    scheduled: testcase.scheduled_count() > 0,
                    favored: testcase.has_metadata::<IsFavoredMetadata>(),
                    depth: testcase
                        .metadata_map()
                        .get::<SchedulerTestcaseMetadata>()
                        .map_or(0, |m| m.depth()),
                })
            }));
            self.last_scan = Some(Instant::now());
        }

        let cycles_done = state
            .metadata_map()
            .get::<SchedulerMetadata>()
            .map_or(0, |m| m.queue_cycles());

        if corpus_count > self.last_corpus_count {
            self.cycles_at_last_find = cycles_done;
        }
        self.last_corpus_count = corpus_count;

        // A missing current index is normal between rounds and is not an error
        let cur_item = state
            .current_corpus_idx()
            .ok()
            .flatten()
            .map_or(0, |id| usize::from(id) as u64);

        self.shared.publish(FuzzerSnapshot {
            corpus_count: corpus_count as u64,
            corpus_favored: self.queue.corpus_favored,
            corpus_found: (corpus_count as u64).saturating_sub(self.initial_corpus_count),
            edges_found: filled as u64,
            var_byte_count: unstable_filled.unwrap_or(0) as u64,
            stability: stability_pct(filled, unstable_filled),
            cycles_done,
            cycles_wo_finds: cycles_without_finds(cycles_done, self.cycles_at_last_find),
            pending_total: self.queue.pending_total,
            pending_favs: self.queue.pending_favs,
            cur_item,
            max_depth: self.queue.max_depth,
        });

        Ok(())
    }

    #[inline]
    fn restart_progress_should_run(&mut self, _state: &mut Self::State) -> Result<bool, Error> {
        // Not running the target, so there is nothing to restore after a crash or timeout
        Ok(true)
    }

    #[inline]
    fn clear_restart_progress(&mut self, _state: &mut Self::State) -> Result<(), Error> {
        Ok(())
    }
}
