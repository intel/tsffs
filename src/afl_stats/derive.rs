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

#[cfg(test)]
mod tests {
    use super::*;

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
