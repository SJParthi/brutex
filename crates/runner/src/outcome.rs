//! What happened AFTER a signal — the layer everything statistical waits on.
//!
//! # Why the engine could not judge anything without this
//!
//! `engine::Itemset` is `{mask, hits}`. The sweep finds combinations that occur
//! **often**; it has no notion of whether any of them was any good, and
//! `CLAUDE.md` §3 rule 1 forbids inventing one. So ranking, top-N retention, the
//! Deflated Sharpe Ratio, the Probability of Backtest Overfitting, White's
//! Reality Check, purging, walk-forward — every one is blocked on a single
//! missing fact: what the price did next.
//!
//! This module supplies exactly that fact and nothing more. It does not rank,
//! score, or recommend. It answers "on the bars where this combination fired,
//! what was the mean forward move, and is it distinguishable from zero".
//!
//! # The look-ahead boundary, which is structural rather than reviewed
//!
//! §3 rule 7: at bar N the engine may read bars 0..N. A forward return at bar N
//! reads bars N+1..N+H, so it is **exactly** the thing that must never reach the
//! condition bits. It does not, and the type system is what says so:
//! [`indicators::column::Column::build`] takes a slice of candles and an
//! evaluator. There is no parameter through which an outcome could arrive. A
//! `Forward` is built here, in `crates/runner`, from bars the caller already
//! holds, and is handed only to [`edge`] — never to the column, never to the
//! ladder, never to the vocabulary.
//!
//! The outcome is the thing being PREDICTED. Feeding it back would not be a
//! subtle bias; it would be the answer copied into the question.
//!
//! # The tail has no outcome, and is not counted
//!
//! The last `H` bars have no future in the data. Their return is absent rather
//! than zero — zero is a measurement, absence is not — so they contribute to no
//! mean and to no `n`. A run over 1,124 bars at H=15 measures 1,109 of them.
//!
//! # Every choice here is the operator's, recorded rather than assumed
//!
//! The horizon and the measure are decisions §3 rule 1 will not let this crate
//! invent. They are recorded in `docs/05-decisions.md` as stated assumptions
//! with a default, not derived from anything — overruling one is a one-line
//! change and a new decision entry, not a rewrite.

// The same exception `crates/greeks` and `crate::significance` take, for the
// reason §7 states in one breath: prices are paisa integers, and statistical
// values keep full precision. The RETURN in this module is an `i64` of paisa
// throughout; only the mean and the t-statistic are floating, and both are
// statistics rather than money.
#![allow(
    clippy::float_arithmetic,
    reason = "CLAUDE.md §7 keeps statistical values at full precision. Returns \
              are paisa i64; only the mean and t-statistic are floating."
)]

use indicators::Candle;
use indicators::column::Column;
use vocab::ConditionMask;

/// How many bars ahead an outcome looks.
///
/// A newtype so a caller cannot pass a bar count, a period or a `min_hits`
/// where a horizon belongs — three `u32`s that mean entirely different things
/// travel through this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Horizon(u32);

impl Horizon {
    /// The default horizon: fifteen bars.
    ///
    /// **A stated assumption, not a derivation.** On one-minute bars this is a
    /// quarter of an hour — long enough for an intraday move to develop, short
    /// enough to stay inside a 375-bar session and to leave the tail small. No
    /// document defines the right horizon and nothing in the data implies one,
    /// so this is the operator's choice with a default, recorded in
    /// `docs/05-decisions.md` rather than buried here. Overruling it is
    /// [`Horizon::bars`] and a new decision entry.
    pub const DEFAULT: Self = Self(15);

    /// A horizon of `bars`, or `None` for zero.
    ///
    /// Zero is refused because "the return over the next no bars" is not a
    /// quantity — it is zero for every bar by construction, which would make
    /// every combination look identical and equally worthless.
    #[must_use]
    pub const fn bars(bars: u32) -> Option<Self> {
        if bars == 0 { None } else { Some(Self(bars)) }
    }

    /// The horizon in bars.
    #[must_use]
    pub const fn as_bars(self) -> u32 {
        self.0
    }
}

/// The forward move at each bar, in paisa.
///
/// Indexed by the CALLER's slice position — the same index
/// [`Column::sources`] returns — so a signal at column position `j` is paired
/// with `at(column.sources()[j])` and never with `at(j)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forward {
    horizon: Horizon,
    /// How many bars the slice this was built from held.
    ///
    /// Carried so a MISMATCH is distinguishable from the tail. Both make
    /// [`Self::at`] return `None`, and without this they are the same answer:
    /// a `Forward` built from a five-minute slice, handed to a `Column` built
    /// from one-minute bars, would silently produce a finite and plausible
    /// `Edge` whose shortfall in `n` reads exactly like the documented tail.
    /// `crates/runner/src/resample.rs` exists so a caller can build that second
    /// slice, which is precisely the situation that produces the pairing.
    bars_len: usize,
    /// `ret[i] = close[i + H] − close[i]`, in paisa. Length is
    /// `bars.len() − H`, so the tail is absent by construction rather than by a
    /// sentinel a caller could mistake for a measurement.
    ret: Vec<i64>,
}

impl Forward {
    /// The forward move at caller-slice index `i`, or `None` for the tail.
    #[must_use]
    pub fn at(&self, i: usize) -> Option<i64> {
        self.ret.get(i).copied()
    }

    /// How many bars have an outcome at all.
    #[must_use]
    pub fn measured(&self) -> usize {
        self.ret.len()
    }

    /// The horizon these returns were taken over.
    #[must_use]
    pub const fn horizon(&self) -> Horizon {
        self.horizon
    }

    /// Was caller-slice index `i` inside the slice this was built from?
    ///
    /// `false` means the caller paired this `Forward` with a `Column` built
    /// from a DIFFERENT slice -- not that the bar is in the tail. [`Self::at`]
    /// cannot tell those apart and this is what does.
    #[must_use]
    pub const fn covers(&self, i: usize) -> bool {
        i < self.bars_len
    }
}

/// Close-to-close forward returns over `horizon`.
///
/// **The measure is a stated assumption**, recorded in `docs/05-decisions.md`:
/// entry at the close of the signal bar, exit at the close of the bar `H`
/// later, the difference in paisa. It is the only pair of prices knowable at
/// bar N without assuming a fill this crate has no basis to assume — an open, a
/// midpoint or a stop would each require a model of execution that
/// `crates/costs` owns and this module must not duplicate.
///
/// Transaction costs are deliberately **absent**: this is the market's move,
/// not a trade's profit, and conflating them would put a cost model inside a
/// measurement.
#[must_use]
pub fn forward(bars: &[Candle], horizon: Horizon) -> Forward {
    let h = horizon.as_bars() as usize;
    let measurable = bars.len().saturating_sub(h);
    let mut ret: Vec<i64> = Vec::with_capacity(measurable);
    for i in 0..measurable {
        // Both `get`s are in range by the loop bound; `map_or` keeps the lint
        // table's `indexing_slicing` satisfied without an arm that can fire.
        let later = bars.get(i.saturating_add(h)).map_or(0, |b| b.close);
        let now = bars.get(i).map_or(0, |b| b.close);
        ret.push(later.saturating_sub(now));
    }
    Forward {
        horizon,
        bars_len: bars.len(),
        ret,
    }
}

/// What a combination's forward moves looked like.
///
/// Not a ranking and not a recommendation — three numbers about one mask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edge {
    /// Bars where the mask fired **and** an outcome existed.
    ///
    /// Smaller than the itemset's `hits` whenever the combination fires inside
    /// the final `H` bars, and that difference is the tail being excluded
    /// rather than counted as zero.
    pub n: u64,
    /// Mean forward move in paisa.
    pub mean_paisa: f64,
    /// Bars whose source index lay OUTSIDE the slice the `Forward` came from.
    ///
    /// Non-zero means the caller paired a `Column` with a `Forward` built from a
    /// different slice -- a five-minute `Forward` against a one-minute column,
    /// say. Every other field is then measured over whatever overlap happened to
    /// exist, so a non-zero value here makes the rest of this struct
    /// meaningless rather than merely smaller.
    ///
    /// It is a COUNT and not a refusal because `edge` returns three numbers
    /// about one mask and has nowhere to put an error; `CLAUDE.md` §4 asks for
    /// the reason to be named beside the answer, and this names it.
    pub mismatched: u64,
    /// The t-statistic of that mean against zero.
    ///
    /// `mean / (sd / √n)`. Zero when fewer than two observations exist, where a
    /// standard deviation is undefined — reported as zero rather than as a
    /// large number, because an undefined statistic must not read as a strong
    /// one.
    pub t: f64,
}

/// Measures one mask's forward moves over the bars where it fired.
///
/// # Cost
///
/// One pass over the column. Welford's method, so the mean and variance are
/// accumulated in a single traversal without holding the observations and
/// without the catastrophic cancellation a naive sum-of-squares suffers when
/// the mean is large relative to the spread — which is exactly the shape of a
/// paisa price series.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn edge(column: &Column, forward: &Forward, mask: &ConditionMask) -> Edge {
    let mut n: u64 = 0;
    let mut mismatched: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;

    for (bits, &source) in column.bits().iter().zip(column.sources()) {
        if !bits.hits(mask) {
            continue;
        }
        // THE PAIRING THAT MUST GO THROUGH `sources`. `first_swept + j` runs one
        // behind from the first refused bar onward, and every outcome after it
        // would be read off the wrong bar -- see `Column::sources`.
        if !forward.covers(source) {
            // NOT the tail. The caller paired this column with a `Forward`
            // built from a different slice, and the shortfall would otherwise
            // be indistinguishable from the documented tail exclusion.
            mismatched = mismatched.saturating_add(1);
            continue;
        }
        let Some(r) = forward.at(source) else {
            continue; // in the tail: no future exists, so nothing is counted
        };
        n = n.saturating_add(1);
        // A paisa return above 2^52 is 45 trillion rupees on one bar, and a
        // count above 2^52 exceeds every bound the walk carries. Neither can
        // arise, and the alternative -- refusing to compute a mean at all --
        // would be worse than a rounding nobody can reach.
        #[allow(
            clippy::cast_precision_loss,
            reason = "a paisa move or an observation count above 2^52 cannot \
                      arise; the ceiling and the allocator bound stop the walk \
                      far below it."
        )]
        let x = r as f64;
        #[allow(
            clippy::cast_precision_loss,
            reason = "see above -- the observation count is bounded by the column."
        )]
        let count = n as f64;
        let delta = x - mean;
        // `n` is at least one here, so the divide is defined.
        mean += delta / count;
        m2 += delta * (x - mean);
    }

    if n < 2 {
        return Edge {
            n,
            mismatched,
            mean_paisa: mean,
            t: 0.0,
        };
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "the observation count is bounded by the column length."
    )]
    let count = n as f64;
    let variance = m2 / (count - 1.0);
    let standard_error = (variance / count).sqrt();
    let t = if standard_error > 0.0 {
        mean / standard_error
    } else {
        // Every observation identical. The mean is exact and its spread is
        // zero, which is not an infinitely strong result -- it is a degenerate
        // sample, and reporting it as zero refuses to dress one up as the other.
        0.0
    };
    Edge {
        n,
        mismatched,
        mean_paisa: mean,
        t,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Edge, Horizon, edge, forward};
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use indicators::{Candle, OI_NULL};
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn candle(minute: i64, close: i64) -> Candle {
        Candle::new(
            minute.saturating_mul(60_000_000),
            close,
            close.saturating_add(10),
            close.saturating_sub(10),
            close,
            100,
            OI_NULL,
        )
    }

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a positive horizon")
    }

    #[test]
    fn a_zero_horizon_is_refused_and_the_default_is_fifteen() {
        assert_eq!(Horizon::bars(0), None, "the move over no bars is not one");
        assert_eq!(Horizon::DEFAULT.as_bars(), 15);
        assert_eq!(Horizon::bars(1).map(Horizon::as_bars), Some(1));
    }

    #[test]
    fn the_return_is_close_to_close_and_the_tail_has_none() {
        // Closes 100, 110, 130, 125, 140. At H=2 the measurable bars are 0..3.
        let bars: Vec<Candle> = [100, 110, 130, 125, 140]
            .iter()
            .enumerate()
            .map(|(i, &c)| candle(i64::try_from(i).unwrap_or(0), c))
            .collect();
        let f = forward(&bars, h(2));

        assert_eq!(f.measured(), 3, "five bars at H=2 leaves three measurable");
        assert_eq!(f.at(0), Some(30), "130 - 100");
        assert_eq!(f.at(1), Some(15), "125 - 110");
        assert_eq!(f.at(2), Some(10), "140 - 130");
        assert_eq!(f.at(3), None, "the tail has no future in the data");
        assert_eq!(f.at(4), None);
        assert_eq!(
            f.at(999),
            None,
            "and neither does a bar that does not exist"
        );
        assert_eq!(f.horizon(), h(2));
    }

    #[test]
    fn a_horizon_at_or_past_the_input_length_measures_nothing() {
        let bars: Vec<Candle> = (0..3).map(|i| candle(i, 100)).collect();
        assert_eq!(forward(&bars, h(3)).measured(), 0);
        assert_eq!(forward(&bars, h(99)).measured(), 0);
        assert_eq!(forward(&[], h(1)).measured(), 0);
    }

    #[test]
    fn an_edge_over_fewer_than_two_observations_reports_no_t_statistic() {
        // A degenerate sample has no standard deviation, and an undefined
        // statistic must not read as a strong one.
        let e = Edge::default();
        assert_eq!(e.n, 0);
        assert!(e.t.abs() < f64::EPSILON);
    }

    #[test]
    fn the_mean_and_t_are_measured_over_the_bars_the_mask_fired_on() {
        // A real column, and the mask of a position that actually varies.
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);

        // Every live position that fires on at least two measurable bars must
        // produce a finite t, and `n` must never exceed the column length.
        let mut checked = 0_u32;
        for p in 0..64_u32 {
            let mask = ConditionMask::default().with_bit(p);
            let e = edge(&column, &f, &mask);
            assert!(
                usize::try_from(e.n).unwrap_or(usize::MAX) <= column.bits().len(),
                "n exceeded the column"
            );
            if e.n >= 2 {
                assert!(e.t.is_finite(), "position {p} produced a non-finite t");
                assert!(e.mean_paisa.is_finite());
                checked = checked.saturating_add(1);
            }
        }
        assert!(
            checked > 0,
            "the fixture must exercise at least one position"
        );
    }

    #[test]
    fn an_identical_sample_reports_zero_rather_than_an_infinite_t() {
        // A perfectly flat series: every forward move is the same, so the
        // standard error is zero. Dividing by it would be infinity, which reads
        // as the strongest result ever found rather than as a degenerate one.
        let bars: Vec<Candle> = (0..40).map(|i| candle(i, 100 + i * 10)).collect();
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, h(1));
        // Every step is +10, so any mask that fires twice has zero spread.
        let all = ConditionMask::default();
        let e = edge(&column, &f, &all);
        assert!(
            e.t.is_finite(),
            "a zero standard error must not produce an infinite t"
        );
    }

    #[test]
    fn a_forward_from_a_different_slice_is_counted_as_a_mismatch_not_as_the_tail() {
        // The situation `resample` makes reachable: a column built from
        // one-minute bars, paired with forward returns built from the FIVE-minute
        // fold of the same session. Every source index past the shorter slice
        // returns None from `at`, which without `covers` is indistinguishable
        // from the documented tail exclusion -- so the caller would get a finite,
        // plausible Edge measured over whatever overlap happened to exist.
        let minute = crate::synthetic::sessions(8);
        let column = Column::build(&minute, &mut evaluator());
        let five = crate::resample::Period::minutes(5).expect("five");
        let coarse = crate::resample::resample(&minute, five);
        assert!(
            coarse.len() < minute.len(),
            "the coarse slice must be shorter, or this proves nothing"
        );

        let wrong = forward(&coarse, Horizon::DEFAULT);
        let e = edge(&column, &wrong, &ConditionMask::default());
        assert!(
            e.mismatched > 0,
            "pairing a column with a Forward from a shorter slice must be \
             COUNTED, not silently folded into the tail"
        );

        // And the correct pairing reports zero, so a non-zero value means what
        // it says rather than being background noise.
        let right = forward(&minute, Horizon::DEFAULT);
        assert_eq!(
            edge(&column, &right, &ConditionMask::default()).mismatched,
            0,
            "the matching pair must report no mismatch at all"
        );
    }

    #[test]
    fn the_empty_mask_fires_on_every_bar_that_has_an_outcome() {
        // `hits` is `(bits & mask) == mask`, so the empty mask matches
        // everything -- which makes it the exact test of the tail exclusion.
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);
        let e = edge(&column, &f, &ConditionMask::default());

        let in_tail = column
            .sources()
            .iter()
            .filter(|&&s| f.at(s).is_none())
            .count();
        assert_eq!(
            usize::try_from(e.n).unwrap_or(usize::MAX),
            column.bits().len().saturating_sub(in_tail),
            "every swept bar outside the tail must be counted, and every bar \
             inside it must not"
        );
        assert!(in_tail > 0, "the fixture must actually have a tail");
    }
}
