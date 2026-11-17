// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Pure derivations used by `TsffsAflStatsStage`. No LibAFL types appear in these signatures,
//! so they are tested with plain numbers.

/// Percentage of occupied map slots that are stable, as AFL++ computes it:
/// `(filled - unstable) / filled`. `unstable_filled` counts only unstable entries that are also
/// filled slots, so the result never goes below 0%. It is 100% until calibration has produced
/// unstable-entries metadata (`None`), and when nothing is filled.
pub(crate) fn stability_pct(filled: usize, unstable_filled: Option<usize>) -> f64 {
    match unstable_filled {
        Some(unstable) if filled > 0 => {
            let unstable = unstable.min(filled);
            (filled - unstable) as f64 / filled as f64 * 100.0
        }
        _ => 100.0,
    }
}

/// Percentage of the coverage map occupied by Edges, never above 100%
pub(crate) fn coverage_pct(filled: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (filled as f64 / total as f64 * 100.0).min(100.0)
    }
}

/// How many of `unstable` entries are filled slots of the history map. `is_filled` reports
/// whether an index is occupied; indices outside the map are not filled.
pub(crate) fn unstable_filled_count(
    unstable: impl IntoIterator<Item = usize>,
    is_filled: impl Fn(usize) -> bool,
) -> usize {
    unstable.into_iter().filter(|&i| is_filled(i)).count()
}

/// What a scan of the corpus needs to know about one entry
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntryFacts {
    /// Whether the scheduler has picked this entry at least once
    pub scheduled: bool,
    /// Whether the scheduler considers this entry favored
    pub favored: bool,
    /// How many mutations separate this entry from an initial input
    pub depth: u64,
}

/// Queue-wide counts derived from one scan of the corpus
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct QueueScan {
    /// Entries the scheduler has not picked yet
    pub pending_total: u64,
    /// Favored entries the scheduler has not picked yet. This is not `entries - favored`, which
    /// is what LibAFL 0.11's own stats stage computes for its `pend_fav` figure.
    pub pending_favs: u64,
    /// Entries the scheduler considers favored, whether or not they have been picked
    pub corpus_favored: u64,
    /// The largest lineage depth of any entry
    pub max_depth: u64,
}

/// Count pending, favored and deepest entries in one pass
pub(crate) fn scan_queue(entries: impl IntoIterator<Item = EntryFacts>) -> QueueScan {
    entries
        .into_iter()
        .fold(QueueScan::default(), |mut scan, entry| {
            if !entry.scheduled {
                scan.pending_total += 1;
                if entry.favored {
                    scan.pending_favs += 1;
                }
            }
            if entry.favored {
                scan.corpus_favored += 1;
            }
            scan.max_depth = scan.max_depth.max(entry.depth);
            scan
        })
}

/// Completed passes over the corpus since the corpus last grew
pub(crate) fn cycles_without_finds(cycles_done: u64, cycles_at_last_find: u64) -> u64 {
    cycles_done.saturating_sub(cycles_at_last_find)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(scheduled: bool, favored: bool, depth: u64) -> EntryFacts {
        EntryFacts {
            scheduled,
            favored,
            depth,
        }
    }

    #[test]
    fn pending_counts_unscheduled_entries_and_the_favored_among_them() {
        let scan = scan_queue([
            entry(true, true, 0),
            entry(false, true, 2),
            entry(false, false, 5),
            entry(true, false, 1),
            entry(false, true, 3),
        ]);
        assert_eq!(scan.pending_total, 3);
        assert_eq!(scan.pending_favs, 2);
        assert_eq!(scan.corpus_favored, 3);
        assert_eq!(scan.max_depth, 5);
    }

    #[test]
    fn pending_favs_is_not_corpus_size_minus_favored() {
        // Upstream LibAFL 0.11 reports `corpus_size - is_favored_size` for this figure, which
        // here would be 4 - 1 = 3. Only the single favored, unscheduled entry is pending.
        let scan = scan_queue([
            entry(true, true, 0),
            entry(true, false, 0),
            entry(true, false, 0),
            entry(false, false, 0),
        ]);
        assert_eq!(scan.pending_favs, 0);
        assert_eq!(scan.pending_total, 1);
    }

    #[test]
    fn an_empty_corpus_scans_to_zero() {
        assert_eq!(scan_queue([]), QueueScan::default());
    }

    #[test]
    fn cycles_without_finds_counts_from_the_last_growth() {
        assert_eq!(cycles_without_finds(7, 4), 3);
        assert_eq!(cycles_without_finds(4, 4), 0);
        assert_eq!(cycles_without_finds(2, 9), 0);
    }

    #[test]
    fn stability_is_unstable_relative_to_filled() {
        assert_eq!(stability_pct(200, Some(50)), 75.0);
        assert_eq!(stability_pct(200, Some(0)), 100.0);
        assert_eq!(stability_pct(200, Some(200)), 0.0);
    }

    #[test]
    fn stability_is_full_without_calibration_data() {
        assert_eq!(stability_pct(200, None), 100.0);
        assert_eq!(stability_pct(0, None), 100.0);
        assert_eq!(stability_pct(0, Some(0)), 100.0);
    }

    #[test]
    fn stability_never_goes_below_zero() {
        assert_eq!(stability_pct(10, Some(25)), 0.0);
    }

    #[test]
    fn unstable_entries_outside_the_filled_set_are_not_counted() {
        let filled = [1usize, 2, 3];
        let count = unstable_filled_count([2, 3, 7, 9_000_000], |i| filled.contains(&i));
        assert_eq!(count, 2);
        let stability = stability_pct(filled.len(), Some(count));
        assert!((stability - 100.0 / 3.0).abs() < 1e-9, "{stability}");
    }

    #[test]
    fn coverage_is_share_of_the_map_and_capped() {
        assert_eq!(coverage_pct(1024, 131072), 0.78125);
        assert_eq!(coverage_pct(131072, 131072), 100.0);
        assert_eq!(coverage_pct(200_000, 131072), 100.0);
        assert_eq!(coverage_pct(5, 0), 0.0);
    }
}
