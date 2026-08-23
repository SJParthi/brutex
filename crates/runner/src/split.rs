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

/// Folds whose training window is a FIXED WIDTH that slides forward.
///
/// # The question this asks that [`anchored_folds`] cannot
///
/// Anchored training always starts at bar zero and grows, so every later fold
/// still contains every earlier year. A combination that worked only in 2020 has
/// 2020 in all five training sets, propping it up, and the walk reports it
/// holding up five times.
///
/// Rolling drops it. Train on 2020 and test on 2021; train on 2021 and test on
/// 2022; train on 2022 and test on 2023. **"Does last year predict this year"**
/// is a far harder question than "does everything so far predict next year", and
/// it is the one an operator actually faces: at the start of 2023 they have 2022
/// in hand and a decision to make.
///
/// # Both are strictly forward
///
/// The test window always follows the training window with a purge between, so
/// no fold is ever judged on a bar its training saw. `CLAUDE.md` §3 rule 7 holds
/// here exactly as it does for the anchored shape.
///
/// # The warm-up, which is why this did not exist before
///
/// `indicators::Evaluator` is stateful — EMAs, session rollovers, a warm-up
/// prefix — so it consumes bars IN ORDER and cannot begin mid-series with an
/// empty state. An anchored fold always starts at bar zero, so its warm-up is
/// real; a rolling window starting at 2021 would spend its first bars warming
/// and trade on state it did not build.
///
/// The left training range solves it: the window is `warm..boundary`, and a
/// caller feeds the evaluator from `0` while only SWEEPING from `warm`. That is
/// what the engine already does at the start of every run, applied per fold, and
/// it is why this returns a range rather than a length.
///
/// # Width
///
/// `bars / (splits + 1)`, the same block size the anchored shape uses, so the
/// two are comparable on the same column at the same split count. A fold whose
/// training window would start before bar zero is clamped there rather than
/// dropped: an early fold with a shorter window is still a forward test, and
/// discarding it would silently reduce the split count a caller asked for.
#[must_use]
pub fn rolling_folds(bars: usize, horizon: Horizon, splits: usize) -> Vec<Fold> {
    let h = usize::try_from(horizon.as_bars()).unwrap_or(0);
    let blocks = splits.saturating_add(1);
    if splits == 0 || bars == 0 || bars < blocks {
        return Vec::new();
    }
    let width = bars / blocks;

    let mut out: Vec<Fold> = Vec::with_capacity(splits);
    for s in 0..splits {
        let boundary = s.saturating_add(1).saturating_mul(width);
        // THE ONE DIFFERENCE FROM ANCHORED: training starts one width back
        // rather than at zero, so the window slides instead of growing.
        let warm = boundary.saturating_sub(width);
        let test_start = boundary.saturating_add(h).min(bars);
        let test_end = if s.saturating_add(1) == splits {
            bars
        } else {
            boundary.saturating_add(width).saturating_add(h).min(bars)
        };
        out.push(Fold {
            purged: test_start.saturating_sub(boundary),
            embargoed: 0,
            // The right-hand range is empty for the reason the anchored shape
            // gives: nothing after the test window is knowable when the
            // decision is made.
            train: (warm..boundary, 0..0),
            test: test_start..test_end,
        });
    }
    out
}
/// Growing-prefix folds: train on everything before, test on what comes next.
///
/// # Why this exists beside [`purged_folds`], rather than instead of it
///
/// [`purged_folds`] puts a test window in the MIDDLE of the series, which
/// leaves training data on both sides. That is the right shape for a
/// cross-validated overfitting probability — and it **cannot drive this
/// engine's evaluator**.
///
/// `indicators::Evaluator` is stateful: EMAs, session rollovers, a warm-up
/// prefix. It consumes bars in order and cannot be handed two disjoint ranges
/// without either restarting the warm-up in the middle of the series or
/// pretending the gap is not there. Both are lies about what the indicators
/// saw, and the second is the quieter one.
///
/// So a walk-forward over THIS engine is anchored: `train = 0..t`,
/// `test = t+H..end`, both contiguous, with the purge between them. The
/// evaluator sees an unbroken prefix, which is the only thing it can honestly
/// be given.
///
/// # What it costs, stated rather than hidden
///
/// An anchored walk-forward tests each period once and trains on everything
/// before it, so the earliest fold has the least data and the latest has the
/// most. It is a weaker design than k-fold — fewer test observations, and the
/// folds are not exchangeable. It is what the evaluator permits.
///
/// `splits` is the number of TEST periods. The first `1/(splits+1)` of the
/// series is training-only, so a model always has something to fit before the
/// first judgement.
#[must_use]
pub fn anchored_folds(bars: usize, horizon: Horizon, splits: usize) -> Vec<Fold> {
    let h = horizon.as_bars() as usize;
    let blocks = splits.saturating_add(1);
    if splits == 0 || bars == 0 || bars < blocks {
        return Vec::new();
    }
    let width = bars / blocks;

    let mut out: Vec<Fold> = Vec::with_capacity(splits);
    for s in 0..splits {
        // Training is everything up to the boundary; the test period follows the
        // purge. The last split takes the remainder so no bar is lost.
        let boundary = s.saturating_add(1).saturating_mul(width);
        let test_start = boundary.saturating_add(h).min(bars);
        let test_end = if s.saturating_add(1) == splits {
            bars
        } else {
            boundary.saturating_add(width).saturating_add(h).min(bars)
        };
        out.push(Fold {
            purged: test_start.saturating_sub(boundary),
            embargoed: 0,
            // The right-hand training range is deliberately empty: an anchored
            // fold never trains on anything after its test period, because that
            // is the future at the moment the decision is made.
            train: (0..boundary, 0..0),
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
    use super::{anchored_folds, purged_folds};
    use crate::outcome::Horizon;

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a positive horizon")
    }

    /// A ROLLING FOLD FORGETS, AND AN ANCHORED ONE DOES NOT.
    ///
    /// # The difference this pins
    ///
    /// Anchored training always starts at bar zero, so every later fold still
    /// contains every earlier year. A combination that worked only in the first
    /// year has that year in ALL of the training sets, propping it up, and the
    /// walk reports it holding five times.
    ///
    /// Rolling slides a fixed width forward: train on 2020, test 2021; train on
    /// 2021, test 2022. "Does last year predict this year" is the harder
    /// question and the one an operator actually faces.
    ///
    /// Asserted as a property rather than on one fixture: for every fold past
    /// the first, an anchored window must still hold bar 0 and a rolling one
    /// must NOT.
    #[test]
    fn a_rolling_window_slides_forward_where_an_anchored_one_grows() {
        for (bars, splits) in [(1_200_usize, 5_usize), (37_791, 17), (600, 2)] {
            let rolling = super::rolling_folds(bars, h(15), splits);
            let anchored = super::anchored_folds(bars, h(15), splits);
            assert_eq!(rolling.len(), anchored.len(), "same split count");

            for (i, (r, a)) in rolling.iter().zip(anchored.iter()).enumerate() {
                assert_eq!(r.test, a.test, "fold {i}: the test window is the same");
                assert!(a.trains_on(0), "fold {i}: anchored always holds bar 0");
                if i > 0 {
                    assert!(
                        !r.trains_on(0),
                        "fold {i}: a rolling window past the first must have \
                         slid off bar 0, or it is anchored wearing another name"
                    );
                }
                // AND EVERY WIDTH IS THE SAME, which is what makes it rolling
                // rather than merely shorter.
                assert_eq!(
                    r.train_len(),
                    bars / splits.saturating_add(1),
                    "fold {i}: the window width is constant"
                );
            }
        }
    }

    /// BOTH SHAPES ARE STRICTLY FORWARD, WHICH IS THE RULE THAT MATTERS.
    ///
    /// Sliding a window is only safe if it still never trains on a bar it will
    /// be judged on. §3 rule 7 is the same for both shapes and this asserts it
    /// for the new one, over the outcome window rather than the bar alone: a
    /// training bar at `i` carries a label built from `i..i+h`, so `i + h` must
    /// fall before the test range begins.
    #[test]
    fn a_rolling_fold_never_trains_on_a_bar_its_outcome_reaches_into_the_test() {
        let horizon = 15_usize;
        for (bars, splits) in [(1_200_usize, 5_usize), (37_791, 17)] {
            for (i, f) in super::rolling_folds(bars, h(15), splits)
                .into_iter()
                .enumerate()
            {
                for bar in f.train.0.clone().chain(f.train.1.clone()) {
                    assert!(
                        bar.saturating_add(horizon) < f.test.start,
                        "fold {i}: training bar {bar} has an outcome window \
                         reaching {} which is inside the test range starting {}",
                        bar + horizon,
                        f.test.start
                    );
                }
            }
        }
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
    fn an_anchored_fold_never_trains_on_its_own_future() {
        // The property that distinguishes a walk-forward from a k-fold: at the
        // moment a decision is made, everything after it is the future.
        let bars = 1_000;
        let folds = anchored_folds(bars, h(15), 4);
        assert_eq!(folds.len(), 4);
        for f in &folds {
            assert!(
                f.train.1.is_empty(),
                "an anchored fold has no training data after its test period"
            );
            assert!(
                f.train.0.end <= f.test.start,
                "training must end before testing begins: {:?} then {:?}",
                f.train.0,
                f.test
            );
            // And the purge holds: no training bar's outcome reaches the test.
            for i in f.train.0.clone() {
                assert!(
                    i.saturating_add(15) < f.test.start || f.test.is_empty(),
                    "training bar {i} reaches into {:?}",
                    f.test
                );
            }
        }
    }

    #[test]
    fn the_training_prefix_grows_and_the_test_periods_advance() {
        let folds = anchored_folds(1_000, h(10), 4);
        let mut previous_train = 0_usize;
        let mut previous_test_start = 0_usize;
        for f in &folds {
            assert!(
                f.train_len() > previous_train,
                "each anchored fold must train on strictly more than the last"
            );
            assert!(
                f.test.start > previous_test_start,
                "and test strictly later"
            );
            previous_train = f.train_len();
            previous_test_start = f.test.start;
        }
        // The last fold reaches the end of the series.
        assert_eq!(folds.last().map(|f| f.test.end), Some(1_000));
    }

    #[test]
    fn an_anchored_split_that_cannot_be_honoured_returns_nothing() {
        assert!(anchored_folds(0, h(5), 3).is_empty(), "no bars");
        assert!(anchored_folds(100, h(5), 0).is_empty(), "no splits");
        assert!(
            anchored_folds(3, h(5), 10).is_empty(),
            "more blocks than bars cannot be honoured"
        );
        // A horizon swallowing the series leaves empty test periods rather than
        // wrapping: the purge consumes everything after the boundary.
        let swallowed = anchored_folds(50, h(1_000), 2);
        for f in &swallowed {
            assert!(f.test.is_empty(), "nothing survives the purge to be tested");
            assert!(f.train_len() > 0, "but the prefix is still trainable");
        }
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
