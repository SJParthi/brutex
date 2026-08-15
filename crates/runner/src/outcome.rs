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

/// The minute of the IST day at which every position is force-closed: 15:10.
///
/// # This is the TRADE's deadline, not the exchange's
///
/// The NSE regular session runs to 15:30 (`SESSION_CLOSE_MINUTE = 930`). This is
/// **910**, twenty minutes earlier, and the two are different facts. The
/// exchange closing is a property of the venue; this is the operator's rule
/// about a POSITION: every trade this engine models is intraday, entry may
/// happen from 09:15 onward, and the position is squared off automatically at
/// 15:10 whether it is long or short and whether or not the horizon has run out.
///
/// Nothing is ever held overnight, so no outcome may be measured across one.
///
/// # What it was before
///
/// [`forward`] computed `close[i + H] - close[i]` over the slice as one
/// unbroken run, with no notion of a day at all. A signal fired before the close
/// measured its "next fifteen bars" straight through the overnight gap and into
/// the following morning's open — an overnight hold priced as a quarter of an
/// hour of intraday movement, and gapping risk is not the same risk. Two
/// independent audit agents confirmed it at high severity.
///
/// The last twenty bars of every session are the ones this changes: at 15:10 a
/// position is closed, so 15:10 onward cannot be an ENTRY, and any entry between
/// 14:55 and 15:10 exits early at 15:10 rather than running its full horizon.
const AUTO_CLOSE_MINUTE: i64 = 15 * 60 + 10;

/// The last bar whose interval ENDS at or before the square-off: the one
/// stamped 15:09.
///
/// # A bar is stamped at its OPEN, so the stamp is one minute early
///
/// `docs/00-charter.md` §3 stamps a bar at the open of its interval, so the bar
/// stamped `m` covers `[m, m+1)` and its close prints at `m+1`. The bar stamped
/// 15:10 therefore closes at **15:11**, a minute AFTER the position was squared
/// off, and using it as the exit reads a price the trade never saw.
///
/// This was wrong in the first version of this rule and the error was measured
/// rather than reasoned about: an entry at 14:55 came out as 105 paisa against
/// the 15:10-stamped bar's close and 77 paisa against the correct one — **36% on
/// fourteen observations per session**. The stamp looked right, reconciled with
/// every neighbouring number, and was measured against the wrong instant.
///
/// So the last fill bar is 909, and [`AUTO_CLOSE_MINUTE`] stays 910 because 910
/// is the INSTANT the position closes. The two are different facts and both are
/// needed: one is a deadline, the other is the last bar that fits inside it.
const LAST_FILL_MINUTE: i64 = AUTO_CLOSE_MINUTE - 1;

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
    ret: Vec<Option<i64>>,
}

impl Forward {
    /// The forward move at caller-slice index `i`, or `None` for the tail.
    #[must_use]
    pub fn at(&self, i: usize) -> Option<i64> {
        self.ret.get(i).copied().flatten()
    }

    /// How many bars have an outcome at all.
    ///
    /// Not `ret.len()` any more, and the difference is the whole intraday rule.
    /// `ret` is now one slot per OFFERED bar, holding `None` wherever no trade
    /// could be entered and measured: a bar at or after [`AUTO_CLOSE_MINUTE`],
    /// or the forced-close bar itself, or the tail of the slice. Counting slots
    /// would count those as measurements.
    #[must_use]
    pub fn measured(&self) -> usize {
        self.ret.iter().filter(|r| r.is_some()).count()
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

    /// Was this built from the same slice as a column that was offered
    /// `offered` bars?
    ///
    /// # The direction [`Self::covers`] structurally cannot see
    ///
    /// `covers` answers a PER-INDEX question — `i < bars_len` — and that is
    /// satisfied by **every** index of a shorter column. So it catches a
    /// `Forward` built from a slice shorter than the column's and nothing else.
    ///
    /// The other direction is silent and worse. Pair a `Forward` over
    /// one-minute bars with a `Column` built from the five-minute resampling of
    /// the same session: the column's sources run `0..600`, the forward's
    /// `bars_len` is 3,000, every per-index check passes, and [`Self::at`]
    /// returns the ONE-MINUTE forward return for a FIVE-MINUTE bar. `n` comes
    /// out full, [`Edge::mismatched`] comes out zero, and the t-statistic is
    /// fabricated from returns belonging to other bars.
    ///
    /// Comparing lengths catches both directions at once, and it is checked
    /// ONCE per [`edge`] rather than per bar.
    ///
    /// # Cost
    ///
    /// One `try_from` and one integer compare. `CLAUDE.md` §3 rule 4.
    #[must_use]
    pub fn built_from_same_slice_as(&self, offered: u64) -> bool {
        u64::try_from(self.bars_len).is_ok_and(|len| len == offered)
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

    // PASS ONE: the IST day and minute of every bar. `ist_day` is monotone even
    // at `i64::MAX` (it saturates rather than wraps), which is the only property
    // the boundary test below depends on -- `day[j] != day[i]` must mean the day
    // genuinely changed.
    let stamps: Vec<(i64, i64)> = bars
        .iter()
        .map(|b| {
            (
                indicators::ist_day(b.ts_micros),
                ist_minute_of_day(b.ts_micros),
            )
        })
        .collect();

    // PASS TWO, BACKWARD: for each bar, the index of the last bar in ITS OWN
    // session at or before the forced close. This is the bar the position is
    // auto-closed on, and no hold may reach past it.
    //
    // Backward and not a scan per bar: the answer for `i` is the answer for
    // `i + 1` whenever they share a day, so one reverse pass gives every bar its
    // exit in O(1) amortised. A forward search per bar would be O(H) and H is
    // caller-supplied, so it would be O(1) only by accident.
    let mut close_at: Vec<Option<usize>> = vec![None; bars.len()];
    for i in (0..bars.len()).rev() {
        let Some(&(day, minute)) = stamps.get(i) else {
            continue;
        };
        let same_day_next = i
            .checked_add(1)
            .and_then(|j| stamps.get(j).map(|&(d, _)| d == day))
            .unwrap_or(false);
        let inherited = if same_day_next {
            i.checked_add(1)
                .and_then(|j| close_at.get(j).copied().flatten())
        } else {
            None
        };
        let mine = if minute <= LAST_FILL_MINUTE {
            Some(i)
        } else {
            None
        };
        // The later bar wins: the auto-close is the LAST tradeable bar of the
        // session, not the first one that qualifies.
        if let Some(slot) = close_at.get_mut(i) {
            *slot = inherited.or(mine);
        }
    }

    // PASS THREE: the return, from entry to the earlier of the horizon and the
    // forced close.
    let mut ret: Vec<Option<i64>> = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        let Some(&(_, minute)) = stamps.get(i) else {
            ret.push(None);
            continue;
        };
        // NO ENTRY AT OR AFTER THE FORCED CLOSE. At 15:10 the position is being
        // closed, so it cannot also be opened; `<` and not `<=`.
        if minute > LAST_FILL_MINUTE {
            ret.push(None);
            continue;
        }
        let Some(forced) = close_at.get(i).copied().flatten() else {
            ret.push(None);
            continue;
        };
        let want = i.saturating_add(h);
        // THE HOLD ENDS AT THE HORIZON OR AT THE FORCED CLOSE, WHICHEVER COMES
        // FIRST -- but "the data ran out" is neither, and conflating the two
        // would invent a measurement.
        //
        // If the horizon fits inside the tradeable window, it is an ordinary
        // outcome. If it does not, the trade was squared off early at 15:10, and
        // THAT is a real measured outcome -- but only if `forced` is genuinely
        // the end of the window rather than the end of the file. A day whose
        // bars simply stop at 11:00 because the slice was cut there has no
        // 15:10 price, and measuring to 11:00 would report a forced exit that
        // never happened.
        let exit = if want <= forced {
            want
        } else if is_window_end(&stamps, forced) {
            forced
        } else {
            ret.push(None);
            continue;
        };
        if exit <= i {
            ret.push(None);
            continue;
        }
        let later = bars.get(exit).map_or(0, |b| b.close);
        let now = bars.get(i).map_or(0, |b| b.close);
        ret.push(Some(later.saturating_sub(now)));
    }

    Forward {
        horizon,
        bars_len: bars.len(),
        ret,
    }
}

/// Minute of the IST day, `0..1440`.
///
/// `div_euclid` and `rem_euclid`, never `/` and `%`: both truncate toward zero,
/// which puts a pre-epoch stamp in a negative minute. `saturating_add` for the
/// reason [`indicators::ist_day`] gives at length -- a wrap near `i64::MAX`
/// returns a plausible in-session minute from a timestamp that is not in the
/// session at all, and a saturated stamp lands far past the close where the
/// comparisons above discard it.
///
/// The same computation [`indicators::orb::minutes_since_open`] makes, without
/// its subtraction: that one answers "how far into the session", this one
/// answers "what time is it", and the forced close is a time.
/// Is bar `j` the genuine last tradeable bar of its window, or just the last bar
/// in the slice?
///
/// The difference is a measurement that happened against one that did not. A
/// position squared off at 15:10 has a real exit price and a real return; a
/// slice that simply stops at 11:00 has neither, and treating its final bar as a
/// forced close would report an exit the market never gave.
///
/// `j` ends the window when the bar after it belongs to another day, or is at or
/// past the forced close. When there is no bar after it, the data ran out and
/// the answer is no.
fn is_window_end(stamps: &[(i64, i64)], j: usize) -> bool {
    let Some(&(day, _)) = stamps.get(j) else {
        return false;
    };
    j.checked_add(1)
        .and_then(|k| stamps.get(k))
        .is_some_and(|&(next_day, next_minute)| next_day != day || next_minute > LAST_FILL_MINUTE)
}

/// Minute of the IST day, `0..1440`.
const fn ist_minute_of_day(ts_micros: i64) -> i64 {
    ts_micros
        .saturating_add(indicators::IST_OFFSET_MICROS)
        .div_euclid(60_000_000)
        .rem_euclid(1_440)
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

    // WHOLE-SLICE AGREEMENT, ASKED ONCE. `covers` is a per-index test and every
    // index of a SHORTER column satisfies it, so on its own it lets a `Forward`
    // built from a longer slice through silently -- see
    // `Forward::built_from_same_slice_as`. Asked here rather than per bar
    // because it is a property of the pair, not of a row.
    let paired = forward.built_from_same_slice_as(column.census().offered);

    for (bits, &source) in column.bits().iter().zip(column.sources()) {
        if !bits.hits(mask) {
            continue;
        }
        // THE PAIRING THAT MUST GO THROUGH `sources`. `first_swept + j` runs one
        // behind from the first refused bar onward, and every outcome after it
        // would be read off the wrong bar -- see `Column::sources`.
        if !paired || !forward.covers(source) {
            // NOT the tail. The caller paired this column with a `Forward`
            // built from a different slice, and the shortfall would otherwise
            // be indistinguishable from the documented tail exclusion.
            //
            // `!paired` first, because when the two slices disagree at all
            // EVERY hit is mispaired -- not merely the ones whose index runs
            // off the end. A longer forward covers every index and would
            // otherwise report a full `n` against returns from other bars.
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
    use super::{Edge, Horizon, LAST_FILL_MINUTE, edge, forward};
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
    fn no_outcome_is_ever_measured_across_the_forced_close_or_into_another_day() {
        // THE INTRADAY RULE, ASSERTED DIRECTLY. Every trade is intraday: entry
        // from 09:15, and the position is squared off automatically at 15:10
        // whether long or short. So no measured return may span two trading
        // days, and none may run past 15:10 on its own day.
        //
        // `synthetic::bar` stamps minute 0 at the IST open, so bar `m` is IST
        // minute 555 + m and the forced close at minute 910 is bar 355. The
        // constants are DERIVED here rather than written down, so a change to
        // the fixture cannot quietly make this test vacuous.
        let bars = crate::synthetic::sessions(3);
        let f = forward(&bars, h(15));

        let minute_of = |i: usize| -> i64 {
            let ts = bars.get(i).map_or(0, |b| b.ts_micros);
            ts.saturating_add(indicators::IST_OFFSET_MICROS)
                .div_euclid(60_000_000)
                .rem_euclid(1_440)
        };
        let close_bar = (0..375)
            .find(|&i| minute_of(i) == LAST_FILL_MINUTE)
            .expect("the fixture must contain a 15:09 bar");
        assert_eq!(
            close_bar, 354,
            "bar 354 is STAMPED 15:09 and CLOSES at 15:10, so it is the last \
             fill that fits inside the square-off. The bar stamped 15:10 closes \
             at 15:11 -- a minute after the position is already gone."
        );

        // ONE: the forced-close bar cannot itself be an entry (entering at the
        // square-off leaves zero holding time), and nor can any bar after it,
        // right up to the exchange close at 15:29.
        for i in close_bar..375 {
            assert!(
                f.at(i).is_none(),
                "bar {i} is IST minute {} -- at or after the 15:10 square-off, \
                 so no position may be opened on it",
                minute_of(i)
            );
        }

        // TWO: the last legal entry is 15:09, and it exits at 15:10 -- one bar,
        // not the full fifteen.
        let last_entry = close_bar.saturating_sub(1);
        let expected = bars.get(close_bar).map_or(0, |b| b.close)
            - bars.get(last_entry).map_or(0, |b| b.close);
        assert_eq!(
            f.at(last_entry),
            Some(expected),
            "an entry at 15:09 is squared off at 15:10, so its return is one \
             bar of movement and not fifteen"
        );

        // THREE: an entry whose horizon would run past 15:10 exits AT 15:10.
        // Before the rule existed this measured close[365] - close[350], which
        // is a price from after the position was already closed.
        let early = close_bar.saturating_sub(5);
        let forced =
            bars.get(close_bar).map_or(0, |b| b.close) - bars.get(early).map_or(0, |b| b.close);
        let unforced = bars.get(early.saturating_add(15)).map_or(0, |b| b.close)
            - bars.get(early).map_or(0, |b| b.close);
        assert_eq!(f.at(early), Some(forced), "the exit is the 15:10 close");
        assert_ne!(
            forced, unforced,
            "the fixture must make the two answers differ, or this proves \
             nothing"
        );

        // FOUR, AND IT IS THE RULE ITSELF: no measured outcome anywhere in the
        // slice crosses a day boundary. Checked over every bar rather than a
        // sample, because "usually intraday" is not the promise.
        for i in 0..bars.len() {
            if f.at(i).is_none() {
                continue;
            }
            let entry_day = indicators::ist_day(bars.get(i).map_or(0, |b| b.ts_micros));
            // The exit is at or before the forced close of the entry's own day, so
            // the last bar this outcome could have read is that day's 15:10 --
            // which is why the horizon bar, clamped to the slice, is still the
            // same day. An outcome that left its day would show here.
            let exit_day = indicators::ist_day(
                bars.get(i.saturating_add(15).min(bars.len().saturating_sub(1)))
                    .map_or(0, |b| b.ts_micros),
            );
            assert!(
                entry_day == exit_day,
                "bar {i} measured an outcome whose exit is on another day"
            );
        }
    }

    #[test]
    fn a_forward_from_a_longer_slice_is_refused_rather_than_silently_mispaired() {
        // THE DIRECTION `covers` CANNOT SEE. A column built from a SHORT slice
        // paired with a forward built from a LONGER one: every source index
        // satisfies `i < bars_len`, so the per-index guard passes on every row
        // and `at(source)` hands back a return belonging to a different bar.
        //
        // Before `built_from_same_slice_as`, this produced a full `n`, a
        // `mismatched` of ZERO, and a t computed from other bars' returns --
        // reported by the renderer as an ordinary finding.
        // `synthetic::sessions` and not hand-rolled candles: a short hand-made
        // slice warms up and never sweeps a bar, so the column comes out EMPTY
        // and every assertion below would pass without the guard existing.
        // Eight sessions and not two: the warm-up consumes more than two
        // sessions, so a two-session column comes out EMPTY and every
        // assertion below would pass without the guard existing at all.
        let short = crate::synthetic::sessions(8);
        let long = crate::synthetic::sessions(16);

        let column = Column::build(&short, &mut evaluator());
        assert!(
            !column.is_empty(),
            "the fixture must sweep bars, or this test proves nothing"
        );
        let from_long = forward(&long, h(1));
        let all = ConditionMask::default();

        let wrong = edge(&column, &from_long, &all);
        assert_eq!(
            wrong.n, 0,
            "not one observation may be counted from a forward built from \
             another slice"
        );
        assert!(
            wrong.mismatched > 0,
            "every hit must be counted as mispaired, not silently measured"
        );

        // And the same column against its OWN forward still measures.
        let from_short = forward(&short, h(1));
        let right = edge(&column, &from_short, &all);
        assert_eq!(right.mismatched, 0, "the matching pair must not be refused");
        assert!(
            right.n > 0,
            "the matching pair must still produce observations"
        );
    }

    #[test]
    fn an_identical_sample_reports_zero_rather_than_an_infinite_t() {
        // A perfectly flat series: every forward move is the same, so the
        // standard error is zero. Dividing by it would be infinity, which reads
        // as the strongest result ever found rather than as a degenerate one.
        // THE FIXTURE HAD TO CHANGE, and the reason is the point of the test.
        // This was forty hand-made candles. Forty candles never clear the
        // warm-up, so the column came out EMPTY, `edge` returned the default
        // with t = 0, and `is_finite()` was satisfied by a measurement that
        // never happened. The assertion could not fail, which is the shape
        // `CLAUDE.md` §4 refuses.
        //
        // `sessions(8)` sweeps real bars; the prices are then overwritten with
        // a constant +10 ramp on the SAME timestamps, so every close-to-close
        // move is identical, the spread is exactly zero, and the degenerate
        // path this test exists for is actually reached.
        let bars: Vec<Candle> = crate::synthetic::sessions(8)
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let close =
                    100_000_i64.saturating_add(i64::try_from(i).unwrap_or(0).saturating_mul(10));
                Candle::new(
                    c.ts_micros,
                    close,
                    close.saturating_add(10),
                    close.saturating_sub(10),
                    close,
                    100,
                    OI_NULL,
                )
            })
            .collect();
        let column = Column::build(&bars, &mut evaluator());
        assert!(
            !column.is_empty(),
            "the fixture must sweep at least one bar, or this test asserts \
             nothing: an empty column yields t = 0 and `is_finite` passes on a \
             measurement that never happened"
        );
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
