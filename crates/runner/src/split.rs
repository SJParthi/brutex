//! Train and test ranges that cannot see each other.
//!
//! # Why an ordinary split leaks, and leaks silently
//!
//! Cut a bar series in half and the last bar of the training half has a forward
//! outcome reaching `H` bars into the test half. Its label was computed from
//! data the test set is supposed to be holding back. Nothing errors, nothing
//! looks wrong, and the out-of-sample result is contaminated by exactly the
//! amount that makes it look good.
//!
//! López de Prado's answer is two operations, and this module is both:
//!
//! * **purge** — drop any training bar whose outcome window overlaps the test
//!   range. The bar is not moved to test and not counted anywhere; it is
//!   discarded, because its label is the thing that cannot be trusted.
//! * **embargo** — drop a further stretch of training bars *after* the test
//!   range. Serial correlation means a bar immediately following the test
//!   window still carries information about it even when its outcome window
//!   does not overlap.
//!
//! # Every index here is into the caller's own bar slice
//!
//! The same index space [`indicators::column::Column::sources`] returns, so a
//! split composes with [`crate::outcome`] without a second mapping to get
//! wrong — and getting exactly that mapping wrong is the defect an audit found
//! in this crate hours ago.
//!
//! # What this does NOT do
//!
//! It does not run a sweep, score a fold, or decide how many folds are right.
//! It answers "which bars may a model see, and which must it not", and that is
//! all. Composing the folds into a walk-forward or a cross-validated overfitting
//! probability is a caller's job, and each is a decision `CLAUDE.md` §3 rule 1
//! will not let this module make.

use core::ops::Range;

use crate::outcome::Horizon;

/// One fold: what a model may fit on, and what it is then judged against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fold {
    /// Bars the model may see. Two ranges because a test window in the middle
    /// of the series leaves training data on both sides, and concatenating them
    /// into one range would silently include the test window itself.
    pub train: (Range<usize>, Range<usize>),
    /// Bars the model is judged on, and must not have seen.
    pub test: Range<usize>,
    /// Training bars discarded because their outcome window reached into `test`.
    pub purged: usize,
    /// Training bars discarded after `test` for serial correlation.
    pub embargoed: usize,
}

impl Fold {
    /// How many bars the model may actually fit on.
    #[must_use]
    pub fn train_len(&self) -> usize {
        self.train.0.len().saturating_add(self.train.1.len())
    }

    /// Is `index` in the training set?
    #[must_use]
    pub fn trains_on(&self, index: usize) -> bool {
        self.train.0.contains(&index) || self.train.1.contains(&index)
    }
}

/// `folds` contiguous test windows over `bars`, each purged and embargoed.
///
/// The embargo is `horizon` bars, matching the outcome window: a bar that far
/// past the test range is the first whose label shares no data with it. Using
/// the horizon rather than a separate constant means there is no second number
/// to get wrong, and no caller to ask for one.
///
/// Returns an empty vector when `folds` is zero or the series is too short to
/// hold a single test window — a fold that cannot be honoured is not returned
/// as a degenerate one, because a caller looping over an empty result is
/// obviously doing nothing while one looping over an empty *range* is not.
///
/// # Cost
///
/// O(folds). No bar is touched; the ranges are arithmetic on indices.
#[must_use]
pub fn purged_folds(bars: usize, horizon: Horizon, folds: usize) -> Vec<Fold> {
    let h = horizon.as_bars() as usize;
    if folds == 0 || bars == 0 || bars < folds {
        return Vec::new();
    }
    // `bars >= folds` is guaranteed by the guard above, so the width is at
    // least one and needs no zero check -- a check there would be an arm no
    // input could reach.
    let width = bars / folds;

    let mut out: Vec<Fold> = Vec::with_capacity(folds);
    for f in 0..folds {
        let test_start = f.saturating_mul(width);
        // The last fold takes the remainder, so no bar is dropped by integer
        // division -- a fold count that does not divide the series would
        // otherwise silently ignore up to `folds - 1` bars at the end.
        let test_end = if f.saturating_add(1) == folds {
            bars
        } else {
            test_start.saturating_add(width)
        };

        // PURGE BEFORE. A training bar at `i` has an outcome window reaching
        // `i + h`, so it must end before the test range begins: `i + h <
        // test_start`, hence `i < test_start - h`.
        let left_end = test_start.saturating_sub(h);
        let left = 0..left_end;

        // EMBARGO AFTER. Training resumes `h` bars past the test window.
        let right_start = test_end.saturating_add(h).min(bars);
        let right = right_start..bars;

        out.push(Fold {
            purged: test_start.saturating_sub(left_end),
            embargoed: right_start.saturating_sub(test_end),
            train: (left, right),
            test: test_start..test_end,
        });
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::purged_folds;
    use crate::outcome::Horizon;

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a positive horizon")
    }

    #[test]
    fn no_training_bar_can_see_into_its_test_window() {
        // THE PROPERTY THIS MODULE EXISTS FOR, asserted directly: for every fold
        // and every training bar, the outcome window [i, i+h] must not overlap
        // the test range. A plain split violates this on its last h bars, and
        // nothing about the result looks wrong when it does.
        let bars = 1_000;
        let horizon = h(15);
        for fold in purged_folds(bars, horizon, 5) {
            for i in fold.train.0.clone().chain(fold.train.1.clone()) {
                let window_end = i.saturating_add(15);
                let overlaps = i < fold.test.end && window_end >= fold.test.start;
                assert!(
                    !overlaps,
                    "training bar {i} has an outcome window reaching {window_end}, \
                     which touches the test range {:?} -- that is the leak",
                    fold.test
                );
            }
        }
    }

    #[test]
    fn the_test_windows_tile_the_series_exactly_once() {
        let bars = 1_003;
        let folds = purged_folds(bars, h(10), 4);
        assert_eq!(folds.len(), 4);
        // Contiguous, non-overlapping, and covering every bar -- the remainder
        // from integer division goes to the last fold rather than being lost.
        let mut expected = 0_usize;
        for f in &folds {
            assert_eq!(f.test.start, expected, "a gap or an overlap between folds");
            expected = f.test.end;
        }
        assert_eq!(expected, bars, "the last fold must absorb the remainder");
    }

    #[test]
    fn purge_and_embargo_are_counted_and_the_counts_are_honest() {
        let folds = purged_folds(1_000, h(20), 5);
        let middle = folds.get(2).expect("five folds");
        assert_eq!(middle.purged, 20, "a middle fold purges a full horizon");
        assert_eq!(middle.embargoed, 20, "and embargoes a full horizon");

        // The first fold has no training data before it, so nothing to purge.
        let first = folds.first().expect("five folds");
        assert_eq!(first.purged, 0, "there is nothing before the first window");
        assert!(first.train.0.is_empty());

        // The last fold reaches the end, so there is nothing to embargo.
        let last = folds.last().expect("five folds");
        assert_eq!(last.embargoed, 0, "there is nothing after the last window");
        assert!(last.train.1.is_empty());
    }

    #[test]
    fn training_shrinks_by_exactly_what_was_removed() {
        let bars = 1_000;
        let folds = purged_folds(bars, h(15), 4);
        for f in &folds {
            let removed = f.test.len() + f.purged + f.embargoed;
            assert_eq!(
                f.train_len(),
                bars - removed,
                "the training set must be everything except the test window, the \
                 purge and the embargo -- no more and no less"
            );
        }
    }

    #[test]
    fn trains_on_agrees_with_the_ranges_it_reports() {
        let folds = purged_folds(200, h(5), 3);
        let f = folds.get(1).expect("three folds");
        for i in 0..200_usize {
            let by_range = f.train.0.contains(&i) || f.train.1.contains(&i);
            assert_eq!(f.trains_on(i), by_range, "at {i}");
            if f.test.contains(&i) {
                assert!(!f.trains_on(i), "a test bar is never a training bar");
            }
        }
    }

    #[test]
    fn a_fold_that_cannot_be_honoured_is_not_returned_as_a_degenerate_one() {
        assert!(purged_folds(0, h(5), 3).is_empty(), "no bars");
        assert!(purged_folds(100, h(5), 0).is_empty(), "no folds");
        assert!(
            purged_folds(3, h(5), 10).is_empty(),
            "more folds than bars cannot be honoured, and an empty result is \
             obviously nothing while an empty RANGE quietly is not"
        );
        // And one fold over the whole series is legitimate: everything is test,
        // nothing is train, which is a valid if useless split.
        let one = purged_folds(100, h(5), 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one.first().map(super::Fold::train_len), Some(0));
    }

    #[test]
    fn a_horizon_longer_than_the_series_purges_everything_rather_than_wrapping() {
        // `saturating_sub` is what makes this a purge rather than an underflow.
        let folds = purged_folds(50, h(1_000), 2);
        assert_eq!(folds.len(), 2);
        for f in &folds {
            assert_eq!(
                f.train_len(),
                0,
                "with an outcome window longer than the whole series, EVERY bar's \
                 label reaches into the test range -- so there is nothing left \
                 that may honestly be trained on, and the purge says so rather \
                 than underflowing into a huge range"
            );
            assert!(f.train.0.is_empty() && f.train.1.is_empty());
            assert!(!f.test.is_empty(), "the test window still exists");
        }
    }
}
