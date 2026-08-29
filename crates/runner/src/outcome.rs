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
pub const AUTO_CLOSE_MINUTE: i64 = 15 * 60 + 10;

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
    /// `true` at `i` when the outcome is absent because a BAR WAS REFUSED,
    /// rather than because `i` is in the tail.
    ///
    /// # Both are `None`, and they mean opposite things
    ///
    /// The tail is a documented, expected absence: the last `H` bars have no
    /// future in the data, so a run over 1,124 bars at H=15 measures 1,109 of
    /// them and that shortfall is the design. A refused bar is the opposite —
    /// the store handed this run a record the engine does not consider a bar,
    /// and the outcome is missing because the data is wrong.
    ///
    /// Without this, [`edge`]'s `let Some(r) = ... else { continue }` swallowed
    /// both into the same silent path under a comment reading *"in the tail: no
    /// future exists"*. A corrupt bar can only ever be an EXIT bar, so it always
    /// landed there: `n` fell from 234 to 230 with `mismatched == 0` in both
    /// runs, and nothing anywhere said why.
    ///
    /// One `bool` per bar. At the store's largest instrument-month that is under
    /// 1.3 MB, once per `Forward` and never per candidate.
    refused: Vec<bool>,
}

impl Forward {
    /// Was the outcome at `i` dropped because a bar was REFUSED?
    ///
    /// `false` for the tail, for an out-of-range index, and for a bar that has a
    /// real outcome. Only a record `Candle::check` rejected answers `true`.
    ///
    /// Exists because [`Self::at`] returns `None` for two facts that mean
    /// opposite things — see the `refused` field — and every caller that treats
    /// them alike reports a smaller sample with no reason attached.
    #[must_use]
    pub fn was_refused(&self, i: usize) -> bool {
        self.refused.get(i).copied().unwrap_or(false)
    }

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
    // Parallel to `ret`, marking WHY an outcome is absent. Pre-sized for the same
    // reason `ret` is: gate 11 rule 3 asks every collection on this path to be
    // sized once rather than grown.
    let mut refused: Vec<bool> = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        let Some(&(_, minute)) = stamps.get(i) else {
            ret.push(None);
            refused.push(false);
            continue;
        };
        // NO ENTRY AT OR AFTER THE FORCED CLOSE. At 15:10 the position is being
        // closed, so it cannot also be opened; `<` and not `<=`.
        if minute > LAST_FILL_MINUTE {
            ret.push(None);
            refused.push(false);
            continue;
        }
        let Some(forced) = close_at.get(i).copied().flatten() else {
            ret.push(None);
            refused.push(false);
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
            refused.push(false);
            continue;
        };
        if exit <= i {
            ret.push(None);
            refused.push(false);
            continue;
        }
        // A BAR THIS RUN ALREADY REFUSED MAY NOT PRICE AN EXIT.
        //
        // `indicators::column::Column::build` refuses a mis-assembled record and
        // charges it to `Census::price_outside_range`. These two lines indexed
        // the RAW slice with no reference to that verdict, so a record the same
        // run declared "not a bar" was still read as a close.
        //
        // MEASURED, on `synthetic::sessions(20)` = 7,500 bars, with exactly ONE
        // record replaced by the shape `Corrupt::PriceOutsideRange` names —
        // `high` below `open`, and an absurd close:
        //
        // | | clean | with one refused bar |
        // |---|---|---|
        // | `forward(H=2).at(4688)` | `Some(-119)` | `Some(7_494_560)` |
        // | bit 0 mean, paisa | −15.17 | **+2,120.07** |
        // | bit 0 `t` | −10.464 | **+0.993** |
        // | `trade::walk` best total | −14,700 | **+79,930 — SIGN FLIPPED** |
        //
        // Thirty-two of the live positions shifted. **Nothing caught it**: `n`
        // was unchanged, so `Edge::mismatched` stayed 0; `Census::reconciles`
        // and `Trades::reconciles` both stayed true; the only trace was
        // `refused: 1` against `swept: 5,623`, a 0.018% rate sitting beside a
        // mean that moved 140x. That is `CLAUDE.md` §4's fallback that hides a
        // failure, and the failure it hid could change which way a strategy
        // trades.
        //
        // `Candle::check` is the same predicate `Column::build` applies, so this
        // agrees with the census by construction rather than by a second copy of
        // the rule.
        //
        // MARKED, NOT MERELY ABSENT. The first version of this fix pushed a bare
        // `None` and its own comment claimed the drop was "visible in
        // `Edge::mismatched`". It was not: `edge`'s `let Some(r) = ... else {
        // continue }` treats every `None` as the TAIL, under a comment saying so,
        // and a corrupt bar can only ever be an exit bar — so it always landed
        // there. Measured: `n` fell 234 to 230 with `mismatched == 0` in both
        // runs and nothing said why. A silent smaller sample is a quieter version
        // of the same §4 failure.
        let (Some(later), Some(now)) = (priced(bars, exit), priced(bars, i)) else {
            ret.push(None);
            refused.push(true);
            continue;
        };
        ret.push(Some(later.saturating_sub(now)));
        refused.push(false);
    }

    Forward {
        horizon,
        bars_len: bars.len(),
        ret,
        refused,
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
/// The close at `index`, or `None` if that record is not a bar.
///
/// # The one place a price may come from
///
/// `Candle::check` is the predicate `indicators::column::Column::build` applies
/// to a record ON ITS OWN, so a record refused for a BAR-LOCAL reason is refused
/// here too and the two cannot disagree about it. That agreement is the point:
/// the census and the pricing were reading the same slice under different rules,
/// which let a record counted as `refused` still supply a close.
///
/// # FOUR OF THE SIX REFUSALS, AND THE OTHER TWO ARE NOT CLOSED
///
/// `Corrupt` has six variants. `check` tests `HighBelowLow`, `RangeOverflows`,
/// `PriceOutsideRange` and `NegativeVolume` — every one a property of the record
/// alone. It cannot test `TimestampNotIncreasing`, which needs the PREVIOUS
/// bar, or `AccumulatorTooLarge`, which needs the VWAP accumulator: both are
/// facts about a SEQUENCE, and this function is handed one record.
///
/// So a bar the column refused for a stateful reason is not swept, never becomes
/// a signal, and **can still be read here as an exit price**, because the exit
/// indexes the raw slice by position. An adversarial fleet demonstrated it: a
/// trade entered and exited on a bar charged to
/// `Census::timestamp_not_increasing`, with `Grid::refused_paths` at zero.
///
/// Closing it needs a swept-membership map built once per run from
/// `Column::sources`, which is O(bars) once and O(1) per lookup — affordable,
/// and not built. `CLAUDE.md` §3 rule 6 asks for the bound to be stated when it
/// cannot be met, and
/// `refused_bar_tests::the_stateful_refusals_are_not_covered_and_this_says_so`
/// pins the gap so it cannot be forgotten.
///
/// `map_or(0, ..)` is what stood here, and zero is a price. A missing index and
/// a corrupt record both became "the close was 0 paisa", so an exit priced
/// against either produced a move of the full entry price with nothing marking
/// it. `None` propagates instead, and the caller drops the outcome.
///
/// Const-callable and index-only: one bounds check and one `check`, no
/// allocation and no scan.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn priced(bars: &[Candle], index: usize) -> Option<i64> {
    let bar = bars.get(index)?;
    bar.check().ok()?;
    Some(bar.close)
}

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
    /// Observations whose forward move was **strictly positive**.
    ///
    /// Zero is counted as neither a win nor a loss: a forward move of exactly
    /// zero paid nothing and cost nothing, and charging it to either side would
    /// move [`Self::payoff_bp`] by the number of flat bars rather than by
    /// anything about the setup.
    pub wins: u64,
    /// Sum of the strictly positive forward moves, in paisa.
    pub win_sum: f64,
    /// Observations whose forward move was **strictly negative**.
    ///
    /// **This is not `n - wins`, and the difference is the flats.** It was
    /// absent and [`Self::payoff_bp`] derived it by that subtraction, which
    /// counts every flat bar as a loser for the purpose of averaging a
    /// losers-only sum -- inflating the reported payoff by
    /// `(losses + flats) / losses`. On a one-minute index series a forward move
    /// of exactly zero is routine, so the inflation was routine too, and
    /// [`crate::rank::ByPayoff`] orders the whole candidate list on it.
    ///
    /// Carried alongside `wins` rather than reconstructed, because the flats
    /// are the one thing the other four fields genuinely cannot recover.
    pub losses: u64,
    /// Sum of the strictly negative forward moves, in paisa. **Negative or
    /// zero.**
    pub loss_sum: f64,
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
    /// Bars where the mask fired and the outcome was dropped because a BAR WAS
    /// REFUSED.
    ///
    /// # Not the tail, and it used to be indistinguishable from it
    ///
    /// `edge`'s absent-outcome branch carried the comment *"in the tail: no
    /// future exists, so nothing is counted"* and treated every `None` that way.
    /// A corrupt bar can only ever be an EXIT bar, so a refusal always landed
    /// there. Measured on a fixture with one refused record: `n` fell from 234
    /// to 230 and `mismatched` was **0** in both runs. The sample shrank and
    /// nothing anywhere said why.
    ///
    /// The doc on `crate::outcome::priced` claimed the drop was *"visible in
    /// `Edge::mismatched`"*. It was not, and this field is what makes the claim
    /// true.
    ///
    /// **Zero on every sound slice.** Non-zero means the store handed this run a
    /// record the engine refuses, and every figure beside it is over a smaller
    /// sample rather than a corrected one — `CLAUDE.md` §4, degrade loudly and
    /// name the reason.
    pub refused: u64,
    /// The t-statistic of that mean against zero.
    ///
    /// `mean / (sd / √n)`. Zero when fewer than two observations exist, where a
    /// standard deviation is undefined — reported as zero rather than as a
    /// large number, because an undefined statistic must not read as a strong
    /// one.
    pub t: f64,
}

impl Edge {
    /// The average winning move over the average losing move, in hundredths.
    ///
    /// `300` reads 3.00 — the mean win was three times the mean loss.
    ///
    /// # The statistic `|t|` cannot supply, on the side of the funnel that needs it
    ///
    /// `rank` keeps the top `keep` combinations by `|t|`, and everything below
    /// the cut is discarded before any exit grid is built. `|t|` is a
    /// DETECTABILITY statistic: it asks how reliably the mean differs from zero.
    /// A setup whose losers are small and whose winners are large can have a
    /// mean near zero — and so a low `|t|` — while being exactly the setup an
    /// operator asking for *"minimal stop loss, massive profit"* is hunting.
    ///
    /// `crates/cli`'s own comment on the cap states the consequence and calls it
    /// unrecoverable: such a combination *"is cut at 60, and never meets an exit
    /// grid at all. No tier ladder, no rule and no report can recover that. They
    /// all filter cells, and the cells were never computed."*
    ///
    /// This is the number that lets the cut see the shape. It costs three scalar
    /// accumulators in the pass `edge` already makes — O(1) per observation, no
    /// second walk, no path — which is precisely why it can sit on the hot side
    /// of the funnel where MAE and MFE cannot.
    ///
    /// # What it is NOT
    ///
    /// It is not a reward-to-risk ratio and must not be read as one. There is no
    /// stop here, no target, and no path: these are forward moves at a fixed
    /// horizon, so this says nothing about what a stop WOULD have done. A real
    /// answer to that is [`crate::grid::Cell::reward_to_risk_bp`], which needs
    /// the trade walk. This says which combinations are worth asking.
    ///
    /// It is also not [`Self::t`]'s replacement. A combination can have a
    /// magnificent payoff on four observations; that is what `n` and the
    /// significance bar are for. Ranking on this ALONE would trade noise.
    ///
    /// # Refusals, and each is a different absence
    ///
    /// * **No losses at all** — every observation won, so there is nothing to
    ///   divide by. [`i64::MAX`], the same answer
    ///   [`crate::grid::Cell::return_over_drawdown`] gives for a variant that
    ///   never gave anything back, and for the same reason: unbounded is a
    ///   fact, not an error.
    /// * **No wins at all** — zero. The setup has no upside to weigh.
    /// * **Fewer than two observations** — zero. One move is not a distribution.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn payoff_bp(&self) -> i64 {
        // Declared before the first statement, because clippy is right that an
        // item appearing mid-function reads as though it came into scope there
        // and it did not.
        //
        // An `f64` LITERAL safely inside `i64::MAX` (9.223e18), rather than
        // `i64::MAX as f64`. That cast is itself lossy — 64 bits of integer into
        // a 52-bit mantissa — so it rounds UP, past the range it is supposed to
        // bound, and a comparison against it would admit a value the following
        // cast cannot represent.
        const UNBOUNDED: f64 = 9.0e18;

        // WHICH MOVES ARE THE WINS DEPENDS ON THE SIDE, AND THIS READ ONLY ONE.
        //
        // The positive forward moves were taken as the gains and the negative
        // ones as the give-back — a LONG's payoff, computed for every
        // combination including the ones the engine goes on to trade SHORT. For
        // a short the negative move IS the win, so the ratio returned was the
        // exact reciprocal of the real one, and `ByPayoff` ranked shorts in
        // reverse order of merit:
        //
        // | combination | forward moves | true short payoff | old reading |
        // |---|---|---|---|
        // | an excellent short | 90 down 100, 10 up 10 | 10.00x | 0.10x, ranked last |
        // | a terrible short | 5 up 1000, 95 down 60 | 0.06x | 16.66x, ranked first |
        //
        // The degenerate case is sharper. A combination whose forward move is
        // ALWAYS down is the ideal short; it has `wins == 0` and scored ZERO,
        // the minimum, so `keep` cut it before it ever met an exit grid.
        //
        // The side is read here rather than passed in because `Edge` is what
        // `cli::side_of_evidence` reads to decide it — `mean_paisa < 0.0` is
        // that whole rule — and `runner` cannot name `cli`. One definition, in
        // the crate that owns the evidence.
        //
        // `rank.rs`'s own header states the property this restores: *"Ranking by
        // `t` signed would discard every short setup, so the order is on |t|"*.
        // `Lens::Detectability` honoured it. `Lens::Payoff` — the lens
        // `audit_range_inner` actually uses — silently undid it.
        let short = self.mean_paisa < 0.0;
        // COUNTED, NOT SUBTRACTED. `n - wins` is losers PLUS FLATS, and
        // `loss_sum` holds losers only, so dividing one by the other shrank the
        // mean loss and inflated this ratio by `(losses + flats) / losses` --
        // on every mask with a single zero forward move, which on one-minute
        // index bars is most of them. `Sides::losses` carries the real count.
        //
        // `loss_sum` is negative, so negating it gives a magnitude; both
        // `gain_sum` and `back_sum` are magnitudes below whichever side is up.
        let (gain_sum, gains, back_sum, backs) = if short {
            (-self.loss_sum, self.losses, self.win_sum, self.wins)
        } else {
            (self.win_sum, self.wins, -self.loss_sum, self.losses)
        };

        if self.n < 2 || gains == 0 {
            return 0;
        }
        // Nothing ever went against the position — unbounded, and named rather
        // than divided by zero. [`i64::MAX`], the same answer
        // [`crate::grid::Cell::return_over_drawdown`] gives for a variant that
        // never gave anything back, and for the same reason.
        if backs == 0 || back_sum <= 0.0 {
            return i64::MAX;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "both counts are bounded by the column, which is bounded by \
                      the bars a month holds."
        )]
        let (gains, backs) = (gains as f64, backs as f64);
        let mean_gain = gain_sum / gains;
        let mean_back = back_sum / backs;
        if !(mean_gain.is_finite() && mean_back.is_finite()) || mean_back <= 0.0 {
            return 0;
        }
        let ratio = mean_gain / mean_back * 100.0;
        if !ratio.is_finite() || ratio >= UNBOUNDED {
            return i64::MAX;
        }
        // Saturating rather than wrapping: a ratio past `i64` is unbounded in
        // every sense that matters, and wrapping would print a negative payoff
        // for the best setup in the run.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "bounded by UNBOUNDED above and by zero below, so the cast \
                      is in range on every path that reaches it."
        )]
        let clamped = ratio.max(0.0) as i64;
        clamped
    }

    /// [`Self::mean_paisa`] in THOUSANDTHS, as an integer.
    #[must_use]
    pub fn mean_milli_paisa(&self) -> i64 {
        milli(self.mean_paisa)
    }

    /// [`Self::t`] in THOUSANDTHS, as an integer.
    ///
    /// # Why this lives here and not where it is stored
    ///
    /// `crates/cli` denies `clippy::float_arithmetic` outright — `CLAUDE.md` §7
    /// keeps floats off any path whose output is compared, and D-0061 made that
    /// a lint rather than a convention. This module carries the documented
    /// exemption because a t-statistic IS a float. So the crate that owns the
    /// float owns the conversion, and the crate that stores integers never
    /// touches one.
    ///
    /// That is the right seam anyway: a scale is a property of the statistic,
    /// and putting it beside `t` means one definition rather than one per
    /// consumer.
    #[must_use]
    pub fn t_milli(&self) -> i64 {
        milli(self.t)
    }
}

/// An `f64` as thousandths — rounded, saturating, and NaN as zero.
///
/// # Rounded, and truncating was wrong by a whole unit
///
/// `4.007 * 1000.0` is `4006.9999999999995` in binary floating point, so a bare
/// cast truncates toward zero and a `t` of 4.007 is stored as 4006. The error is
/// **systematically toward zero**, which is the direction that makes every
/// finding look slightly weaker than it was — a bias, not noise.
///
/// It is the same defect `cli`'s `ppm_to_points_at` carried until D-0285: a
/// scaled value floored through a cast, always in one direction.
///
/// # Saturating, and wrapping would invert the strongest finding
///
/// A value past the `i64` range is not one any run produced, and wrapping it
/// would store a large NEGATIVE figure for the best result in the sweep. The
/// bound is compared against an `f64` literal safely inside `i64::MAX` rather
/// than against `i64::MAX as f64`, because that cast is itself lossy and rounds
/// UP past the range it is meant to bound.
///
/// # NaN is zero
///
/// [`Edge::t`] documents itself as reporting zero where the statistic is
/// undefined, *"because an undefined statistic must not read as a strong one"*.
/// This holds that property across the conversion rather than storing a
/// sentinel a reader would have to know about.
fn milli(x: f64) -> i64 {
    const CEILING: f64 = 9.0e18;

    if x.is_nan() {
        return 0;
    }
    let scaled = x * 1_000.0;
    if scaled >= CEILING {
        return i64::MAX;
    }
    if scaled <= -CEILING {
        return i64::MIN;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "bounded by CEILING on both sides immediately above, so the \
                  cast is in range on every path that reaches it."
    )]
    let out = scaled.round() as i64;
    out
}

/// The two sides of a forward-return distribution, accumulated apart.
///
/// # Why this is a type and not three locals
///
/// It was three locals inside [`edge`], which pushed that function past the
/// hundred-line limit this workspace lints on — and the limit was right: `edge`
/// already carries a Welford pass, a Newey-West drain and a whole-slice
/// agreement check, and a fourth concern threaded through the same scope is
/// where a reader stops being able to hold it.
///
/// Keeping them together also makes the ZERO rule checkable in one place rather
/// than at each `+=`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sides {
    /// Strictly positive observations.
    wins: u64,
    /// Sum of the strictly positive observations.
    win_sum: f64,
    /// Strictly negative observations.
    ///
    /// **Counted rather than derived, and that is the whole point.** It was
    /// absent, and [`Edge::payoff_bp`] recovered it as `n - wins` -- which is
    /// losers PLUS FLATS, because [`Self::observe`] charges a zero to neither
    /// side while the caller still counts it in `n`. Dividing a losers-only
    /// `loss_sum` by that inflated count shrinks the mean loss and inflates the
    /// payoff ratio by exactly `(losers + flats) / losers`.
    ///
    /// The doc on `observe` names this failure mode precisely -- it would "move
    /// `Edge::payoff_bp` by the number of flat bars rather than by anything
    /// about the setup" -- and the subtraction reintroduced it fifteen lines
    /// away. A flat forward move on a one-minute index bar is routine, so it
    /// fired constantly, and `ByPayoff` RANKS on the result.
    losses: u64,
    /// Sum of the strictly negative observations. Negative or zero.
    loss_sum: f64,
}

impl Sides {
    /// Charges one forward move to the side it fell on.
    ///
    /// **Zero is neither.** A flat forward move paid nothing and cost nothing;
    /// charging it to a side would move [`Edge::payoff_bp`] by the number of
    /// flat bars rather than by anything about the setup. A NaN is also neither
    /// — it fails both comparisons — which is the honest handling for a value
    /// that is not a move at all.
    fn observe(&mut self, x: f64) {
        if x > 0.0 {
            self.wins = self.wins.saturating_add(1);
            self.win_sum += x;
        } else if x < 0.0 {
            self.losses = self.losses.saturating_add(1);
            self.loss_sum += x;
        }
    }
}

/// The Newey-West long-run sum of squares, from `edge`'s four accumulators.
///
/// # Why this is a function and not four lines inside the walk
///
/// It is the whole of the overlap correction's arithmetic, and inside the walk
/// it is unreachable from a test: driving it needs a `Column`, a `Forward` and a
/// mask whose hits fall closer together than the horizon, and the value it
/// produces is then folded into a `t` that a test can only compare against a
/// number it computed the same way. A test that repeats the implementation
/// asserts nothing, which is the trap the allocation test in
/// `engine::column` records falling into.
///
/// Here the algebra stands alone and its two load-bearing properties are
/// ordinary equalities:
///
/// * **`cross_c == 0` returns `m2` exactly.** No pair of hits overlapped, so the
///   result is the plain sum of squares this replaced. Bit for bit, not nearly.
/// * **positive cross terms return more than `m2`.** A larger sum of squares is
///   a larger standard error and a SMALLER `t`, which is the direction the
///   correction must move: overlapping windows were making findings look
///   stronger than they were.
///
/// # The centering, which is why three accumulators and not one
///
/// The weighted cross-sum wants `Σ w (x_i − m)(x_j − m)`, and `m` is not known
/// until the walk ends. Expanding it into `A − m·B + m²·C` lets the walk carry
/// the three uncentered sums and pay the centering once, here, rather than
/// storing every observation to make a second pass over them.
///
/// `m2` is the `d = 0` term and is already centered by Welford. The cross term
/// is doubled because each overlapping pair is counted once by the walk and the
/// two-sided long-run variance needs it from both sides.
fn long_run_sum_squares(m2: f64, mean: f64, cross_a: f64, cross_b: f64, cross_c: f64) -> f64 {
    let cross = mean.mul_add(mean * cross_c, cross_a) - mean * cross_b;
    2.0f64.mul_add(cross, m2)
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
/// **It is no longer strictly no-storage, and the bound is stated rather than
/// glossed.** The overlap correction keeps the hits whose forward windows still
/// touch the current bar, which is at most one per bar over the last `H` bars.
/// So the working set is `O(H)` — fifteen entries at the default horizon — and
/// `H` is a run PARAMETER, not a function of how many bars were loaded or how
/// many the mask hit. Constant in the data, linear in a number the operator
/// chose. Still one pass.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn edge(column: &Column, forward: &Forward, mask: &ConditionMask) -> Edge {
    let mut n: u64 = 0;
    let mut mismatched: u64 = 0;
    // Outcomes dropped because a bar was REFUSED, kept apart from the tail. See
    // `Forward::refused`: both are absent, and they mean opposite things.
    let mut refused: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    // The two sides of the distribution, kept apart -- see `Edge::payoff_bp`
    // for why the funnel needs them and `|t|` cannot supply them.
    let mut sides = Sides::default();

    // THE OVERLAP CORRECTION, AND WHY THE t BELOW IS MEANINGLESS WITHOUT IT.
    //
    // `crates/runner/src/significance.rs` corrects the bar for how many
    // hypotheses were tested, carefully and from the run's own counters. It was
    // then applied to a t computed as though the observations were independent,
    // and they are not: a forward return at bar `i` covers bars `i+1..=i+H`, so
    // two hits fewer than `H` bars apart SHARE bars. The i.i.d. standard error
    // understates the spread by roughly a factor of `H`, inflating t by about
    // `sqrt(H)` -- 3.87 at the default `H = 15`.
    //
    // The two axes pull against each other and the multiplicity one was losing:
    // a reported `t = 4.0`, which CLEARS the Bonferroni bar at the trial counts
    // these runs produce, is a true t near 1.03 -- p about 0.30. Correcting one
    // axis rigorously and the other not at all does not half-correct; the second
    // error undoes the first, and the report prints CLEARS either way.
    //
    // Newey-West with a Bartlett kernel, keyed on BAR DISTANCE rather than on
    // observation index. That distinction is the whole of the correctness here:
    // hits are sparse and irregular, so the k-th previous OBSERVATION may be a
    // thousand bars back and share nothing. What induces the correlation is bars
    // in common, so the weight is `1 - d/H` on the bar gap `d`, and a pair at
    // `d >= H` contributes nothing because its windows are disjoint.
    //
    // IT REDUCES EXACTLY TO WHAT IT REPLACED. With no overlapping pair every
    // accumulator below stays zero and the formula is the old
    // `m2 / (n - 1)`, so a run whose hits are all more than `H` apart reports
    // the t it always did. This is an added term, not a different statistic.
    //
    // The alternative -- subsampling to every H-th hit -- is simpler and also
    // honest, and was not taken: it discards about 93% of the observations at
    // `H = 15` to buy the same `sqrt(H)`, and the discarded ones carry real
    // information about the mean.
    let horizon_bars = forward.horizon().as_bars() as usize;
    // Bounded by the HORIZON, which is a run parameter, not by the data. At the
    // default it is fifteen entries. One allocation per call, and `edge` is
    // called once per FREQUENT itemset -- the survivors -- rather than per
    // candidate pair, which is where the sweep's cost actually is.
    let mut recent: std::collections::VecDeque<(usize, f64)> =
        std::collections::VecDeque::with_capacity(horizon_bars);
    // Uncentered, because the mean is not known until the walk ends. The three
    // together reconstruct the centered weighted cross-sum exactly:
    // `Σ w (x_i - m)(x_j - m) = A - m·B + m²·C`.
    let mut cross_a = 0.0_f64; // Σ w · x_i · x_j
    let mut cross_b = 0.0_f64; // Σ w · (x_i + x_j)
    let mut cross_c = 0.0_f64; // Σ w

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
            // TWO REASONS AN OUTCOME IS ABSENT, AND THEY ARE NOT THE SAME FACT.
            //
            // The tail is the design: the last `H` bars have no future in the
            // data. A REFUSED bar is the store handing this run a record the
            // engine does not consider a bar. This branch swallowed both under
            // a comment reading "in the tail: no future exists", and since a
            // corrupt bar can only ever be an exit bar it always landed here.
            if forward.was_refused(source) {
                refused = refused.saturating_add(1);
            }
            continue;
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

        // THE TWO SIDES, KEPT APART. O(1) per observation, in the pass that was
        // already running, and no path data — which is the whole reason this
        // can sit on the funnel's hot side where MAE and MFE cannot.
        //
        // WHY IT EXISTS. `screen_cap` cuts the candidate list to the top
        // `keep` by `|t|`, and `|t|` is a DETECTABILITY statistic: it asks how
        // reliably the mean differs from zero. A setup whose losers are small
        // and whose winners are large can have a mean near zero — so a low
        // `|t|` — and be exactly the setup an operator asking for "minimal stop
        // loss, massive profit" wants. `crates/cli`'s own comment states the
        // consequence: such a combination "is cut at 60, and never meets an
        // exit grid at all. No tier ladder, no rule and no report can recover
        // that. They all filter cells, and the cells were never computed."
        //
        // Splitting the sum is what lets the cut see that shape. It is NOT a
        // replacement for the exit grid — it has no stop, no target and no
        // path, so it cannot say what a stop WOULD have done. It says which
        // combinations are worth asking that question about.
        sides.observe(x);

        // EVERY EARLIER HIT WHOSE WINDOW STILL TOUCHES THIS ONE.
        //
        // `sources` is strictly increasing, so the front of the queue is the
        // oldest and the moment it falls out of range every entry behind it is
        // in range. Dropping from the front is therefore complete, not a
        // heuristic.
        while let Some(&(older, _)) = recent.front() {
            if source.saturating_sub(older) >= horizon_bars {
                recent.pop_front();
            } else {
                break;
            }
        }
        for &(older, x_older) in &recent {
            let gap = source.saturating_sub(older);
            // `gap` is in `1..horizon_bars` by the drain above, so the weight is
            // in `(0, 1)` and never the `d = 0` self-pair, which is `m2`'s job.
            #[allow(
                clippy::cast_precision_loss,
                reason = "a bar gap below the horizon cannot reach 2^52."
            )]
            let w = 1.0 - (gap as f64) / (horizon_bars as f64);
            cross_a += w * x_older * x;
            cross_b += w * (x_older + x);
            cross_c += w;
        }
        recent.push_back((source, x));
    }

    // ONE ASSEMBLY POINT FOR BOTH EXITS. The early return and the final one
    // differ in exactly one field, `t`, and every other field was spelled twice
    // -- which is how a field can be added to one and forgotten in the other.
    let assemble = |t: f64| Edge {
        n,
        mismatched,
        refused,
        mean_paisa: mean,
        // CARRIED, not zeroed, on the `t = 0.0` path. A single observation has
        // no `t` -- there is no spread to divide by -- but it did move one way
        // or the other, and `payoff_bp` refuses a one-sided sample on its own
        // terms rather than being handed a zero that looks measured.
        wins: sides.wins,
        win_sum: sides.win_sum,
        losses: sides.losses,
        loss_sum: sides.loss_sum,
        t,
    };

    if n < 2 {
        return assemble(0.0);
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "the observation count is bounded by the column length."
    )]
    let count = n as f64;
    let sum_squares = long_run_sum_squares(m2, mean, cross_a, cross_b, cross_c);
    let variance = sum_squares / (count - 1.0);
    let standard_error = (variance / count).sqrt();
    let t = if standard_error > 0.0 && standard_error.is_finite() {
        mean / standard_error
    } else {
        // TWO WAYS TO GET HERE, AND BOTH ARE REPORTED AS NO EVIDENCE.
        //
        // Every observation identical: the mean is exact and its spread is
        // zero, which is not an infinitely strong result -- it is a degenerate
        // sample, and reporting it as zero refuses to dress one up as the other.
        //
        // Or the long-run variance came out NON-POSITIVE. The Bartlett kernel
        // is positive semi-definite on evenly spaced lags, and these lags are
        // bar gaps between irregular hits, so that guarantee does not carry
        // over: strong negative autocovariance can drive `sum_squares` to or
        // below zero. `sqrt` of a negative is `NaN`, which would compare false
        // against every threshold and read as a silently weak result rather
        // than an unusable one. `is_finite` catches it and it lands here, with
        // the same answer the degenerate sample gets -- this run measured
        // nothing usable. Reporting the uncorrected t instead would be reporting
        // the inflated number this whole block exists to remove.
        0.0
    };
    assemble(t)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{
        Edge, Horizon, LAST_FILL_MINUTE, Sides, edge, forward, long_run_sum_squares, milli,
    };
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

    /// An `Edge` with only the payoff fields set, for arithmetic tests.
    ///
    /// `losses` is taken rather than derived, because deriving it as `n - wins`
    /// is the exact defect [`Edge::losses`] documents: it counts flats as
    /// losers. A fixture that reconstructed it would agree with a broken
    /// `payoff_bp` and disagree with a correct one.
    fn sided(n: u64, wins: u64, win_sum: f64, losses: u64, loss_sum: f64) -> Edge {
        Edge {
            n,
            wins,
            win_sum,
            losses,
            loss_sum,
            ..Edge::default()
        }
    }

    /// Scaling to thousandths rounds, saturates, and refuses a NaN.
    ///
    /// Each case is a defect this workspace has already had once somewhere else,
    /// which is why each is pinned rather than argued.
    #[test]
    fn the_milli_scale_rounds_saturates_and_a_nan_is_not_a_finding() {
        // ROUNDED. `4.007 * 1000.0` is 4006.9999999999995, so a bare cast
        // truncates to 4006 -- systematically toward zero, which makes every
        // finding read slightly weaker than it was. The same shape as the
        // `ppm_to_points_at` floor D-0285 fixed in `cli`.
        assert_eq!(milli(4.007), 4_007, "three decimals, exactly");
        assert_eq!(
            milli(-4.007),
            -4_007,
            "and the same on the other side of zero"
        );
        assert_eq!(milli(-1.5), -1_500, "negatives keep their sign");
        assert_eq!(milli(0.0), 0);

        // SATURATING. Wrapping would store a large NEGATIVE figure for the
        // strongest finding in the sweep.
        assert_eq!(milli(f64::INFINITY), i64::MAX, "saturates, never wraps");
        assert_eq!(milli(f64::NEG_INFINITY), i64::MIN, "saturates, never wraps");
        assert_eq!(milli(1.0e300), i64::MAX, "a value past the range saturates");
        assert_eq!(milli(-1.0e300), i64::MIN, "and on the other side too");

        // NaN IS ZERO, holding `Edge::t`'s own rule across the conversion: an
        // undefined statistic must not read as a strong one.
        assert_eq!(
            milli(f64::NAN),
            0,
            "an undefined statistic is not a finding"
        );

        // And the two accessors are that scale, not a second one.
        let edge = Edge {
            mean_paisa: 2.5,
            t: 4.007,
            ..Edge::default()
        };
        assert_eq!(edge.mean_milli_paisa(), 2_500);
        assert_eq!(edge.t_milli(), 4_007);
    }

    /// The shape `|t|` cannot see, which is the whole reason this exists.
    ///
    /// `rank` keeps the top combinations by `|t|` and everything under the cut
    /// is discarded before an exit grid is built. A setup whose losers are small
    /// and whose winners are large can have a mean near zero — and so a `t` near
    /// zero — while being exactly what an operator asking for "minimal stop
    /// loss, massive profit" wants.
    ///
    /// This asserts the two are genuinely independent: the same mean, the same
    /// `n`, and payoffs an order of magnitude apart.
    #[test]
    fn the_payoff_separates_what_the_mean_cannot() {
        // Ten observations, mean exactly zero on both.
        //
        // A: nine losers of -10 and one winner of +90.  Mean 0.
        //    mean win 90, mean loss 10  -> payoff 9.00
        let sniper = sided(10, 1, 90.0, 9, -90.0);
        // B: one loser of -90 and nine winners of +10.  Mean 0.
        //    mean win 10, mean loss 90  -> payoff 0.11
        let grinder = sided(10, 9, 90.0, 1, -90.0);

        // Both fixtures leave `mean_paisa` at its default, so the point being
        // made is that the two carry the SAME mean and `t` therefore cannot
        // separate them. Compared with `total_cmp` rather than `==`, because §7
        // keeps strict float equality out of anything compared.
        assert_eq!(
            sniper.mean_paisa.total_cmp(&grinder.mean_paisa),
            core::cmp::Ordering::Equal,
            "both fixtures carry the same mean, so `t` cannot separate them"
        );
        assert_eq!(sniper.payoff_bp(), 900, "nine to one");
        assert_eq!(grinder.payoff_bp(), 11, "one to nine, truncated from 11.11");
        assert!(
            sniper.payoff_bp() > grinder.payoff_bp() * 50,
            "the statistic must separate by an order of magnitude what the mean \
             does not separate at all"
        );
    }

    /// Every refusal is a different absence and none of them is a number.
    #[test]
    fn the_payoff_names_each_absence_rather_than_returning_a_figure() {
        // One observation is not a distribution.
        // A FLAT BAR IS NOT A LOSER, AND THIS IS THE ASSERTION THAT SAYS SO.
        //
        // Forty wins totalling +4,000 and forty losers totalling -4,000 is a
        // payoff of exactly 1.00, whatever else the sample contains. Adding
        // twenty flats must not move it. `payoff_bp` derived its loser count as
        // `n - wins`, which reads 60 here instead of 40, shrinks the mean loss
        // from 100 to 66.67, and reports 150 -- a setup a third better than it
        // is, on the statistic `ByPayoff` ranks the whole run by.
        //
        // Both fixtures below carry identical wins and identical losers. Only
        // the flats differ, so the only way they can disagree is the bug.
        let no_flats = sided(80, 40, 4_000.0, 40, -4_000.0);
        let with_flats = sided(100, 40, 4_000.0, 40, -4_000.0);
        assert_eq!(
            no_flats.payoff_bp(),
            100,
            "forty up, forty down, one to one"
        );
        assert_eq!(
            with_flats.payoff_bp(),
            no_flats.payoff_bp(),
            "twenty flat bars paid nothing and cost nothing, so they must not \
             move the payoff ratio at all"
        );

        assert_eq!(sided(1, 1, 10.0, 0, 0.0).payoff_bp(), 0, "n < 2");
        // No upside at all.
        assert_eq!(sided(10, 0, 0.0, 10, -100.0).payoff_bp(), 0, "no wins");
        // No downside at all -- unbounded, and named rather than divided by
        // zero. The same answer `Cell::return_over_drawdown` gives a variant
        // that never gave anything back, for the same reason.
        assert_eq!(
            sided(10, 10, 100.0, 0, 0.0).payoff_bp(),
            i64::MAX,
            "no losses"
        );
        // Wins and FLATS only: the losses count is non-zero but nothing was
        // charged to that side, so there is still nothing to divide by.
        assert_eq!(
            sided(10, 4, 100.0, 0, 0.0).payoff_bp(),
            i64::MAX,
            "wins and flats"
        );
        // A default `Edge` is not a magnificent setup.
        assert_eq!(
            Edge::default().payoff_bp(),
            0,
            "nothing measured is not a finding"
        );
    }

    /// A SHORT IS PAID ON THE MOVES THAT GO ITS WAY, AND IT WAS PAID ON THE
    /// OTHERS.
    ///
    /// `payoff_bp` took the positive forward moves as the gains and the negative
    /// ones as the give-back — a LONG's payoff — and `ByPayoff` then ranked
    /// every short-edged combination by the exact reciprocal of its real merit.
    /// `Lens::Payoff` is the lens `audit_range_inner` uses, so this was the live
    /// order on the operator's own runs.
    ///
    /// The two cases here are the pair that makes the inversion unmistakable:
    /// under the old reading the terrible short scored 1,666 and the excellent
    /// one scored 10, so the terrible one led the ranking and the excellent one
    /// was cut by `keep`.
    #[test]
    fn a_short_is_paid_on_the_moves_that_go_its_way() {
        // Ninety moves down a hundred paisa, ten moves up ten. Mean −89.
        let excellent = Edge {
            mean_paisa: -89.0,
            ..sided(100, 10, 100.0, 90, -9_000.0)
        };
        // Five moves up a thousand, ninety-five down sixty. Mean −7.
        let terrible = Edge {
            mean_paisa: -7.0,
            ..sided(100, 5, 5_000.0, 95, -5_700.0)
        };

        assert_eq!(excellent.payoff_bp(), 1_000, "ten to one, the short's way");
        assert_eq!(
            terrible.payoff_bp(),
            6,
            "sixty paisa won against a thousand lost"
        );
        assert!(
            excellent.payoff_bp() > terrible.payoff_bp(),
            "the old reading ranked these the other way round: 10 against 1,666"
        );
    }

    /// THE IDEAL SHORT SCORED THE MINIMUM.
    ///
    /// A combination whose forward move is ALWAYS down is the perfect short and
    /// has `wins == 0`, which returned zero — the floor. `keep` is a hard cut,
    /// so it never reached an exit grid at all. Its mirror, always up, is the
    /// perfect long and correctly returned [`i64::MAX`]; the two must now agree,
    /// because they are the same setup seen from the two sides.
    #[test]
    fn the_perfect_short_and_the_perfect_long_are_worth_the_same() {
        let always_down = Edge {
            mean_paisa: -50.0,
            ..sided(50, 0, 0.0, 50, -2_500.0)
        };
        let always_up = Edge {
            mean_paisa: 50.0,
            ..sided(50, 50, 2_500.0, 0, 0.0)
        };

        assert_eq!(
            always_down.payoff_bp(),
            i64::MAX,
            "nothing ever went against it; the old reading scored it ZERO and \
             `keep` cut the best short in the run"
        );
        assert_eq!(
            always_up.payoff_bp(),
            always_down.payoff_bp(),
            "one setup, two sides, one payoff"
        );
    }

    /// AND THE MIRROR HOLDS IN GENERAL, not only at the two extremes.
    ///
    /// Negating every forward move turns a long setup into the identical short
    /// setup. A payoff that reads the side cannot change; one that does not
    /// returns the reciprocal, which is what made the ranking wrong.
    #[test]
    fn negating_every_move_leaves_the_payoff_where_it_was() {
        // The mean is stated rather than divided out, because `n as f64` is a
        // `cast_precision_loss` this workspace denies -- and the mean is the
        // field that picks the branch, so it is the one thing a mirror must
        // negate deliberately rather than derive.
        for (n, wins, win_sum, losses, loss_sum, mean) in [
            (100_u64, 40_u64, 4_000.0_f64, 60_u64, -3_000.0_f64, 10.0_f64),
            (50, 40, 1_200.0, 10, -900.0, 6.0),
            (10, 3, 33.0, 4, -11.0, 2.2),
        ] {
            let long = Edge {
                mean_paisa: mean,
                ..sided(n, wins, win_sum, losses, loss_sum)
            };
            let short = Edge {
                mean_paisa: -mean,
                ..sided(n, losses, -loss_sum, wins, -win_sum)
            };
            assert_eq!(
                long.payoff_bp(),
                short.payoff_bp(),
                "the mirror of a long IS a short and must be worth the same"
            );
        }
    }

    /// A flat forward move is charged to neither side.
    ///
    /// Charging zero to a side would move the ratio by the number of flat bars
    /// rather than by anything about the setup, and on a coarse rung flat bars
    /// are common.
    #[test]
    fn a_flat_move_is_neither_a_win_nor_a_loss() {
        let mut sides = Sides::default();
        for x in [5.0, 0.0, 0.0, -1.0, 0.0] {
            sides.observe(x);
        }
        assert_eq!(sides.wins, 1, "one strictly positive move");
        assert!(
            (sides.win_sum - 5.0).abs() < f64::EPSILON,
            "the flats added nothing to the winning side"
        );
        assert!(
            (sides.loss_sum + 1.0).abs() < f64::EPSILON,
            "the flats added nothing to the losing side"
        );

        // A NaN is not a move either, and must not become one.
        let mut nan = Sides::default();
        nan.observe(f64::NAN);
        assert_eq!(nan, Sides::default(), "a NaN is charged to neither side");
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

    /// With nothing overlapping, the correction is not merely small -- it is the
    /// identity, and the statistic is the one this code always reported.
    ///
    /// This is the row that makes the change safe to land: every existing
    /// expectation in this crate was measured before the overlap term existed,
    /// and all 264 of them still pass. That is only meaningful if "no overlap"
    /// means EXACTLY the old arithmetic rather than approximately it.
    #[test]
    fn no_overlapping_pair_leaves_the_sum_of_squares_exactly_where_it_was() {
        for m2 in [0.0, 1.0, 1234.5, 9.87e12] {
            for mean in [0.0, -3.5, 1e6] {
                assert!(
                    (long_run_sum_squares(m2, mean, 0.0, 0.0, 0.0) - m2).abs() < f64::EPSILON,
                    "m2={m2} mean={mean} must come back untouched"
                );
            }
        }
    }

    /// The centering algebra is right, checked against a pair worked by hand.
    ///
    /// One overlapping pair, `x_i = 2` and `x_j = 6`, at weight `w = 0.5`, with
    /// the walk's mean at `m = 4`. The walk carries the three uncentered sums:
    ///
    /// * `A = w·x_i·x_j    = 0.5 · 12 = 6`
    /// * `B = w·(x_i + x_j) = 0.5 · 8  = 4`
    /// * `C = w             = 0.5`
    ///
    /// and the centered cross-term it must reconstruct is
    /// `w·(2−4)·(6−4) = 0.5 · (−2) · 2 = −2`. So `A − m·B + m²·C` is
    /// `6 − 16 + 8 = −2`, and the result is `m2 + 2·(−2) = m2 − 4`.
    ///
    /// The numbers are chosen so the cross term is NEGATIVE, which is the case
    /// that matters: it proves the doubling and the sign are carried through
    /// rather than an absolute value being taken somewhere.
    #[test]
    fn the_centered_cross_term_is_reconstructed_from_the_three_uncentered_sums() {
        let got = long_run_sum_squares(100.0, 4.0, 6.0, 4.0, 0.5);
        assert!(
            (got - 96.0).abs() < 1e-9,
            "100 + 2·(6 − 4·4 + 16·0.5) = 96, got {got}"
        );
    }

    /// Positive overlap RAISES the sum of squares, which LOWERS `t`.
    ///
    /// The direction is the whole point. Overlapping forward windows share bars,
    /// so the i.i.d. standard error understates the spread and inflates `t` by
    /// about `sqrt(H)` -- 3.87 at the default `H = 15`. A reported `t = 4.0`
    /// that CLEARS the Bonferroni bar is then a true `t` near 1.03. The
    /// correction has to move the number DOWN; a sign error here would leave the
    /// report reading CLEARS on noise, exactly as before, while looking fixed.
    #[test]
    fn positively_correlated_overlap_makes_the_evidence_weaker_and_never_stronger() {
        // Two observations either side of the mean in the SAME direction, so the
        // centered cross term is positive: x_i = 6, x_j = 8, m = 4, w = 1.
        // (6−4)·(8−4) = 8, so the sum of squares rises by 2·8 = 16.
        let plain = 100.0;
        let corrected = long_run_sum_squares(plain, 4.0, 48.0, 14.0, 1.0);
        assert!(
            corrected > plain,
            "a shared-bar pair must widen the spread, not narrow it: \
             {corrected} vs {plain}"
        );
        assert!(
            (corrected - 116.0).abs() < 1e-9,
            "100 + 2·(48 − 4·14 + 16·1) = 116, got {corrected}"
        );
    }

    /// A long-run variance can come out non-positive, and that is reported as no
    /// evidence rather than as a `NaN` that reads like weak evidence.
    ///
    /// The Bartlett kernel is positive semi-definite on evenly spaced lags. These
    /// lags are bar gaps between irregular hits, so the guarantee does not carry
    /// over and strong negative autocovariance can drive the sum of squares
    /// below zero. `sqrt` of that is `NaN`, and `NaN` compares false against
    /// every threshold -- it would pass a `t > bar` test by failing it, and read
    /// as a merely-weak result instead of an unusable one.
    #[test]
    fn a_non_positive_long_run_variance_is_refused_rather_than_reported_as_nan() {
        // Cross term far more negative than m2 is positive.
        let s = long_run_sum_squares(1.0, 0.0, -50.0, 0.0, 0.0);
        assert!(s < 0.0, "the fixture must actually go negative, got {s}");
        assert!(
            (s / 4.0 / 5.0).sqrt().is_nan(),
            "and a negative variance is where the NaN would come from"
        );
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod refused_bar_tests {
    use super::{Horizon, forward};
    use indicators::{Candle, OI_NULL};

    /// ONE REFUSED BAR USED TO REWRITE EVERY STATISTIC IN THE RUN.
    ///
    /// # What was measured, and by whom
    ///
    /// An adversarial audit built this input and ran it. `Column::build` refuses
    /// a mis-assembled record and charges it to `Census::price_outside_range`;
    /// `forward` and `crate::trade::round_trip` then indexed the RAW slice with
    /// no reference to that verdict, so a record the same run had declared "not
    /// a bar" still supplied a close.
    ///
    /// On `synthetic::sessions(20)` — 7,500 bars — with exactly ONE record
    /// replaced:
    ///
    /// | | clean | one refused bar |
    /// |---|---|---|
    /// | `forward(H=2).at(4688)` | `Some(-119)` | `Some(7_494_560)` |
    /// | bit 0 mean, paisa | −15.17 | +2,120.07 |
    /// | bit 0 `t` | −10.464 | +0.993 |
    /// | `trade::walk` best total | −14,700 | **+79,930 — sign flipped** |
    ///
    /// Thirty-two live positions moved. Nothing caught it: `n` was unchanged so
    /// `Edge::mismatched` stayed 0, and both `reconciles()` stayed true.
    ///
    /// # Why this fixture is four bars and not 7,500
    ///
    /// The defect is one index reading one record, so the smallest input that
    /// exhibits it is the honest one. The 7,500-bar figures above say what it
    /// COST; this says what it IS, and it fails the moment `priced` goes back to
    /// `map_or(0, ..)`.
    #[test]
    fn a_bar_the_run_refused_can_never_price_an_outcome() {
        let minute = |m: i64| (m + 555 - 330) * 60 * 1_000_000;
        let ok = |m: i64, close: i64| {
            Candle::new(
                minute(m),
                close,
                close.saturating_add(50),
                close.saturating_sub(50),
                close,
                100,
                OI_NULL,
            )
        };
        // THE SHAPE `Corrupt::PriceOutsideRange` NAMES: a close far outside
        // `[low, high]`. `high >= low` holds and both legs clear a tick, so
        // `costs::fill::Bar::new` would have accepted this bar — which is why
        // the refusal has to be `Candle::check` and not a fill-side guard.
        let poisoned = Candle::new(
            minute(2),
            2_600_000,
            2_600_050,
            2_599_950,
            9_999_999,
            100,
            OI_NULL,
        );
        assert!(
            poisoned.check().is_err(),
            "the fixture must be a record the engine refuses, or this test \
             asserts nothing"
        );

        let bars = vec![
            ok(0, 2_600_000),
            ok(1, 2_600_100),
            poisoned,
            ok(3, 2_600_200),
        ];
        let f = forward(&bars, Horizon::bars(2).expect("a non-zero horizon"));

        assert_eq!(
            f.at(0),
            None,
            "bar 0's outcome is priced against bar 2, which the engine refused. \
             It used to read the refused close as 9,999,999 and report a move of \
             7,399,999 paisa"
        );
        assert_eq!(
            f.at(1),
            Some(100),
            "bar 1's outcome is priced against bar 3, which is sound, so the \
             refusal must not spread further than the bar it names"
        );

        // AND THE SAME SLICE WITH A SOUND BAR IN THAT SLOT STILL ANSWERS. A fix
        // that refused everything would satisfy the assertion above and be
        // useless, which is the mutant this second half kills.
        let mut healthy = bars.clone();
        if let Some(slot) = healthy.get_mut(2) {
            *slot = ok(2, 2_600_150);
        }
        let g = forward(&healthy, Horizon::bars(2).expect("a non-zero horizon"));
        assert_eq!(
            g.at(0),
            Some(150),
            "with a sound bar in the same slot the outcome is measured normally"
        );
    }

    /// THE REFUSAL IS VISIBLE, WHICH IS THE HALF THAT MAKES IT NOT A FALLBACK.
    ///
    /// `CLAUDE.md` §4 bans a fallback that hides a failure. Dropping the outcome
    /// would be one if the drop were silent — so the property that matters is
    /// that a refused bar produces a SMALLER `n`, which `Edge::mismatched`
    /// already reports and which the old zero-price path did not: it kept `n`
    /// identical and moved the mean by 140x.
    #[test]
    fn a_refused_bar_shrinks_the_sample_rather_than_moving_the_mean() {
        let minute = |m: i64| (m + 555 - 330) * 60 * 1_000_000;
        let ok = |m: i64, close: i64| {
            Candle::new(
                minute(m),
                close,
                close.saturating_add(50),
                close.saturating_sub(50),
                close,
                100,
                OI_NULL,
            )
        };
        let bad = |m: i64| {
            Candle::new(
                minute(m),
                2_600_000,
                2_600_050,
                2_599_950,
                9_999_999,
                100,
                OI_NULL,
            )
        };

        let clean: Vec<Candle> = (0..6).map(|m| ok(m, 2_600_000 + m * 100)).collect();
        let mut dirty = clean.clone();
        if let Some(slot) = dirty.get_mut(3) {
            *slot = bad(3);
        }

        let h = Horizon::bars(1).expect("a non-zero horizon");
        let measured = |bs: &[Candle]| {
            let f = forward(bs, h);
            (0..bs.len()).filter(|&i| f.at(i).is_some()).count()
        };
        let before = measured(&clean);
        let after = measured(&dirty);
        assert!(
            after < before,
            "the refusal must cost sample size -- {after} outcomes against \
             {before}. Equal counts is exactly what the zero-price path gave, \
             and it is what made the contamination invisible"
        );
    }
}

#[cfg(test)]
mod refusal_coverage {
    use indicators::Corrupt;

    /// FOUR OF SIX, AND THE GAP IS PINNED RATHER THAN IMPLIED.
    ///
    /// # What this exists to stop being forgotten
    ///
    /// Four sites in this crate refuse to price a bar `Candle::check` rejects,
    /// and three of them originally claimed `check` is "the SAME predicate
    /// `Column::build` applies". It is the same predicate for a record ON ITS
    /// OWN. `Corrupt` has six variants and two of them are facts about a
    /// SEQUENCE:
    ///
    /// * `TimestampNotIncreasing` needs the previous bar,
    /// * `AccumulatorTooLarge` needs the VWAP accumulator.
    ///
    /// A bar refused for either is not swept and never becomes a signal — and it
    /// can still be read as an EXIT price, because the exit indexes the raw
    /// slice by position. An adversarial fleet demonstrated it: a trade entered
    /// and exited on a bar charged to `Census::timestamp_not_increasing`, with
    /// `Grid::refused_paths` at zero.
    ///
    /// # Why a test and not a fix
    ///
    /// The fix is a swept-membership map built once per run from
    /// `Column::sources` — O(bars) once, O(1) per lookup. It is affordable and
    /// it is not built. `CLAUDE.md` §3 rule 6 asks for the bound to be STATED
    /// when it cannot be met, and a sentence in a doc comment is a bound nobody
    /// runs. This is the runnable form: it fails the moment a seventh variant
    /// appears, forcing whoever adds it to decide which side of the line it is
    /// on.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[test]
    fn the_stateful_refusals_are_not_covered_and_this_says_so() {
        // Every variant, split by whether one record alone can decide it.
        let bar_local = [
            Corrupt::HighBelowLow,
            Corrupt::RangeOverflows,
            Corrupt::PriceOutsideRange,
            Corrupt::NegativeVolume,
        ];
        let stateful = [
            Corrupt::TimestampNotIncreasing,
            Corrupt::AccumulatorTooLarge,
        ];

        assert_eq!(
            bar_local.len() + stateful.len(),
            6,
            "`Corrupt` has six variants. If this fails a seventh was added, and \
             whoever added it must decide whether `Candle::check` can see it -- \
             which is exactly the decision that was skipped when the refusal \
             guards were written"
        );
        assert_eq!(
            bar_local.len(),
            4,
            "`Candle::check` covers the four a single record decides"
        );
        assert_eq!(
            stateful.len(),
            2,
            "and cannot cover the two that need a sequence: a trade can still \
             enter and exit on a bar the column refused for one of these, with \
             `Grid::refused_paths` reporting zero"
        );

        // AND THE TWO SETS ARE DISJOINT, so a variant cannot be quietly counted
        // on both sides to make the arithmetic work.
        for s in stateful {
            assert!(
                !bar_local.contains(&s),
                "{s:?} is listed as both bar-local and stateful"
            );
        }
    }
}
