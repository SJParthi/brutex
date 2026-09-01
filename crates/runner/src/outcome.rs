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

/// The product's fixed intraday liquidation deadline: **15:10 IST**.
///
/// Bars are left-labelled, so the accepted one-minute bar stamped 15:09 covers
/// `[15:09, 15:10)` and is the only stored OHLCV record that can price this
/// deadline. The 15:10 bar contains post-deadline prices and is never used.
/// Missing, refused, duplicated, or otherwise ambiguous 15:09 data cannot be
/// replaced by 15:08, 15:10, interpolation, or a fabricated tick.
///
/// This deliberately supersedes the former `session_close - 10` policy. A
/// non-regular session without an accepted unique 15:09 bar has no forced-close
/// price; ordinary exact-horizon trades that finish earlier may still exist,
/// but any hold needing liquidation is dropped.
pub const FORCED_EXIT_MINUTE: i64 = 15 * 60 + 10;

/// One bar's place in its own session.
#[derive(Clone, Copy)]
struct BarBound {
    /// IST day, from [`indicators::ist_day`].
    day: i64,
    /// The only opening minute whose one-minute close prices the fixed forced
    /// exit: 15:09 IST.
    last_fill: i64,
    /// The last bar in this session whose interval ends at or before the
    /// fixed deadline, or `None` when no observed accepted row ends before it.
    last_fill_bar: Option<usize>,
    /// Whether this bar's own interval ends at or before the square-off.
    fillable: bool,
    /// Does this day contain exactly one accepted one-minute bar stamped 15:09?
    /// A missing, refused, or duplicated required bar proves no forced fill.
    day_ended: bool,
}

/// Where every bar sits relative to the fixed 15:10 IST liquidation deadline.
///
/// The boundary is fixed by [`FORCED_EXIT_MINUTE`], but its PRICE is not a
/// constant. Only a unique accepted one-minute 15:09 bar can supply the stored
/// OHLCV whose close reaches that instant. The table therefore carries two
/// separate facts per day: entry geometry (`open + step <= 15:10`) and whether
/// the exact forced-close record exists. An earlier observed bar can bound an
/// ordinary exact-horizon trade but has `day_ended == false`, so it can never be
/// promoted into a forced fill.
///
/// Non-regular sessions are intentionally governed by the same clock. If they
/// do not contain 15:09, an overrun is unpriceable and is dropped. This is the
/// stated consequence of the fixed policy, not an inferred exchange rule.
///
/// # Cost
///
/// The median step is derived once. Given it, one forward pass proves each
/// day's exact required record and one reverse pass carries the usable boundary
/// to every earlier bar. Every lookup afterwards is one
/// bounds-checked read. `CLAUDE.md` §3 rule 4.
pub struct SessionBounds {
    /// One entry per bar of the slice this was built from, in the same order.
    bars: Vec<BarBound>,
}

impl SessionBounds {
    /// Derive the session geometry of `bars`.
    #[must_use]
    pub fn of(bars: &[Candle]) -> Self {
        Self::with_step(bars, median_step_micros(bars), None)
    }

    /// Derive the session geometry with the slice's median step already known.
    ///
    /// This is the constructor [`crate::trade::SliceFacts`] uses so its horizon
    /// clock and its session boundaries share one measurement and no candidate
    /// loop can allocate the median sample again.
    #[must_use]
    pub(crate) fn with_step(bars: &[Candle], step_micros: i64, accepted: Option<&[bool]>) -> Self {
        struct ReverseSession {
            day: i64,
            square_off: i64,
            last_fill: i64,
            last_fill_bar: Option<usize>,
            day_ended: bool,
        }

        let step = step_micros.max(0);
        let is_accepted = |index: usize| {
            accepted.is_none_or(|verdict| verdict.get(index).copied().unwrap_or(false))
        };

        // EXACTLY ONE RAW 15:09 RECORD, AND IT MUST BE ACCEPTED. Counting raw
        // rows as well as verdicts makes a duplicate timestamp ambiguous even
        // when the evaluator accepts the first and rejects the second.
        let required_open = FORCED_EXIT_MINUTE.saturating_sub(1);
        let mut required: std::collections::HashMap<i64, (u64, Option<usize>)> =
            std::collections::HashMap::new();
        for (index, bar) in bars.iter().enumerate() {
            if ist_minute_of_day(bar.ts_micros) != required_open {
                continue;
            }
            let day = indicators::ist_day(bar.ts_micros);
            required
                .entry(day)
                .and_modify(|seen| {
                    seen.0 = seen.0.saturating_add(1);
                    if is_accepted(index) {
                        seen.1 = Some(index);
                    }
                })
                .or_insert((1, is_accepted(index).then_some(index)));
        }
        let proved: std::collections::HashSet<i64> = required
            .iter()
            .filter_map(|(&day, &(count, accepted_index))| {
                (step == 60_000_000 && count == 1 && accepted_index.is_some()).then_some(day)
            })
            .collect();
        let mut stamped: Vec<BarBound> = Vec::with_capacity(bars.len());
        let mut current: Option<ReverseSession> = None;

        // ONE REVERSE TRAVERSAL. The first bar visited for a day initializes
        // that day's fixed 15:10 timestamp; the same absolute instant is then
        // carried through every earlier row on the day.
        for (index, bar) in bars.iter().enumerate().rev() {
            let day = indicators::ist_day(bar.ts_micros);
            if !is_accepted(index) {
                stamped.push(BarBound {
                    day,
                    last_fill: 0,
                    last_fill_bar: None,
                    fillable: false,
                    day_ended: false,
                });
                continue;
            }
            let mut session = match current.take() {
                Some(session) if session.day == day => session,
                _ => {
                    let minute = ist_minute_of_day(bar.ts_micros);
                    let square_off = bar
                        .ts_micros
                        .saturating_sub(minute.saturating_mul(60_000_000))
                        .saturating_add(FORCED_EXIT_MINUTE.saturating_mul(60_000_000));
                    ReverseSession {
                        day,
                        square_off,
                        last_fill: required_open,
                        last_fill_bar: None,
                        day_ended: proved.contains(&day),
                    }
                }
            };

            // A non-positive step cannot prove that even this bar's interval
            // completed. Refusing is the only answer that does not invent a
            // timeframe for an empty or one-bar slice.
            let fillable = step > 0 && bar.ts_micros.saturating_add(step) <= session.square_off;
            if fillable && session.last_fill_bar.is_none() {
                session.last_fill_bar = Some(index);
            }
            stamped.push(BarBound {
                day,
                last_fill: session.last_fill,
                last_fill_bar: session.last_fill_bar,
                fillable,
                day_ended: session.day_ended,
            });
            current = Some(session);
        }
        stamped.reverse();
        Self { bars: stamped }
    }

    /// May a fill land on bar `i`?
    ///
    /// `false` for a bar past its own session's forced exit, and for an index
    /// the slice does not hold.
    #[must_use]
    pub fn fillable(&self, i: usize) -> bool {
        self.bars.get(i).is_some_and(|b| b.fillable)
    }

    /// Did the slice prove the exact fixed forced-close record for this day?
    ///
    /// `true` only for exactly one accepted one-minute bar stamped 15:09.
    /// Missing, corrupt, duplicated and coarse-only paths return `false`.
    #[must_use]
    pub fn day_ended(&self, i: usize) -> bool {
        self.bars.get(i).is_some_and(|b| b.day_ended)
    }

    /// Do bars `i` and `j` belong to the same IST day?
    ///
    /// `false` when either index is outside the slice — two bars that do not
    /// both exist share no day.
    #[must_use]
    pub fn same_day(&self, i: usize, j: usize) -> bool {
        match (self.bars.get(i), self.bars.get(j)) {
            (Some(a), Some(b)) => a.day == b.day,
            _ => false,
        }
    }

    /// The last fillable bar of bar `i`'s own session.
    ///
    /// This is precomputed with the boundary, so outcome measurement and trade
    /// execution use the same O(1) lookup rather than rebuilding parallel
    /// reverse tables.
    #[must_use]
    pub fn last_fill_bar(&self, i: usize) -> Option<usize> {
        self.bars.get(i).and_then(|b| b.last_fill_bar)
    }

    /// The last opening minute of bar `i`'s own session at which a fill may
    /// land, or `None` for an index the slice does not hold.
    ///
    /// Exposed for the tests that pin the derivation against the charter's
    /// worked sessions; [`Self::fillable`] is what the engine asks.
    #[must_use]
    pub fn last_fill_minute(&self, i: usize) -> Option<i64> {
        self.bars.get(i).map(|b| b.last_fill)
    }
}

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
    /// could be entered and measured: a bar its own session's forced exit
    /// refuses — [`SessionBounds::fillable`] — or the forced-close bar itself,
    /// or the tail of the slice. Counting slots would count those as
    /// measurements.
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
pub fn forward(bars: &[Candle], column: &Column, horizon: Horizon) -> Forward {
    let h = horizon.as_bars() as usize;
    let facts = crate::trade::SliceFacts::of(bars, column);

    // PASS THREE: the return, from entry to the earlier of the horizon and the
    // forced close.
    let mut ret: Vec<Option<i64>> = Vec::with_capacity(bars.len());
    // Parallel to `ret`, marking WHY an outcome is absent. Pre-sized for the same
    // reason `ret` is: gate 11 rule 3 asks every collection on this path to be
    // sized once rather than grown.
    let mut refused: Vec<bool> = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if !facts.accepts(i) {
            ret.push(None);
            refused.push(true);
            continue;
        }
        // NO ENTRY ON A FORCED-EXIT BAR. At the square-off the position is being
        // CLOSED, so it cannot also be opened, and a bar whose own interval ends
        // after that instant is a forced-exit bar -- `docs/00-charter.md`'s
        // `bar_open + tf > forced_minute`, which [`SessionBounds::fillable`]
        // answers per day rather than against a fixed minute.
        let Some(square_off) = facts.exits().get(i).copied().flatten() else {
            ret.push(None);
            refused.push(false);
            continue;
        };
        let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {
            ret.push(None);
            refused.push(true);
            continue;
        };
        let span = i64::try_from(h).unwrap_or(i64::MAX);
        let deadline = start.saturating_add(facts.step_micros().saturating_mul(span));
        let forced_stamp = bars
            .get(square_off.bar)
            .map_or(i64::MIN, |bar| bar.ts_micros);
        // THE HOLD ENDS AT THE HORIZON OR AT THE FORCED CLOSE, WHICHEVER COMES
        // FIRST -- but "the data ran out" is neither, and conflating the two
        // would invent a measurement.
        //
        // If the horizon fits inside the tradeable window, it is an ordinary
        // outcome. If it does not, the trade was squared off early, and THAT is
        // a real measured outcome -- but only if the session genuinely ENDED,
        // rather than the file ending on it. A day whose bars simply stop at
        // 11:00 because the slice was cut there has no square-off price at all,
        // and measuring to 10:50 would report a forced exit that never happened.
        //
        // A verified square-off wins before the exact-deadline lookup. The
        // deadline bar need not exist when the exchange closed first; requiring
        // it would refuse the very early exit this branch has proved. When the
        // deadline is inside the tradeable window, however, only a bar stamped
        // at that exact instant can price it -- a prior bar is not a fill at a
        // future time. An unproved session end cannot price either case.
        let exit = if deadline > forced_stamp && square_off.real {
            square_off.bar
        } else if deadline <= forced_stamp {
            let Some(want) = facts.at_timestamp(deadline) else {
                ret.push(None);
                refused.push(true);
                continue;
            };
            want
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
        // `path_accepts` reads the exact `Evaluator::step` verdict retained by
        // the column. That includes timestamp ordering and VWAP accumulator
        // state, so no second local predicate can drift or silently cover only
        // four of the six refusal variants.
        //
        // MARKED, NOT MERELY ABSENT. The first version of this fix pushed a bare
        // `None` and its own comment claimed the drop was "visible in
        // `Edge::mismatched`". It was not: `edge`'s `let Some(r) = ... else {
        // continue }` treats every `None` as the TAIL, under a comment saying so,
        // and a corrupt bar can only ever be an exit bar — so it always landed
        // there. Measured: `n` fell 234 to 230 with `mismatched == 0` in both
        // runs and nothing said why. A silent smaller sample is a quieter version
        // of the same §4 failure.
        if !facts.path_accepts(i, exit) {
            ret.push(None);
            refused.push(true);
            continue;
        }
        let Some((later, now)) = bars
            .get(exit)
            .zip(bars.get(i))
            .map(|(later, now)| (later.close, now.close))
        else {
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

/// The median positive same-session gap between consecutive bars, in
/// microseconds.
///
/// Overnight gaps are excluded outright; median and not mean keeps an intraday
/// closure from redefining the timeframe. Zero means the slice contains no
/// positive observed step; callers then refuse to invent one.
pub(crate) fn median_step_micros(bars: &[Candle]) -> i64 {
    median_step_micros_over(bars, None)
}

/// [`median_step_micros`], excluding every record the execution evaluator
/// refused. A duplicate timestamp or overflowing accumulator record may not
/// define another trade's clock.
pub(crate) fn median_step_micros_over(bars: &[Candle], accepted: Option<&[bool]>) -> i64 {
    let mut steps: Vec<i64> = bars
        .iter()
        .enumerate()
        .zip(bars.iter().enumerate().skip(1))
        .filter(|((a_index, a), (b_index, b))| {
            accepted.is_none_or(|verdict| {
                verdict.get(*a_index).copied().unwrap_or(false)
                    && verdict.get(*b_index).copied().unwrap_or(false)
            }) && indicators::ist_day(a.ts_micros) == indicators::ist_day(b.ts_micros)
        })
        .map(|((_, a), (_, b))| b.ts_micros.saturating_sub(a.ts_micros))
        .filter(|&step| step > 0)
        .collect();
    if steps.is_empty() {
        return 0;
    }
    let middle = steps.len() / 2;
    let (_, median, _) = steps.select_nth_unstable(middle);
    *median
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
///
/// **This doc block, and `is_window_end`'s beside it, had come adrift.** Both
/// sat ABOVE `priced` with no blank line between them and its own doc, so
/// rustdoc attached all three to `priced` and the two functions they describe
/// carried a one-line summary or none at all. That is the trap a new `/// doc`
/// inserted between an existing block and its item sets, and it compiles
/// silently. `is_window_end` is gone -- [`SessionBounds::day_ended`] answers its
/// question without the circularity a derived boundary gave it -- and this block
/// is back on the function it was written for.
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
    // NO DISPERSION IS NO EVIDENCE, AND IT IS ASKED OF `m2` BEFORE THE
    // CORRECTION RATHER THAN OF THE STANDARD ERROR AFTER IT.
    //
    // The guard below already intends this — its own comment says "Every
    // observation identical: the mean is exact and its spread is zero" — and it
    // could not deliver it, because it tests `standard_error > 0.0` and
    // `standard_error` is computed from `long_run_sum_squares`, which is
    // UNCENTERED by construction: `m2 + 2(A - m·B + m²·C)`. On a sample whose
    // every observation is identical, `m2` is exactly zero and that bracket is
    // pure rounding noise in the cancellation of three large terms. The SIGN of
    // that noise alone decided between `t = 0` and `t` in the hundreds of
    // millions.
    //
    // REPRODUCED, against the real `edge` and on this crate's own default bars.
    // The first version of this comment said the artefact was reported and not
    // reproduced, and that the guard was therefore kept as a stricter question
    // rather than a demonstrated repair. Both halves were wrong, and the reason
    // the earlier attempt missed it is worth more than the correction: it used
    // `Horizon::bars(1)`, and at a one-bar horizon the drain
    // `source - older >= horizon_bars` empties `recent` BEFORE the pair loop, so
    // no cross term is ever formed. The cross-sums did not cancel -- they were
    // never accumulated. Overlap is the precondition, not irregular spacing.
    //
    // MEASURED, at `Horizon::DEFAULT` over a ramp with every truncated window
    // excluded, n = 1016 identical observations of 15 paisa:
    //
    //   m2 = 0x0 exactly | A = 1.575e6 | B = 2.1e5 | C = 7.000000000000009e3
    //   long_run_sum_squares = 3.725290298461914e-9  (2^-28, four ulp)
    //   t with this check removed = 2.4956924974889034e8
    //
    // The sign is the whole outcome and it is not stable: over `step` in 1..=60
    // at eight sessions every case came out POSITIVE and scored |t| above 1e8;
    // at twelve sessions every case came out NEGATIVE, `sqrt` gave `NaN` and
    // `is_finite` sent it to zero. 60 huge and 60 zero, maximum 2.6116e8.
    //
    // And it needs no contrived fixture. On plain `synthetic::sessions(12)`,
    // 170 of the 466 measuring two-bit masks have zero spread and 71 of those
    // report |t| > 1e5 with this check removed, the largest 4.0710e8. Zero
    // masks with genuine spread are affected either way.
    // `a_constant_sample_whose_windows_overlap_is_still_no_evidence` and
    // `no_zero_spread_mask_reports_a_finding_on_the_ordinary_fixture` are those
    // two measurements, and both FAIL when this check is removed.
    //
    // AND `rank::walk` ORDERS ON `edge.t.abs()`, so those artefacts sorted
    // ABOVE every genuine finding and occupied the head of `Ranked::top`, where
    // `keep` cut the real results out beneath them. `significance::p_value` is
    // `2(1 - Φ(|t|))`, which clamps to exactly zero at that magnitude, so they
    // cleared any Bonferroni bar the run could set. `Scored::cmp` deliberately
    // demotes NON-FINITE scores — the ordering defends against infinity and was
    // defeated by a large finite artefact.
    //
    // `m2` IS THE EXACT TEST AND NEEDS NO EPSILON. Welford increments it by
    // `delta * delta2`, and on identical observations both factors are exactly
    // zero, so `m2` is exactly zero — not nearly. A sample with genuine but
    // tiny dispersion has a genuine tiny `m2` and is unaffected. That is why
    // the question is asked here, of the centered sum, and not of a quantity
    // three cancellations later.
    //
    // `edge`'s own doc says Welford was chosen to avoid "catastrophic
    // cancellation ... when the mean is large relative to the spread". Welford
    // protects `m2`; the three Newey-West cross-sums reintroduced exactly that
    // defect into the correction term. This keeps the protection Welford was
    // for.
    if m2 <= 0.0 {
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
        Edge, FORCED_EXIT_MINUTE, Horizon, SessionBounds, Sides, edge, forward,
        long_run_sum_squares, median_step_micros, milli,
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

    /// Build the evaluator-produced acceptance map beside a test forward.
    fn forward_of(bars: &[Candle], horizon: Horizon) -> super::Forward {
        let column = Column::build(bars, &mut evaluator());
        forward(bars, &column, horizon)
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
        // These bars are far before the fixed 15:10 IST deadline, so every
        // complete two-bar horizon is priceable even though this deliberately
        // tiny arithmetic fixture cannot prove a forced exit.
        let bars: Vec<Candle> = [100, 110, 130, 125, 140]
            .iter()
            .enumerate()
            .map(|(i, &c)| candle(i64::try_from(i).unwrap_or(0), c))
            .collect();
        let f = forward_of(&bars, h(2));

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
        assert_eq!(forward_of(&bars, h(3)).measured(), 0);
        assert_eq!(forward_of(&bars, h(99)).measured(), 0);
        assert_eq!(forward_of(&[], h(1)).measured(), 0);
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

    /// A SAMPLE WITH NO DISPERSION IS NO EVIDENCE.
    ///
    /// # What this proves, and what it does NOT
    ///
    /// It proves the property: every observation identical gives `t = 0`, and
    /// the mean is still carried because the sample has no spread, not no
    /// direction.
    ///
    /// **It does not discriminate against the previous implementation, and that
    /// is stated rather than left to be assumed.** This test passes with the
    /// `m2 <= 0.0` check disabled, and the reason is `h(1)`: at a one-bar
    /// horizon the drain `source - older >= horizon_bars` empties `recent`
    /// before the pair loop runs, so `cross_a`, `cross_b` and `cross_c` all stay
    /// exactly zero and `long_run_sum_squares` returns `m2` untouched. There is
    /// no cancellation here to be inexact — there is no cross term at all.
    ///
    /// The earlier note in this place said the artefact could not be reproduced
    /// and blamed the fixture's EVEN spacing. That was the wrong diagnosis. The
    /// precondition is OVERLAP, not irregularity, and
    /// `a_constant_sample_whose_windows_overlap_is_still_no_evidence` reaches it
    /// by running the same shape of ramp at `Horizon::DEFAULT`: it fails without
    /// the guard with `t = 2.4956924974889034e8` on 1,016 identical
    /// observations. `no_zero_spread_mask_reports_a_finding_on_the_ordinary_fixture`
    /// shows 71 of `synthetic::sessions(12)`'s own two-bit masks doing the same.
    ///
    /// # Why the fixture is shaped the way it is
    ///
    /// `Evaluator::warmed_up` needs five completed prior sessions, so a
    /// two-session slice yields an EMPTY column and every assertion below would
    /// pass against nothing. `rank::walk` orders on `edge.t.abs()`, which is
    /// why a large `t` here would sort above every genuine finding, and
    /// `p_value` is `2(1 - Φ(|t|))`, which clamps to exactly zero at that
    /// magnitude and clears any Bonferroni bar the run can set.
    #[test]
    fn a_sample_with_no_dispersion_is_no_evidence_and_not_certainty() {
        // REAL SESSION STAMPS WITH RAMPED PRICES. `forward` refuses a window
        // that runs past a session close, so a fixture stamped from the epoch
        // decides nothing. Taking `synthetic::sessions`' clock and replacing
        // only the prices keeps every stamp legal while making every
        // one-bar forward move exactly one paisa.
        // EIGHT sessions, not two: `Evaluator::warmed_up` needs five completed
        // prior sessions before `Column::build` emits a single row, so a
        // two-session fixture produces an EMPTY column and every assertion
        // below would pass against nothing.
        let bars: Vec<Candle> = crate::synthetic::sessions(8)
            .into_iter()
            .enumerate()
            .map(|(i, b)| {
                let close = 2_500_000 + i64::try_from(i).unwrap_or(0);
                Candle::new(
                    b.ts_micros,
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
        // A ONE-BAR HORIZON, so every decided window is EXACTLY one step of the
        // ramp. At fifteen bars a window near the forced close is truncated to
        // a shorter move, which is genuine dispersion and a genuinely large
        // `t` -- not the artefact. At one bar a window is either decided in
        // full or refused, so every observation is exactly one paisa and the
        // centred sum of squares is exactly zero.
        let f = forward_of(&bars, h(1));

        // THE EMPTY MASK, which fires on every bar: `(bits & mask) == mask` is
        // `0 == 0`. A monotone ramp makes every NAMED condition constant, so no
        // single-bit mask fires at all on it — and the sample this test is
        // about is precisely "every observation identical", which the empty
        // mask over a ramp produces exactly.
        let e = edge(&column, &f, &ConditionMask::default());
        assert!(
            e.n >= 2,
            "the fixture must produce a measurable sample, or it proves nothing -- n={}, mismatched={}, refused={}, column rows={}",
            e.n,
            e.mismatched,
            e.refused,
            column.bits().len()
        );
        // EXACT ZERO IS THE CLAIM, so the comparison is exact. `assemble(0.0)`
        // writes the literal, so there is no arithmetic between it and this
        // assertion for an epsilon to absorb -- and an epsilon here would pass
        // for the very artefact this is about if it ever shrank.
        assert!(
            e.t == 0.0,
            "{} identical forward moves scored a t of {} -- a degenerate sample \
             reported as certainty, which `rank::walk` then sorts above every \
             real finding and `p_value` clamps to zero",
            e.n,
            e.t
        );
        assert!(
            e.mean_paisa > 0.0,
            "and the mean is still carried: the sample has no spread, not no \
             direction"
        );
    }
    #[test]
    fn the_mean_and_t_are_measured_over_the_bars_the_mask_fired_on() {
        // A real column, and the mask of a position that actually varies.
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let f = forward_of(&bars, Horizon::DEFAULT);

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
        // from 09:15, and a position is squared off at the fixed 15:10 IST
        // deadline, long or short. So no measured return may span two trading
        // days, and none may run past that deadline.
        //
        // `synthetic::bar` stamps minute 0 at the IST open, so bar `m` is IST
        // minute 555 + m. With left-edge stamps, the bar at 15:09 represents
        // [15:09, 15:10), making offset 354 the last admissible fill. The
        // constants are derived here rather than written down, so a fixture
        // change cannot quietly make this test vacuous.
        let bars = crate::synthetic::sessions(3);
        let f = forward_of(&bars, h(15));

        let minute_of = |i: usize| -> i64 {
            let ts = bars.get(i).map_or(0, |b| b.ts_micros);
            ts.saturating_add(indicators::IST_OFFSET_MICROS)
                .div_euclid(60_000_000)
                .rem_euclid(1_440)
        };
        // The exact stored one-minute interval that closes at the policy
        // deadline. A 15:10-stamped bar contains post-deadline prices.
        let last_fill_minute = FORCED_EXIT_MINUTE - 1;
        let close_bar = (0..375)
            .find(|&i| minute_of(i) == last_fill_minute)
            .expect("the fixture must contain a 15:09 bar");
        assert_eq!(
            close_bar, 354,
            "09:15 + 354 minutes is the 15:09 bar whose interval closes at the \
             fixed 15:10 deadline"
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

        // TWO: the last legal entry is 15:08, and it exits at 15:09 -- one bar,
        // not the full fifteen.
        let last_entry = close_bar.saturating_sub(1);
        let expected = bars.get(close_bar).map_or(0, |b| b.close)
            - bars.get(last_entry).map_or(0, |b| b.close);
        assert_eq!(
            f.at(last_entry),
            Some(expected),
            "an entry at 15:08 is squared off at 15:10, so its return is one \
             bar of movement and not fifteen"
        );

        // THREE: an entry whose horizon would run past the square-off exits AT
        // it. Before the rule existed this measured close[365] - close[350], which
        // is a price from after the position was already closed.
        let early = close_bar.saturating_sub(5);
        let forced =
            bars.get(close_bar).map_or(0, |b| b.close) - bars.get(early).map_or(0, |b| b.close);
        let unforced = bars.get(early.saturating_add(15)).map_or(0, |b| b.close)
            - bars.get(early).map_or(0, |b| b.close);
        assert_eq!(
            f.at(early),
            Some(forced),
            "the exit is the proved 15:10 square-off"
        );
        assert_ne!(
            forced, unforced,
            "the fixture must make the two answers differ, or this proves \
             nothing"
        );

        // FOUR, AND IT IS THE RULE ITSELF: no measured outcome anywhere in the
        // slice crosses a day boundary. Checked over every bar rather than a
        // sample, because "usually intraday" is not the promise.
        //
        // THE EXIT IS RECONSTRUCTED, NOT GUESSED AT. This read
        // `ist_day(bars[i + 15])` and called that the exit day, which was true
        // only while no enterable bar was within fifteen bars of the session's
        // end. A proxy that fails on correct output is not a check of the rule,
        // so this recomputes `min(i + H, that day's exact 15:09 bar)` and
        // asserts against the actual move.
        let day_of = |i: usize| indicators::ist_day(bars.get(i).map_or(0, |b| b.ts_micros));
        let forced_bar_of = |day: i64| -> usize {
            (0..bars.len())
                .find(|&j| day_of(j) == day && minute_of(j) == FORCED_EXIT_MINUTE.saturating_sub(1))
                .unwrap_or(0)
        };
        let mut checked = 0_usize;
        for i in 0..bars.len() {
            let Some(moved) = f.at(i) else {
                continue;
            };
            let day = day_of(i);
            let exit = i.saturating_add(15).min(forced_bar_of(day));
            assert_eq!(
                day_of(exit),
                day,
                "bar {i} measured an outcome whose exit is on another day"
            );
            assert_eq!(
                moved,
                bars.get(exit).map_or(0, |b| b.close) - bars.get(i).map_or(0, |b| b.close),
                "bar {i} must be measured to `min(i + H, its own session's \
                 forced bar)` and to nothing else"
            );
            checked = checked.saturating_add(1);
        }
        assert!(
            checked > 1_000,
            "the loop must have measured most of the fixture, or it proves \
             nothing: {checked} of {} bars",
            bars.len()
        );
    }

    /// The UTC microsecond stamp of IST minute `minute` on IST day `day`.
    fn ist_stamp(day: i64, minute: i64) -> i64 {
        (day * 1_440 + minute) * 60_000_000 - indicators::IST_OFFSET_MICROS
    }

    /// One bar per minute in `minutes`, on IST day `day`, at a flat price.
    fn day_of_bars(day: i64, minutes: impl IntoIterator<Item = i64>) -> Vec<Candle> {
        minutes
            .into_iter()
            .map(|m| {
                Candle::new(
                    ist_stamp(day, m),
                    2_500_000,
                    2_500_010,
                    2_499_990,
                    2_500_000,
                    1,
                    OI_NULL,
                )
            })
            .collect()
    }

    /// THE PRODUCT CLOCK IS FIXED EVEN WHEN THE OBSERVED SESSION SHAPE DIFFERS.
    ///
    /// Regular and extended sessions both contain 15:09 and therefore expose
    /// the exact left-labelled minute whose close reaches 15:10. The Muhurat
    /// fixture ends before it and must refuse an overrun. This is a locked
    /// product policy, not a claim that every NSE session closes at one time.
    #[test]
    fn every_session_uses_fixed_1510_and_only_exact_1509_prices_it() {
        let mut bars = day_of_bars(0, 9 * 60 + 15..=15 * 60 + 29);
        bars.extend(day_of_bars(1, 13 * 60 + 45..=14 * 60 + 44));
        bars.extend(day_of_bars(2, 9 * 60 + 15..=16 * 60 + 59));
        // A fourth day demonstrates that "tomorrow exists" still does not
        // certify either non-regular shape.
        bars.extend(day_of_bars(3, 9 * 60 + 15..=9 * 60 + 15));

        let session = SessionBounds::of(&bars);
        assert_eq!(FORCED_EXIT_MINUTE, 15 * 60 + 10);

        // Both shapes that contain the exact accepted minute prove the same
        // forced fill. The short shape does not, even though a later day follows.
        let expected: [(usize, &str, i64); 2] = [
            (
                0,
                "a regular session containing the fixed deadline",
                FORCED_EXIT_MINUTE,
            ),
            (
                475,
                "an extended session still using the fixed deadline",
                FORCED_EXIT_MINUTE,
            ),
        ];
        for (index, what, forced_minute) in expected {
            assert_eq!(
                session.last_fill_minute(index),
                Some(forced_minute - 1),
                "{what}: a one-minute bar is forced iff `bar_open + 1 > \
                 {forced_minute}`, so the last fill is stamped {}",
                forced_minute - 1
            );
        }
        assert!(session.day_ended(0));
        assert!(!session.day_ended(375));
        assert!(session.day_ended(475));

        // AND THE RULE REACHES `forward`, not only the table. Every measured
        // outcome for the certified regular session ends at or before its own
        // forced exit, and the last measured entry is stamped a minute before
        // its last possible exit.
        let f = forward_of(&bars, h(15));
        let minute_of = |i: usize| -> i64 {
            (bars.get(i).map_or(0, |b| b.ts_micros) + indicators::IST_OFFSET_MICROS)
                .div_euclid(60_000_000)
                .rem_euclid(1_440)
        };
        let day_of = |i: usize| indicators::ist_day(bars.get(i).map_or(0, |b| b.ts_micros));
        for (index, what, forced_minute) in expected {
            let day = day_of(index);
            let last_measured = (0..bars.len())
                .filter(|&i| day_of(i) == day && f.at(i).is_some())
                .max()
                .expect("a session containing 15:09 must measure something");
            assert_eq!(
                minute_of(last_measured),
                forced_minute - 2,
                "{what}: the last bar that can be ENTERED is one before the last \
                 bar that can be EXITED on, because a hold of zero bars is not a \
                 hold. The last fill is {}, so the last entry is {}.",
                forced_minute - 1,
                forced_minute - 2
            );
        }
        let overruns = forward_of(&bars, h(400));
        let short_end = 375_usize.saturating_add(60);
        assert!(
            (375..short_end).all(|index| overruns.at(index).is_none()),
            "an unknown short-session shape cannot manufacture a forced close"
        );
    }

    /// A forced fill is evidence from one exact raw record, not a nearest-row
    /// lookup. Missing, evaluator-refused, and duplicated 15:09 rows all refuse;
    /// neither the valid 15:08 nor the valid 15:10 neighbour substitutes.
    #[test]
    fn missing_corrupt_or_ambiguous_1509_never_fabricates_a_forced_exit() {
        let complete = day_of_bars(0, 9 * 60 + 15..=15 * 60 + 29);
        let required = usize::try_from(15 * 60 + 9 - (9 * 60 + 15))
            .expect("15:09 lies inside the regular fixture");

        let mut missing = complete.clone();
        let removed = missing.remove(required);
        let missing_bounds = SessionBounds::with_step(&missing, 60_000_000, None);
        assert!(!missing_bounds.day_ended(0));
        assert_eq!(
            missing_bounds.last_fill_bar(0),
            Some(required.saturating_sub(1)),
            "15:08 may bound an ordinary observed horizon but is not proof of liquidation"
        );
        assert!(
            forward_of(&missing, h(400)).at(0).is_none(),
            "a later 15:10 row must not become the missing forced price"
        );

        let mut accepted = vec![true; complete.len()];
        *accepted
            .get_mut(required)
            .expect("the required row is inside the acceptance fixture") = false;
        let corrupt_bounds = SessionBounds::with_step(&complete, 60_000_000, Some(&accepted));
        assert!(!corrupt_bounds.day_ended(0));
        assert_eq!(
            corrupt_bounds.last_fill_bar(0),
            Some(required.saturating_sub(1)),
            "an evaluator-refused 15:09 row is absence, not a usable print"
        );

        let mut duplicated = complete;
        duplicated.insert(required, removed);
        let duplicate_acceptance = vec![true; duplicated.len()];
        let duplicate_bounds =
            SessionBounds::with_step(&duplicated, 60_000_000, Some(&duplicate_acceptance));
        assert!(!duplicate_bounds.day_ended(0));
        assert_eq!(
            duplicate_bounds.last_fill_minute(0),
            Some(FORCED_EXIT_MINUTE - 1),
            "the policy timestamp remains named, but two raw rows make its price ambiguous"
        );
    }

    /// A boundary needs an observed timeframe as well as an observed last bar.
    /// Two bars with the same stamp provide no positive step, so choosing one
    /// minute here would be an invented fact and manufacture a square-off.
    #[test]
    fn a_slice_without_an_observed_timeframe_invents_no_session_boundary() {
        let bars = [candle(600, 10_000), candle(600, 10_010)];
        assert_eq!(median_step_micros(&bars), 0);

        let session = SessionBounds::of(&bars);
        assert!(!session.fillable(0));
        assert!(session.last_fill_bar(0).is_none());
        assert!(forward_of(&bars, h(1)).at(0).is_none());
    }

    /// A SLICE THAT STOPS MID-SESSION MEASURES NO FORCED OUTCOME.
    ///
    /// The `crate::trade` twin of this is
    /// `a_slice_that_stops_mid_session_fabricates_no_square_off`, and the two
    /// modules have to agree: `outcome` measures the return and `trade` fills
    /// it, so one of them refusing while the other traded is the disagreement
    /// RULE 1c was written about.
    ///
    /// The required record is the accepted row stamped 15:09. A slice cut at
    /// 14:14 does not have it, and a later day cannot fill the missing minute.
    #[test]
    fn a_session_the_slice_never_saw_end_measures_no_square_off() {
        // Day 0 whole, day 1 cut at 14:14. H is longer than either session, so
        // NOTHING can reach its horizon and every outcome that exists at all is
        // a square-off.
        let mut bars = day_of_bars(0, 9 * 60 + 15..=15 * 60 + 29);
        let cut = day_of_bars(1, 9 * 60 + 15..=14 * 60 + 14);
        let first_of_cut = bars.len();
        bars.extend(cut);
        let f = forward_of(&bars, h(400));

        assert!(
            (0..first_of_cut).any(|i| f.at(i).is_some()),
            "the session that genuinely ended must still be measured, or this \
             test would pass over an empty answer"
        );
        for i in first_of_cut..bars.len() {
            assert!(
                f.at(i).is_none(),
                "bar {i} is in the session the file cut short: no bar in the \
                 slice proves where it closed, so no forced exit may be measured \
                 there"
            );
            assert!(
                !f.was_refused(i),
                "and the absence is the missing session end, not a corrupt bar"
            );
        }

        // A HOLD THAT FITS INSIDE THE CUT IS STILL MEASURED, because only the
        // OVERRUN has nowhere to go. Refusing the whole truncated session would
        // be a second wrong answer -- the same distinction
        // `a_horizon_that_fits_inside_the_truncated_session_is_still_traded`
        // pins on the trade side.
        let short = forward_of(&bars, h(5));
        assert!(
            (first_of_cut..bars.len()).any(|i| short.at(i).is_some()),
            "a five-bar hold inside a session the slice cut short completed, \
             and both its prices printed"
        );
    }

    /// A COARSE-ONLY SLICE CANNOT SUPPLY THE FIXED MINUTE'S OHLCV.
    ///
    /// # `forward` runs on the SIGNAL series, which is not always one minute
    ///
    /// `crate::resample` buckets on the IST clock, so the sixty-minute rung of a
    /// regular session is seven bars stamped 09:00 … 15:00. None is the exact
    /// 15:09 one-minute record, so no coarse bar may be promoted into a forced
    /// fill. Exact coarse horizons observed before 15:10 remain usable only for
    /// this legacy same-series runner surface; stored operator paths reproject
    /// onto explicit one-minute OHLCV before reaching money.
    #[test]
    fn a_coarse_rung_prices_exact_horizons_but_not_an_unproved_square_off() {
        let minutes: Vec<i64> = (0..7).map(|k| 9 * 60 + k * 60).collect();
        let mut bars = day_of_bars(0, minutes.clone());
        bars.extend(day_of_bars(1, minutes));
        let session = SessionBounds::of(&bars);

        assert_eq!(
            session.last_fill_minute(0),
            Some(FORCED_EXIT_MINUTE - 1),
            "the product deadline stays fixed at 15:10 even though a coarse-only \
             compatibility slice cannot supply its required 15:09 price"
        );
        assert!(
            session.fillable(5) && !session.fillable(6),
            "the 14:00 bar may be entered and the 15:00 bar may not: `open + tf` \
             is 15:00 for one and 16:00 for the other, against a forced exit the \
             same `tf` cancels out of"
        );

        // These seven coarse buckets do not prove the required one-minute row.
        let f = forward_of(&bars, h(400));
        assert!(
            f.at(0).is_none(),
            "a later day does not prove the coarse prior session reached a known close"
        );
        assert!(
            forward_of(&bars, h(1)).at(0).is_some(),
            "an exact timestamp one rung ahead remains an observed outcome"
        );
        assert!(
            f.at(6).is_none(),
            "and the 15:00 bar carries no outcome at all: it is the forced-exit \
             bar, so nothing may be opened on it"
        );
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
        let from_long = forward_of(&long, h(1));
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
        let from_short = forward_of(&short, h(1));
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
        let f = forward_of(&bars, h(1));
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

        let wrong = forward_of(&coarse, Horizon::DEFAULT);
        let e = edge(&column, &wrong, &ConditionMask::default());
        assert!(
            e.mismatched > 0,
            "pairing a column with a Forward from a shorter slice must be \
             COUNTED, not silently folded into the tail"
        );

        // And the correct pairing reports zero, so a non-zero value means what
        // it says rather than being background noise.
        let right = forward_of(&minute, Horizon::DEFAULT);
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
        let f = forward_of(&bars, Horizon::DEFAULT);
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

    /// A ramp whose every FULL-horizon window moves by exactly `step * H`.
    ///
    /// The bar stamped 15:09 of each session — offset 354 of 375, the last bar
    /// [`SessionBounds::fillable`] admits and therefore every session's forced
    /// close — is built with `high` below `low`, so `Candle::check` refuses it and
    /// [`priced`] will not let it settle an exit. Every entry whose window would
    /// otherwise have been TRUNCATED at the forced close is dropped instead of
    /// measured short, and truncation is the one thing that puts genuine
    /// dispersion into a ramp: a window cut from fifteen bars to four is a
    /// smaller move, not the same one.
    ///
    /// What is left is the sample this file's guard is about — every observation
    /// identical, arrived at from a real column over real session stamps rather
    /// than asserted.
    fn ramp_with_no_truncated_window(sessions: i64, step: i64) -> Vec<Candle> {
        crate::synthetic::sessions(sessions)
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let close = 2_500_000 + i64::try_from(i).unwrap_or(0).saturating_mul(step);
                if i % 375 == 354 {
                    Candle::new(
                        c.ts_micros,
                        close,
                        close - 10,
                        close + 10,
                        close,
                        100,
                        OI_NULL,
                    )
                } else {
                    Candle::new(
                        c.ts_micros,
                        close,
                        close + 10,
                        close - 10,
                        close,
                        100,
                        OI_NULL,
                    )
                }
            })
            .collect()
    }

    /// The distinct forward moves `mask` was measured over.
    fn distinct_moves(column: &Column, f: &super::Forward, mask: &ConditionMask) -> Vec<i64> {
        let mut seen: Vec<i64> = column
            .bits()
            .iter()
            .zip(column.sources())
            .filter(|(bits, _)| bits.hits(mask))
            .filter_map(|(_, &s)| f.at(s))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen
    }

    /// THE ARTEFACT, REPRODUCED. This is the test the `m2 <= 0.0` guard exists
    /// for, and unlike its neighbour it FAILS when that guard is removed.
    ///
    /// # Why the neighbouring test could not reach it
    ///
    /// `a_sample_with_no_dispersion_is_no_evidence_and_not_certainty` uses
    /// `h(1)`, and at a one-bar horizon the drain
    /// `source - older >= horizon_bars` empties `recent` before the pair loop
    /// runs. **No pair is ever formed**, so `cross_a`, `cross_b` and `cross_c`
    /// all stay exactly zero, `long_run_sum_squares` returns `m2` untouched, and
    /// the old `standard_error > 0.0` guard caught the case on its own. That
    /// fixture proves the property and discriminates against nothing.
    ///
    /// The correction only has arithmetic to get wrong when hits fall CLOSER
    /// together than the horizon. This fixture runs at `Horizon::DEFAULT`, where
    /// consecutive hits are one bar apart and every observation pairs with the
    /// fourteen before it at weights `14/15 .. 1/15`.
    ///
    /// # The algebra, and why the residue is not zero
    ///
    /// With every observation equal to `x`, Welford makes `mean` exactly `x` and
    /// `m2` exactly `+0.0` — measured, `m2.to_bits() == 0x0`. The three
    /// uncentered sums are then `A = Σ w x²`, `B = Σ 2 w x`, `C = Σ w`, and
    /// `A − x·B + x²·C` is algebraically zero. It is not zero in `f64`, because
    /// each is a DIFFERENT sequential summation at a different magnitude:
    /// `A` accumulates near `x²·ΣW`, `C` near `ΣW`, and their rounding errors do
    /// not correspond. What survives is a few ulp of `2 x² ΣW`.
    ///
    /// MEASURED on this fixture at `step = 1`, `n = 1016`:
    ///
    /// | | |
    /// |---|---|
    /// | `m2` | `0x0` — exactly zero |
    /// | `A` | 1.575e6 |
    /// | `B` | 2.1e5 |
    /// | `C` | 7.000000000000009e3 |
    /// | `long_run_sum_squares` | **3.725290298461914e-9** = 2⁻²⁸, four ulp |
    /// | `t` with the guard removed | **2.4956924974889034e8** |
    ///
    /// The SIGN of that residue is the whole outcome and it is not stable: over
    /// `step` in `1..=60` at eight sessions every case came out POSITIVE and
    /// scored `|t|` above 1e8; at twelve sessions (`n = 2372`) every case came
    /// out NEGATIVE, `sqrt` returned `NaN`, and `is_finite` sent them to zero.
    /// 60 huge and 60 zero out of 120, maximum `|t|` 2.611610832622043e8. There
    /// is no middle: a degenerate sample scores either nothing or a `t` that
    /// `rank::walk` sorts above every real finding and that `p_value` clamps to
    /// exactly zero.
    #[test]
    fn a_constant_sample_whose_windows_overlap_is_still_no_evidence() {
        let bars = ramp_with_no_truncated_window(8, 1);
        let column = Column::build(&bars, &mut evaluator());
        let f = forward_of(&bars, Horizon::DEFAULT);
        let all = ConditionMask::default();
        let e = edge(&column, &f, &all);

        // THE FIXTURE MUST BE THE ONE THIS IS ABOUT, and each clause is a way it
        // has already failed to be.
        assert!(e.n >= 2, "an empty column proves nothing -- n={}", e.n);
        assert_eq!(
            distinct_moves(&column, &f, &all).len(),
            1,
            "every measured window must move by the SAME amount, or the sample \
             has genuine dispersion and a large t is arithmetic rather than an \
             artefact"
        );
        // AND THE WINDOWS MUST ACTUALLY OVERLAP, which is what the `h(1)`
        // fixture next door cannot do: with no pair closer than the horizon
        // every cross-sum stays zero and the correction has no arithmetic to
        // get wrong.
        let horizon = usize::try_from(Horizon::DEFAULT.as_bars()).unwrap_or(usize::MAX);
        let measured: Vec<usize> = column
            .bits()
            .iter()
            .zip(column.sources())
            .filter(|(bits, _)| bits.hits(&all))
            .filter_map(|(_, &s)| f.at(s).map(|_| s))
            .collect();
        let overlapping = measured
            .iter()
            .zip(measured.iter().skip(1))
            .filter(|(a, b)| b.saturating_sub(**a) < horizon)
            .count();
        assert!(
            overlapping > 0,
            "no two hits are closer than the horizon, so no cross term is \
             formed and this fixture cannot reach the defect"
        );

        assert!(
            e.t == 0.0,
            "{} identical forward moves over OVERLAPPING windows scored a t of \
             {} -- the rounding residue of an exact cancellation, reported as \
             the strongest finding in the run",
            e.n,
            e.t
        );
        assert!(
            e.mean_paisa > 0.0,
            "and the mean is still carried: no spread is not no direction"
        );
    }

    /// The same artefact WITHOUT a hand-shaped price series, on the fixture the
    /// rest of this crate already sweeps.
    ///
    /// `synthetic::sessions` carries a period-seven wobble on a linear drift, so
    /// a sparse mask routinely fires on bars whose fifteen-bar forward move is
    /// identical — nothing has to be constructed for it. Over the 466 two-bit
    /// masks that measure at least two bars on twelve sessions, **170 have zero
    /// spread**, and with the `m2 <= 0.0` guard removed **71 of them report
    /// `|t| > 1e5`**, the largest 4.0710201489847726e8. With the guard in place
    /// none does, and no mask with genuine spread is suppressed — measured:
    /// zero of the 71 had more than one distinct move.
    ///
    /// So the defect is not a property of a contrived fixture. It is reachable
    /// from the bars this crate generates by default, on ordinary pairs.
    #[test]
    fn no_zero_spread_mask_reports_a_finding_on_the_ordinary_fixture() {
        let bars = crate::synthetic::sessions(12);
        let column = Column::build(&bars, &mut evaluator());
        let f = forward_of(&bars, Horizon::DEFAULT);
        let mut zero_spread = 0_u32;
        for a in 0..64_u32 {
            let one = ConditionMask::default().with_bit(a);
            for b in (a + 1)..64_u32 {
                let mask = one.with_bit(b);
                let e = edge(&column, &f, &mask);
                if e.n < 2 || distinct_moves(&column, &f, &mask).len() != 1 {
                    continue;
                }
                zero_spread = zero_spread.saturating_add(1);
                assert!(
                    e.t == 0.0,
                    "bits {a},{b}: {} identical forward moves scored t={}",
                    e.n,
                    e.t
                );
            }
        }
        assert!(
            zero_spread > 100,
            "the scan must actually find zero-spread masks or it asserts \
             nothing -- found {zero_spread}"
        );
    }

    /// AND THE GUARD IS NOT A CLIFF THE REAL CASES FALL OFF.
    ///
    /// `m2 <= 0.0` is exact, with no epsilon, so the question is whether a
    /// sample with GENUINE but minimal dispersion still measures. Forward moves
    /// are `i64` paisa, so the smallest non-zero `m2` any sample can carry is
    /// one observation differing by one paisa from the rest — `1 − 1/n`, at
    /// least 0.5 for `n >= 2`. That is thirteen orders of magnitude above the
    /// rounding residue this file's guard is about (3.7e-9 at `n = 1016`), which
    /// is why the exact test separates them rather than merely happening to.
    ///
    /// Measured here: one paisa of dispersion in 1,016 observations produces a
    /// finite, non-zero `t` — the guard does not fire.
    #[test]
    fn one_paisa_of_genuine_dispersion_still_measures() {
        let mut bars = ramp_with_no_truncated_window(8, 1);
        // One bar lifted by a single paisa, which moves the two windows that
        // end on it and nothing else.
        if let Some(b) = bars.get_mut(2_000) {
            *b = Candle::new(
                b.ts_micros,
                b.close + 1,
                b.close + 11,
                b.close - 9,
                b.close + 1,
                100,
                OI_NULL,
            );
        }
        let column = Column::build(&bars, &mut evaluator());
        let f = forward_of(&bars, Horizon::DEFAULT);
        let all = ConditionMask::default();
        let e = edge(&column, &f, &all);
        assert!(
            distinct_moves(&column, &f, &all).len() > 1,
            "the fixture must carry genuine dispersion, or it proves nothing"
        );
        assert!(
            e.t != 0.0 && e.t.is_finite(),
            "one paisa of real spread must still be measured, not swallowed by \
             the guard -- t={}",
            e.t
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod refused_bar_tests {
    use super::{Horizon, forward};
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use indicators::{Candle, OI_NULL};

    fn forward_of(bars: &[Candle], horizon: Horizon) -> super::Forward {
        let mut evaluator = Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        );
        let column = Column::build(bars, &mut evaluator);
        forward(bars, &column, horizon)
    }

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

        // Every complete two-minute horizon here ends long before 15:10, so the
        // tiny fixture needs no synthetic session tail.
        let bars = vec![
            ok(0, 2_600_000),
            ok(1, 2_600_100),
            poisoned,
            ok(3, 2_600_200),
        ];
        let f = forward_of(&bars, Horizon::bars(2).expect("a non-zero horizon"));

        assert_eq!(
            f.at(0),
            None,
            "bar 0's outcome is priced against bar 2, which the engine refused. \
             It used to read the refused close as 9,999,999 and report a move of \
             7,399,999 paisa"
        );
        assert_eq!(
            f.at(1),
            None,
            "bar 1's path crosses refused bar 2, so the interior record cannot \
             move extrema or fire orders and the entire path is refused"
        );

        // AND THE SAME SLICE WITH A SOUND BAR IN THAT SLOT STILL ANSWERS. A fix
        // that refused everything would satisfy the assertion above and be
        // useless, which is the mutant this second half kills.
        let mut healthy = bars.clone();
        if let Some(slot) = healthy.get_mut(2) {
            *slot = ok(2, 2_600_150);
        }
        let g = forward_of(&healthy, Horizon::bars(2).expect("a non-zero horizon"));
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

        // Six bars, then the ten that hold the square-off window off them --
        // see the neighbouring test for why a hand-made session needs them.
        let mut clean: Vec<Candle> = (0..6).map(|m| ok(m, 2_600_000 + m * 100)).collect();
        clean.extend((6..16).map(|m| ok(m, 2_600_500)));
        let mut dirty = clean.clone();
        if let Some(slot) = dirty.get_mut(3) {
            *slot = bad(3);
        }

        let h = Horizon::bars(1).expect("a non-zero horizon");
        let measured = |bs: &[Candle]| {
            let f = forward_of(bs, h);
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
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod refusal_coverage {
    use super::{Horizon, forward};
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use indicators::{Candle, OI_NULL};

    fn evaluator(availability: Availability) -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            availability,
            Thresholds::CLASSICAL,
        )
    }

    fn assert_forward_refuses_the_path(
        clean: &[Candle],
        dirty: &[Candle],
        availability: Availability,
        victim: usize,
    ) {
        let horizon = Horizon::bars(2).expect("a non-zero horizon");
        let clean_column = Column::build(clean, &mut evaluator(availability));
        let dirty_column = Column::build(dirty, &mut evaluator(availability));
        let clean_forward = forward(clean, &clean_column, horizon);
        let dirty_forward = forward(dirty, &dirty_column, horizon);

        assert!(clean_forward.at(victim.saturating_sub(2)).is_some());
        assert!(!dirty_column.accepts(victim));
        assert!(
            dirty_forward.at(victim).is_none(),
            "a refused entry has no outcome"
        );
        assert!(
            dirty_forward.at(victim.saturating_sub(2)).is_none(),
            "a refused exit cannot price a return"
        );
        assert!(
            dirty_forward.at(victim.saturating_sub(1)).is_none(),
            "a refused interior bar invalidates the path rather than silently \
             shortening its extrema"
        );
        assert!(dirty_forward.was_refused(victim.saturating_sub(2)));
    }

    /// Duplicate time is locally a valid candle and statefully refused. The
    /// evaluator's stored membership, not a second predicate, gates entries,
    /// exits and interior forward paths.
    #[test]
    fn duplicate_timestamp_never_prices_a_forward_entry_exit_or_interior() {
        let clean = crate::synthetic::sessions(8);
        let mut dirty = clean.clone();
        let victim = dirty.len().saturating_sub(100);
        let prior = dirty
            .get(victim.saturating_sub(1))
            .map_or(0, |bar| bar.ts_micros);
        if let Some(bar) = dirty.get_mut(victim) {
            bar.ts_micros = prior;
        }
        assert!(dirty.get(victim).is_some_and(|bar| bar.check().is_ok()));
        let column = Column::build(&dirty, &mut evaluator(Availability::Absent));
        assert_eq!(column.acceptance_census().timestamp_not_increasing, 1);
        assert_forward_refuses_the_path(&clean, &dirty, Availability::Absent, victim);
    }

    /// A VWAP accumulator overflow is also locally valid and stateful. Its
    /// extreme price cannot leak into a forward return.
    #[test]
    fn oversized_accumulator_never_prices_a_forward_entry_exit_or_interior() {
        let clean = crate::synthetic::sessions(8);
        let mut dirty = clean.clone();
        let victim = dirty.len().saturating_sub(100);
        let stamp = dirty.get(victim).map_or(0, |bar| bar.ts_micros);
        if let Some(bar) = dirty.get_mut(victim) {
            *bar = Candle::new(
                stamp,
                5_000_000_000_000_000_000,
                5_000_000_000_000_000_000,
                5_000_000_000_000_000_000,
                5_000_000_000_000_000_000,
                1,
                OI_NULL,
            );
        }
        assert!(dirty.get(victim).is_some_and(|bar| bar.check().is_ok()));
        let column = Column::build(&dirty, &mut evaluator(Availability::Present));
        assert_eq!(column.acceptance_census().accumulator_too_large, 1);
        assert_forward_refuses_the_path(&clean, &dirty, Availability::Present, victim);
    }
}
