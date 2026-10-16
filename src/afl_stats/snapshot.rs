// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! The counts the fuzzer thread derives from the LibAFL `State`, and the lock-guarded cell
//! they are published through. Nothing here depends on LibAFL.

use std::sync::{Arc, Mutex};

/// Counts derived from the fuzzer `State` by `TsffsAflStatsStage`. This holds counts only,
/// never a copy of the history map, so copying it under the lock is cheap.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FuzzerSnapshot {
    /// Entries in the corpus
    pub corpus_count: u64,
    /// Corpus entries that are favored by the scheduler
    pub corpus_favored: u64,
    /// Corpus entries found by this fuzzer rather than loaded at the start
    pub corpus_found: u64,
    /// Edges: occupied slots in the map feedback's history map
    pub edges_found: u64,
    /// Occupied slots that calibration found to be unstable
    pub var_byte_count: u64,
    /// Percentage of occupied slots that are stable
    pub stability: f64,
    /// Completed passes over the corpus
    pub cycles_done: u64,
    /// Passes over the corpus since the corpus last grew
    pub cycles_wo_finds: u64,
    /// Corpus entries the scheduler has not picked yet
    pub pending_total: u64,
    /// Favored corpus entries the scheduler has not picked yet
    pub pending_favs: u64,
    /// Index of the corpus entry being fuzzed, or 0 if there is none
    pub cur_item: u64,
    /// The largest lineage depth of any corpus entry
    pub max_depth: u64,
}

/// The latest [`FuzzerSnapshot`], shared between the fuzzer thread (which publishes) and the
/// Simics side (which reads on its own cadence). Only the latest value matters, so this is a
/// cell and not a channel.
#[derive(Debug, Clone, Default)]
pub(crate) struct SharedSnapshot {
    /// The generation of the latest snapshot, bumped on every publish, and the snapshot
    inner: Arc<Mutex<(u64, Option<FuzzerSnapshot>)>>,
}

impl SharedSnapshot {
    /// Replace the latest snapshot
    pub(crate) fn publish(&self, snapshot: FuzzerSnapshot) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.0 += 1;
        guard.1 = Some(snapshot);
    }

    /// The latest snapshot and its generation, if it is newer than `seen`
    pub(crate) fn newer_than(&self, seen: u64) -> Option<(u64, FuzzerSnapshot)> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match &guard.1 {
            Some(snapshot) if guard.0 > seen => Some((guard.0, snapshot.clone())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_than_only_reports_unseen_generations() {
        let shared = SharedSnapshot::default();
        assert_eq!(shared.newer_than(0), None);

        shared.publish(FuzzerSnapshot {
            corpus_count: 3,
            ..Default::default()
        });
        let (generation, snapshot) = shared.newer_than(0).expect("published");
        assert_eq!(snapshot.corpus_count, 3);
        assert_eq!(shared.newer_than(generation), None);

        shared.publish(FuzzerSnapshot {
            corpus_count: 4,
            ..Default::default()
        });
        let (next, snapshot) = shared.newer_than(generation).expect("republished");
        assert!(next > generation);
        assert_eq!(snapshot.corpus_count, 4);
    }

    #[test]
    fn clones_share_one_cell() {
        let shared = SharedSnapshot::default();
        let publisher = shared.clone();
        publisher.publish(FuzzerSnapshot::default());
        assert!(shared.newer_than(0).is_some());
    }
}
