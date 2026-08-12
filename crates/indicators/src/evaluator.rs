//! One bar in, its whole mask out.
//!
//! # Why this exists
//!
//! Before this module, `crates/indicators` had **seven modules and no evaluator**.
//! Each returned its own [`ConditionMask`] and no production code unioned them —
//! the only `union` calls in the crate were inside its own tests. So "give me every
//! condition bit for this bar" was a function that did not exist, and every caller
//! would have had to reinvent the session bookkeeping below. An audit named this the
//! missing integration layer and it was right.
//!
//! # What the bookkeeping actually is
//!
//! Three families need a *completed* session, not the running one:
//!
//! - [`crate::daily`] needs the previous session's high, low and close to build the
//!   CPR and the S1–S5 / R1–R5 ladder.
//! - [`crate::fib::prev_day_bits`] needs the same levels.
//! - [`crate::fib::Prev5`] needs the last five completed sessions' extremes.
//!
//! None of them can be handed a bar. Something has to notice the IST day change,
//! close the books on the session that just ended, and roll the result forward. That
//! is the whole of what this module adds, and it is why the seven modules could not
//! simply be called in a row.
//!
//! # The order, and why it is not one order
//!
//! Per bar: **roll over, then emit, then fold.**
//!
//! The fold is last because the day's running high and low are an **anchor** — a
//! reference the current bar is measured *against* — so they must exclude it. Were
//! they folded first, the first bar of a session would be compared to a level its own
//! high had just set, and `close_at_day_high` would be true on every opening bar.
//!
//! The rollover is first because *which session we are in* is a **description** of
//! the current bar and must include it. `crate::orb` had this exact bug: it set its
//! `closed` flag inside its fold, after the emit, and the family was false for one
//! bar longer than it should have been, once per window per day.
//!
//! A module holding both kinds of quantity needs both orders. That is not a
//! contradiction; it is the distinction, and it is the one thing to hold onto when
//! reading any of these modules.
//!
//! # Cost
//!
//! Seven module states, each fixed-size, plus four `i64` of session bookkeeping.
//! `size_of` is asserted below. Nothing grows with the number of bars fed, and there
//! is no allocation on the per-bar path — the mask is `Copy` and the unions are
//! bitwise ORs over six `u64`.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

use crate::daily::DailyLevels;
use crate::fib::Prev5;
use crate::gap::GapFib;
use crate::orb::Orb;
use crate::pattern::{Patterns, Thresholds};
use crate::session::{PreviousSession, SessionState};
use crate::trend::TrendState;
use crate::vwap::{Availability, Vwap};
use crate::{Corrupt, CurDayFib};

/// The two tolerance widths a full evaluation needs.
///
/// Two, not one, and this is not a convenience: the Fibonacci families measure their
/// band as thousandths of the **session range**, while the pivot families measure it
/// as thousandths of the **CPR width**. Those are different quantities and a single
/// number cannot serve both — at the width correct for one, the other is off by a
/// factor of fifty. Recorded as D-0079.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Widths {
    /// Band width for the Fibonacci ladders, in thousandths of the session range.
    pub fib: Tolerance,
    /// Band width for the pivot and CPR families, in thousandths of the CPR width.
    pub pivot: Tolerance,
}

impl Widths {
    /// The widths this repository ships.
    ///
    /// # Errors
    ///
    /// [`vocab::VocabError`] if either pinned width is unusable, which a const
    /// assertion in `vocab::tolerance` already makes impossible — this returns a
    /// `Result` rather than panicking because §9 denies `unwrap` in this workspace.
    ///
    /// Written as `and_then` over `map` and not as two `?`, for the reason
    /// `vocab::tolerance`'s own test helper is: `?` writes the refusal arm **here**,
    /// and at the pinned widths no input can take it, so it was a region no green run
    /// could ever execute. `and_then` decides the same thing inside `core` and
    /// short-circuits identically — the pivot width is not even read when the fib width
    /// is refused, and a refused fib width is still the error returned.
    pub fn pinned() -> Result<Self, vocab::VocabError> {
        vocab::tolerance::pinned_fib()
            .and_then(|fib| vocab::tolerance::pinned_pivot().map(|pivot| Self { fib, pivot }))
    }
}

/// Set `above` or `below`, and neither when the two are equal.
///
/// A `match` on the ordering rather than an `if` chain: `Ordering` has exactly three
/// variants, so the compiler checks the equality arm exists instead of a reader having to
/// notice it does. Equality setting NEITHER bit is the rule D-0109 locked — five occurrences
/// of the opposite mistake were found and fixed across this crate, and on a paisa tick grid
/// exact equality is common rather than a measure-zero curiosity.
fn set_side(mask: ConditionMask, value: i64, level: i64, above: u16, below: u16) -> ConditionMask {
    match value.cmp(&level) {
        core::cmp::Ordering::Greater => vocab::table::set_exact(mask, above).unwrap_or(mask),
        core::cmp::Ordering::Less => vocab::table::set_exact(mask, below).unwrap_or(mask),
        core::cmp::Ordering::Equal => mask,
    }
}

/// The IST days that are **not** regular trading sessions.
///
/// # What this is for, and the defect it closes
///
/// `docs/00-charter.md` §3 states the rule and states it exactly:
///
/// > For a bar on any day after a Muhurat session, the previous-day anchor is the OHLC of
/// > the **last regular trading session strictly before the Muhurat date**. The Muhurat
/// > day's own OHLC never enters the previous-day anchor, the multi-day rolling history,
/// > or the previous-session edge.
///
/// It then says the mechanism is "an anchor walk restricted to trading days [that] skips
/// it structurally rather than by a special case that can be forgotten". **No such walk
/// existed.** The session boundary was an `ist_day` inequality and nothing else, so a
/// one-hour Muhurat session was a session like any other: its OHLC became the previous-day
/// anchor, entered `Prev5`, and became the gap family's previous-session edge — the three
/// things the charter forbids by name.
///
/// Measured before this existed: a 375-bar session, then a 60-bar session, then another
/// 375-bar session, and **all 375 bars of the third emitted a different mask** than the
/// same session fed without the short day. The whole 44-position pivot ladder and the 15
/// previous-day Fibonacci rungs were measured off a one-hour session, and `Prev5` stayed
/// contaminated for five more sessions.
///
/// # Why a set of dates and not a rule
///
/// A Muhurat date cannot be derived — it is set by the exchange each year against the Hindu
/// calendar — so §3 rule 1 forbids computing one. The charter records six as **VERIFIED**
/// and those six are what this holds. A seventh must be added here when the exchange
/// announces it, and until then the engine treats it as a regular session, which is the
/// honest failure: wrong in the same direction as before, on one day, and visible.
///
/// # Why the exposure is smaller than it looks, and still real
///
/// Five of the six never reach disk. `pull::fetch::land` drops every minute bar whose IST
/// minute-of-day falls outside `[09:15, 15:30)`, and the 2020–2024 sessions are all
/// **evening** sessions at 18:00 or later — so the pull accidentally implements this rule,
/// for the unrelated reason that it hardcodes a 15:30 close.
///
/// **2025-10-21 is the exception and it is why this is not merely tidiness.** The charter
/// records it as "an afternoon session, not an evening one", 13:45–14:45 IST: every one of
/// its 60 bars sits inside the pull's window, so it lands, it becomes the previous-day
/// anchor, and the prohibition is broken on it. Afternoon Muhurats are now the live
/// pattern, which makes this forward-looking rather than historical.
pub const CHARTER_NON_REGULAR_IST_DAYS: [i64; 6] = [
    18_580, // 2020-11-14, 18:15–19:15 IST
    18_935, // 2021-11-04, 18:15–19:15
    19_289, // 2022-10-24, 18:15–19:15
    19_673, // 2023-11-12, 18:00–19:00
    20_028, // 2024-11-01, 18:00–19:00
    20_382, // 2025-10-21, 13:45–14:45 — an AFTERNOON session, and the one that lands
];

/// Which IST days are not regular sessions.
///
/// A fixed-size set, checked once per session rollover rather than once per bar. The scan is
/// bounded by a compile-time length, so it is O(1) under §3 rule 4 — and it is not on the
/// per-bar path at all.
///
/// Measured by `C-I-01` in `crates/indicators/benches/ratio.rs`. That row times
/// `Evaluator::step` at 1,000 candles folded against 200,000, and the rollover — including
/// this scan — is inside what it times: if the check grew with anything, the per-candle cost
/// would drift and the row would breach. Measured 1.009x.
///
/// Named rather than asserted because gate 12 refuses a cost claim with no proof beside it,
/// and this block was the gate's first refusal after it was written. The claim was true and
/// unaccompanied, which is exactly what that gate exists to catch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calendar {
    days: [i64; 8],
    len: usize,
}

impl Default for Calendar {
    fn default() -> Self {
        Self::charter()
    }
}

impl Calendar {
    /// The six dates `docs/00-charter.md` §3 records as VERIFIED.
    #[must_use]
    pub const fn charter() -> Self {
        // Written out rather than copied from `CHARTER_NON_REGULAR_IST_DAYS` in a loop: the
        // workspace denies indexing, and a `const fn` cannot iterate. The two are held
        // equal by the const assertion below, so this cannot silently drift from the
        // documented source.
        Self {
            days: [18_580, 18_935, 19_289, 19_673, 20_028, 20_382, 0, 0],
            len: 6,
        }
    }

    /// Every day is a regular session.
    ///
    /// For a caller sweeping a slice that provably contains no non-regular date, and for
    /// the tests that pin what the contaminated behaviour used to be.
    #[must_use]
    pub const fn all_regular() -> Self {
        Self {
            days: [0; 8],
            len: 0,
        }
    }

    /// Is this IST day a session whose OHLC must not become an anchor?
    #[must_use]
    pub fn is_non_regular(&self, ist_day: i64) -> bool {
        self.days.iter().take(self.len).any(|d| *d == ist_day)
    }
}

/// The two spellings of the charter's six agree.
///
/// `Calendar::charter` writes the days out because a `const fn` cannot index an array
/// under this workspace's lints. That duplication is only safe if something compares them,
/// and a const assertion is the only thing that can compare them before the code runs.
const _: () = {
    assert!(CHARTER_NON_REGULAR_IST_DAYS.len() == 6);
    let c = Calendar::charter();
    assert!(c.len == CHARTER_NON_REGULAR_IST_DAYS.len());
    assert!(c.days[0] == CHARTER_NON_REGULAR_IST_DAYS[0]);
    assert!(c.days[1] == CHARTER_NON_REGULAR_IST_DAYS[1]);
    assert!(c.days[2] == CHARTER_NON_REGULAR_IST_DAYS[2]);
    assert!(c.days[3] == CHARTER_NON_REGULAR_IST_DAYS[3]);
    assert!(c.days[4] == CHARTER_NON_REGULAR_IST_DAYS[4]);
    assert!(c.days[5] == CHARTER_NON_REGULAR_IST_DAYS[5]);
};

/// Every module, and the session bookkeeping that feeds the ones needing yesterday.
#[derive(Clone, Copy, Debug)]
pub struct Evaluator {
    widths: Widths,
    calendar: Calendar,
    /// The IST day of the bar last folded. `i64::MIN` before the first bar.
    day: i64,
    /// True once at least one bar has been folded into the running session.
    seeded: bool,
    /// The timestamp of the last accepted bar, so a receding one can be refused.
    last_ts: Option<i64>,
    /// The running session's extremes and last close — **including** every bar
    /// folded so far, which is why the fold happens after the emit.
    running_high: i64,
    running_low: i64,
    running_close: i64,
    running_open: i64,
    /// Levels from the last **completed** session. `None` on the first day, where
    /// every position depending on yesterday is correctly false rather than guessed.
    yesterday: Option<DailyLevels>,
    /// Yesterday's shape, for the gap family.
    previous: Option<PreviousSession>,
    prev5: Prev5,
    gap: GapFib,
    trend: TrendState,
    orb: Orb,
    curday: CurDayFib,
    patterns: Patterns,
    session: SessionState,
    vwap: Vwap,
}

// Fixed size. If this assertion ever fails, something in a module started growing
// with the number of bars and the constant-space claim is no longer true.
const _: () = assert!(core::mem::size_of::<Evaluator>() <= 1792);

impl Evaluator {
    /// A fresh evaluator.
    ///
    /// `availability` is the VWAP verdict for the **whole run**, decided by the
    /// caller before the first bar and never revisited. A run must not change its
    /// mind halfway about whether bit 52 means "below VWAP" or "no opinion" — see
    /// [`crate::vwap`]. On index spot it is [`Availability::Absent`], which makes all
    /// twenty VWAP positions false for the run rather than emitting a plain average
    /// wearing a VWAP label.
    #[must_use]
    pub const fn new(widths: Widths, availability: Availability, thresholds: Thresholds) -> Self {
        Self::with_calendar(widths, availability, thresholds, Calendar::charter())
    }

    /// An evaluator with an explicit non-regular-session calendar.
    ///
    /// [`Self::new`] uses [`Calendar::charter`], which is the right default: the six dates
    /// are recorded VERIFIED in `docs/00-charter.md` §3, so defaulting to them needs no
    /// invention, and a caller who forgets this parameter gets the sourced behaviour rather
    /// than the contaminated one. [`Calendar::all_regular`] is available for a slice that
    /// provably contains none.
    #[must_use]
    pub const fn with_calendar(
        widths: Widths,
        availability: Availability,
        thresholds: Thresholds,
        calendar: Calendar,
    ) -> Self {
        Self {
            widths,
            calendar,
            day: i64::MIN,
            seeded: false,
            last_ts: None,
            running_high: 0,
            running_low: 0,
            running_close: 0,
            running_open: 0,
            yesterday: None,
            previous: None,
            prev5: Prev5::new(),
            gap: GapFib::new(),
            trend: TrendState::new(crate::trend::TrendThresholds::CLASSICAL),
            orb: Orb::new(),
            curday: CurDayFib::new(),
            patterns: Patterns::new(thresholds),
            session: SessionState::new(
                crate::session::DayWindows::CLASSICAL,
                crate::session::ShapeLimits::CLASSICAL,
            ),
            vwap: Vwap::for_slice(availability),
        }
    }

    /// Every condition bit this crate can compute for `bar`.
    ///
    /// # Errors
    ///
    /// [`Corrupt::HighBelowLow`] or [`Corrupt::RangeOverflows`] for a record that is
    /// not a bar. Checked **once, here**, before any module sees it: seven modules
    /// each re-deriving the same refusal is seven chances to disagree about what a
    /// bar is, and a partially-evaluated bar is worse than a refused one.
    pub fn step(&mut self, bar: &Candle) -> Result<ConditionMask, Corrupt> {
        // COMMIT ON SUCCESS, and the `?` is what enforces it.
        //
        // `stepped` takes `self` BY VALUE and hands back a new evaluator beside the
        // mask. On the refusal path the `?` below discards that value and `*self` is
        // never written, so a refused bar cannot leave a trace. This is structural: to
        // half-apply a bar somebody would have to write a second assignment, not merely
        // forget an early return.
        //
        // WHY IT IS NEEDED, measured rather than argued. `Refused::AccumulatorTooLarge`
        // is raised by VWAP AFTER the session rollover and seven module folds have
        // already run. Fed the fixture in this file's own tests -- flat at 5e18 paisa,
        // volume 1 -- the refusal used to leave: two poisoned EMA accumulators, an
        // advanced ORB window, a pushed `prev5`, an installed `yesterday`, and
        // `sessions_completed()` incremented. The NEXT ordinary bar then emitted four
        // bits a fresh evaluator did not, and over 400 following bars the two never
        // agreed again. `last_ts` was also unadvanced, so a retried refused bar was
        // folded again on every retry.
        //
        // D-0097 recorded this invariant as holding on the strength of
        // `a_refused_candle_changes_nothing`. That test walks the CANDLE-VALIDITY path,
        // which is refused before any fold; it never reached the one refusal that fires
        // after them. The entry was right about the fix it described and wrong that the
        // fix was complete.
        //
        // Free, because `Evaluator` is `Copy` and `size_of` is held at or below 1792
        // bytes by the const assertion above -- a 1,664-byte memcpy per bar against a
        // fold that already costs ~320 ns.
        let (next, mask) = self.stepped(bar)?;
        *self = next;
        Ok(mask)
    }

    /// [`Self::step`]'s whole body, on a value that is thrown away if it refuses.
    ///
    /// Takes `self` by value deliberately: there is no `&mut self` here to half-write.
    /// A `&mut self` variant is precisely the shape that produced the torn state this
    /// replaces, so the signature is load-bearing rather than stylistic.
    fn stepped(mut self, bar: &Candle) -> Result<(Self, ConditionMask), Corrupt> {
        if bar.high < bar.low {
            return Err(Corrupt::HighBelowLow);
        }
        if bar.high.checked_sub(bar.low).is_none() {
            return Err(Corrupt::RangeOverflows);
        }
        // Containment, and it was missing. `open` and `close` are ticks, so both must
        // lie inside the extremes; when they do not, every wick length goes negative
        // and every `_at_most` shape predicate is satisfied by a `<=` against a
        // positive bound. The result is a bar labelled "no wicks" because its wicks
        // are impossible. `store::format::Bar::ohlc_is_sane` already refuses exactly
        // this — the check was absent here because a doc comment in this crate said
        // that function checked field order only, and it does not.
        if bar.open > bar.high || bar.close > bar.high || bar.open < bar.low || bar.close < bar.low
        {
            return Err(Corrupt::PriceOutsideRange);
        }
        // Ordering, and it was also missing. The rollover below triggers on
        // `today != self.day` — an inequality — so a receding timestamp closes the
        // books on a LATER session and installs it as `yesterday`. Every level the
        // daily, prev-day and prev-5 families emit would then come from a session
        // after the bar being evaluated: look-ahead, which §3 rule 7 requires be
        // prevented by a mechanism and not by review. A monotonic feed is the store's
        // guarantee, but this crate no longer depends on the store, and a live
        // consumer does not control its feed.
        if let Some(last) = self.last_ts
            && bar.ts_micros <= last
        {
            return Err(Corrupt::TimestampNotIncreasing);
        }

        // ── roll over first: which session we are in describes THIS bar ─────────
        let today = crate::ist_day(bar.ts_micros);
        if today != self.day {
            if self.seeded {
                // The session that just ENDED is `self.day`, not `today`. Asking about the
                // wrong one is the obvious slip here and it inverts the fix: a Muhurat
                // session would poison the anchor and the regular day after it would be
                // discarded instead.
                let ending_was_regular = !self.calendar.is_non_regular(self.day);
                self.close_the_books(ending_was_regular);
            }
            self.day = today;
            self.seeded = false;
        }

        // ── emit: every module, unioned ─────────────────────────────────────────
        let mut mask = ConditionMask::ZERO;
        mask = mask.union(&self.curday.step(bar, self.widths.fib)?);
        mask = mask.union(&self.patterns.step(bar)?);
        mask = mask.union(&self.orb.step(bar, self.widths.fib)?);
        mask = mask.union(&self.session.step(bar, self.previous, self.widths.fib)?);
        mask = mask.union(&self.prev5.bits(bar.close, self.widths.fib));
        mask = mask.union(&self.gap.step(bar, self.widths.fib, &self.calendar)?);
        mask = mask.union(&self.trend.step(bar, self.widths.fib)?);
        if let Some(levels) = self.yesterday.as_ref() {
            mask = mask.union(&crate::daily::bits(levels, bar.close, self.widths.pivot));
            mask = mask.union(&crate::fib::prev_day_bits(
                levels,
                bar.close,
                self.widths.fib,
            ));
        }
        // VWAP has one refusal the shared check cannot make — a negative volume — and
        // one the shared check makes differently, the accumulator ceiling. Both used to
        // be discarded by `if let Ok(v) = ...`, which is the §4 fallback that hides a
        // failure in its purest form: a bar VWAP called corruption came back as a
        // SUCCESSFUL evaluation with the whole twenty-position family silently absent,
        // indistinguishable from a run that legitimately has no volume. Three separate
        // review dimensions found it independently.
        //
        // A refused bar is now a refused bar. `Refused::Corrupt` is already what
        // `Candle::check` returns and cannot fire here; the other two are mapped so the
        // caller learns which, rather than learning nothing.
        match self.vwap.step(bar, self.widths.fib) {
            Ok(v) => mask = mask.union(&v),
            Err(crate::vwap::Refused::Corrupt(why)) => return Err(why),
            Err(crate::vwap::Refused::AccumulatorTooLarge) => {
                return Err(Corrupt::AccumulatorTooLarge);
            }
        }

        // ── 276–279: two predicates from state this type already held ───────────
        //
        // Both read the session's own bookkeeping rather than a module's, which is why they
        // live here and not in one. `running_open` was written on the first bar of every
        // session and read NOWHERE until this block existed.
        //
        // Read BEFORE the fold, like every other anchor: `running_open` is the session's
        // first bar's open, and on that first bar there is no session open yet to compare
        // against — `seeded` is false and neither bit is set. On the first bar the close
        // against its own open is bits 30/31's question, not this one's.
        if self.seeded {
            mask = set_side(mask, bar.close, self.running_open, 276, 277);
        }
        if let Some(direction) = self.trend.structure_in_force() {
            let index = match direction {
                crate::trend::Trend::Up => 278,
                crate::trend::Trend::Down => 279,
            };
            mask = vocab::table::set_exact(mask, index).unwrap_or(mask);
        }

        // ── fold last: the day's extremes are an anchor and must exclude the bar ─
        if self.seeded {
            if bar.high > self.running_high {
                self.running_high = bar.high;
            }
            if bar.low < self.running_low {
                self.running_low = bar.low;
            }
        } else {
            self.running_high = bar.high;
            self.running_low = bar.low;
            self.running_open = bar.open;
            self.seeded = true;
        }
        self.running_close = bar.close;
        self.last_ts = Some(bar.ts_micros);

        // Nothing outside the live vocabulary may reach a caller: a retired or void
        // position that escaped would be swept as a real condition.
        Ok((self, vocab::table::only_live(mask)))
    }

    /// Hand the finished session to the families that need yesterday.
    /// Hand the finished session to the families that need yesterday — **if it was a
    /// regular one.**
    ///
    /// A non-regular session's bars are emitted exactly like any other bar's: it is a real
    /// hour of real trading and every intraday position on it is a genuine measurement. What
    /// it must not do is become an ANCHOR — the previous-day pivot ladder, the five-session
    /// rolling window, or the gap family's previous-session edge. `docs/00-charter.md` §3
    /// states that rule and D-0110 records why the mechanism it claimed did not exist.
    ///
    /// Skipping the push leaves the previous regular session in place, which is exactly what
    /// the charter asks for: *"the previous-day anchor is the OHLC of the last regular
    /// trading session strictly before the Muhurat date"*. No walk is needed, because
    /// nothing overwrote it.
    fn close_the_books(&mut self, was_regular: bool) {
        if !was_regular {
            return;
        }
        self.prev5
            .push_completed_session(self.running_high, self.running_low);
        // A session whose levels are unusable leaves `yesterday` as it was rather
        // than half-updating it: stale-and-consistent beats fresh-and-partial, and
        // `DailyLevels` already refuses rather than inventing a ladder.
        if let Ok(levels) = DailyLevels::from_previous_session(
            self.running_high,
            self.running_low,
            self.running_close,
        ) {
            self.yesterday = Some(levels);
        }
        self.previous = Some(PreviousSession {
            high: self.running_high,
            low: self.running_low,
            close: self.running_close,
        });
    }

    /// Completed sessions folded so far, saturating at the `Prev5` window.
    #[must_use]
    pub const fn sessions_completed(&self) -> usize {
        self.prev5.filled()
    }

    /// Whether the families that need yesterday can answer yet.
    #[must_use]
    pub const fn has_yesterday(&self) -> bool {
        self.yesterday.is_some()
    }

    /// Can **every** family answer yet?
    ///
    /// # Why a caller needs this, and what happens without it
    ///
    /// `min_hits` filters on measured support, and measured support is depressed by warm-up
    /// in a way that is invisible in the result. `Prev5::extremes` is `None` until five
    /// sessions have been pushed, so positions 110–120 are **false** on every bar before
    /// that — for a reason that is not a measurement. Over six 375-bar sessions that is
    /// 1,875 of 2,250 bars, and a condition genuinely present on three bars of every
    /// session has a true support of 18 and a **measured support of 3**.
    ///
    /// At `min_hits = 10` that condition is dropped at k=1, and by anti-monotonicity every
    /// combination containing it dies with it. **This is a completeness loss that the
    /// Apriori proof cannot see**: the ladder correctly keeps every frequent set, and the
    /// set was made infrequent before the ladder ever saw it.
    ///
    /// The engine cannot fix this. `Ladder::walk` receives `&[ConditionMask]` and nothing
    /// else — a false bit and an unanswerable bit are the same bit to it, and that is the
    /// right shape for it to have. So the decision belongs to whoever feeds it, and this is
    /// the signal they need: **start the sweep at the first bar where this is true**, or
    /// accept a diluted support and know that you have.
    ///
    /// # What "every family" means, and why it is the longest of them
    ///
    /// The families warm at wildly different rates, and the answer is the slowest:
    ///
    /// | Family | Warm when |
    /// |---|---|
    /// | previous-day pivots, previous-day Fibonacci | one completed session |
    /// | the five-session Fibonacci ladder | **five** completed sessions |
    /// | the gap family | one completed session, for the previous close |
    /// | `SuperTrend`, positions 64–65 | `atr_period` candles folded |
    /// | the EMAs, positions 0–5 | **200** candles folded, at `CLASSICAL` |
    /// | the opening ranges | their own window has closed — 60 minutes for the longest |
    ///
    /// Five sessions dominates: 1,875 one-minute bars against 200 for the slowest average.
    /// The answer is deliberately the conjunction rather than a per-family report, because a
    /// caller that starts a sweep needs one boundary and a partial one is the defect.
    ///
    /// # This is a signal, not a filter
    ///
    /// `step` keeps emitting before this is true, and those bits are correct — a position
    /// that cannot be evaluated evaluates false, which is `docs/03-vocabulary.md` §4 and not
    /// a compromise. Nothing here silently drops a bar. `docs/06-limits.md` records the
    /// consequence for `min_hits`, because a signal a caller may ignore is not a guarantee.
    #[must_use]
    pub fn every_family_can_answer(&self) -> bool {
        self.warmed_up() && self.orb.every_window_closed()
    }

    /// Has the RUN warmed up? Monotone, and one boundary for the whole sweep.
    ///
    /// # Why this is separate from [`Self::every_family_can_answer`], and why that matters
    ///
    /// The first version of this was one function, and it was wrong in a way a test caught:
    /// **the opening ranges re-open every session.** At minute 0 of every trading day the
    /// 60-minute window has no level again, so a signal that includes them goes FALSE at
    /// every session start and is not a warm-up signal at all — it is a per-bar
    /// can-everything-answer signal, which is a different question.
    ///
    /// Both are worth having and they answer different things:
    ///
    /// | | Monotone? | False when |
    /// |---|---|---|
    /// | `warmed_up` | **yes**, once true it stays true | the run has not seen five sessions, one previous day, or 200 candles |
    /// | `every_family_can_answer` | no | additionally, during the first 60 minutes of any session |
    ///
    /// **Use `warmed_up` to choose where a sweep starts.** Using the other would discard the
    /// first hour of every session — a quarter of the bars, and the most active quarter —
    /// which trades one distortion for a larger one.
    ///
    /// Use `every_family_can_answer` to interpret a single bar: it is what says an
    /// `orb60_*` position is false because the window has not closed rather than because the
    /// price is elsewhere.
    /// # `previous.is_some()` is redundant today, and stays
    ///
    /// A verification sweep deleted it and nothing caught it — correctly, because nothing
    /// CAN: `close_the_books` assigns `self.previous` unconditionally, so it is `Some` after
    /// any completed session and `prev5.filled() >= 5` already implies five of those.
    ///
    /// It is not removed, and the distinction matters. The other three conditions are
    /// independent and each is guarded by a test that fails without it — `yesterday` in
    /// particular is NOT implied by the session count, because `close_the_books` installs it
    /// only `if let Ok(levels)` and an unusable session fills the window while leaving the
    /// pivot ladder absent. This line states the same requirement for the previous-session
    /// edge, and it stops being redundant the moment that assignment becomes conditional too.
    /// A redundant conjunct that documents a requirement is cheaper than rediscovering the
    /// requirement after the assignment changes.
    #[must_use]
    pub fn warmed_up(&self) -> bool {
        self.prev5.filled() >= 5
            && self.yesterday.is_some()
            && self.previous.is_some()
            && self.trend.every_position_can_answer()
    }

    /// How many completed sessions the five-session ROLLING WINDOW still needs.
    ///
    /// One family's question and nothing else. Zero here means the prev-5 ladder is full;
    /// it does **not** mean the evaluator can answer. This is the arithmetic
    /// [`Self::sessions_until_every_family_can_answer`] used to report under a name that
    /// promised the stronger thing.
    #[must_use]
    pub const fn sessions_until_the_rolling_window_fills(&self) -> usize {
        5_usize.saturating_sub(self.prev5.filled())
    }

    /// How many more completed sessions before every family can answer.
    ///
    /// **Zero if and only if [`Self::every_family_can_answer`] is true**, and the
    /// biconditional is the whole point of this function existing separately from the one
    /// above.
    ///
    /// # It used to return zero while the answer was false
    ///
    /// It was `5 - prev5.filled()`, which answers a different question. An audit found two
    /// states where that reads zero and `every_family_can_answer` is false:
    ///
    /// * five 375-bar sessions plus the first bar of the sixth. The ORB windows re-open
    ///   every session, so the four of them are forming again while the ladder is full.
    /// * five sessions whose own spans leave `i64`, so the rolling window fills and
    ///   `DailyLevels` refuses every one of them. `warmed_up` is false through
    ///   `yesterday.is_some()` and stays false until one further USABLE session arrives.
    ///   This crate's own test asserted `== 0` in exactly that state.
    ///
    /// A caller reading zero concludes it can answer. Reporting zero when it cannot is the
    /// §4 fallback that hides a failure, in a function whose only job is to describe the
    /// failure.
    ///
    /// Reported in sessions rather than bars because the binding constraint is the
    /// five-session ladder, and a bar count would be a guess about how many bars a session
    /// holds — 375 on a regular day and 60 on the 2025 Muhurat.
    #[must_use]
    pub fn sessions_until_every_family_can_answer(&self) -> usize {
        if self.every_family_can_answer() {
            return 0;
        }
        // The rolling window is the only remaining need that is COUNTABLE. When it is
        // already full and something else is not ready, the honest answer is "at least one
        // more" -- never zero, because zero is what a caller reads as ready.
        self.sessions_until_the_rolling_window_fills().max(1)
    }

    /// Every position this evaluator can ever set.
    ///
    /// The union of nine sources' own `positions()` — eight modules and the current-day
    /// Fibonacci rung range — so it cannot drift from them: adding a position to a
    /// module adds it here, plus the four this type computes itself. 238 positions today,
    /// which is every live bit in the table.
    #[must_use]
    pub fn positions() -> Vec<u16> {
        let mut all: Vec<u16> = Vec::new();
        all.extend(crate::daily::positions());
        all.extend(crate::pattern::positions());
        all.extend(crate::session::positions());
        all.extend(crate::orb::positions());
        all.extend(crate::fib::positions());
        all.extend(crate::vwap::positions());
        all.extend(crate::gap::GapFib::positions());
        all.extend(crate::trend::TrendState::positions());
        // 276–279 are computed HERE rather than in a module, because both read the session
        // bookkeeping this type owns: `running_open` and the structure latch. So they are
        // named here too — `only_live_positions_are_ever_emitted` compares what `step` emits
        // against this list, and it caught their absence the moment they started firing.
        all.extend([276, 277, 278, 279]);
        let last = crate::CURDAY_FIRST
            .saturating_add(u16::try_from(crate::CURDAY_RUNGS.len()).unwrap_or(0))
            .saturating_sub(1);
        all.extend(crate::CURDAY_FIRST..=last);
        all.sort_unstable();
        all.dedup();
        all
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test that \
              cannot panic cannot fail — and here it buys a second thing. \
              `unreachable!` expands to a panic inside THIS crate, so llvm-cov counts \
              a region that can never run and the coverage gate can never reach 100 on \
              this file. `.expect` panics inside core, which is not instrumented: the \
              same failure, with the refused value printed beside the message, and no \
              dead region left behind."
)]
mod tests {
    use super::*;

    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
    const MINUTE_MICROS: i64 = 60 * 1_000_000;

    fn widths() -> Widths {
        Widths::pinned().expect("both pinned widths are valid")
    }

    fn fresh() -> Evaluator {
        Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL)
    }

    /// An evaluator that will actually run VWAP.
    ///
    /// `Availability::Present` is load-bearing for the refusal tests: on index spot the
    /// verdict is `Absent` and the whole family abstains, so the accumulator ceiling --
    /// the one refusal that fires after seven modules have folded -- is unreachable.
    fn fresh_with_volume() -> Evaluator {
        Evaluator::new(widths(), Availability::Present, Thresholds::CLASSICAL)
    }

    /// A synthetic session. Prices wander so bars differ from one another.
    fn session(day: i64, n: i64) -> Vec<Candle> {
        (0..n)
            .map(|m| {
                let mid = 2_500_000 + ((m * 137) % 811 - 405) * 7;
                Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * 60 * 1_000_000,
                    open: mid,
                    high: mid + 300 + (m % 11) * 20,
                    low: mid - 300 - (m % 7) * 20,
                    close: mid + ((m % 5) - 2) * 40,
                    volume: 0,
                    open_interest: i64::MIN,
                }
            })
            .collect()
    }

    /// The evaluator's reach is exactly the union of its modules'.
    ///
    /// Derived rather than listed, so adding a position to any module adds it here
    /// and a hand-maintained list cannot drift.
    #[test]
    fn the_position_set_is_the_union_of_the_modules() {
        let all = Evaluator::positions();
        assert_eq!(
            all.len(),
            238,
            "238 positions are computable — every live one"
        );
        let mut sorted = all.clone();
        sorted.sort_unstable();
        assert_eq!(all, sorted, "positions() must be sorted");
        let mut deduped = all.clone();
        deduped.dedup();
        assert_eq!(
            all.len(),
            deduped.len(),
            "no module may claim another's bit"
        );
        for p in &all {
            assert!(
                vocab::table::definition(*p).is_some(),
                "position {p} is not in the vocabulary"
            );
        }
    }

    /// Every session's first bar compares itself to nothing -- in EVERY session.
    ///
    /// The rollover clears `seeded`, so positions 276 and 277 have no session open to read
    /// on the bar that establishes one. Deleting that single line survived the whole
    /// suite: `the_close_against_the_session_open_sets_at_most_one_bit` uses ONE session,
    /// so it reaches the first-bar case exactly once, and on that bar `seeded` is false
    /// whether or not the reset exists.
    ///
    /// From session two onward the mutation compares the close against the PREVIOUS
    /// session's open, and `running_high`/`running_low` then span two days -- so the next
    /// `close_the_books` builds the whole pivot ladder from a two-day range. The fixture
    /// opens each session far from the last precisely so the wrong comparison is visible.
    #[test]
    fn the_first_bar_of_every_session_has_no_session_open_to_compare_against() {
        let at = |day: i64, m: i64, open: i64, close: i64| Candle {
            ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
            open,
            high: open.max(close) + 300,
            low: open.min(close) - 300,
            close,
            volume: 0,
            open_interest: i64::MIN,
        };
        let above = named_position("close_above_day_open");
        let below = named_position("close_below_day_open");

        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        // Three sessions, each opening a long way from the one before, so a comparison
        // against the wrong session's open lands decisively on one side.
        for (day, base) in [
            (24_000_i64, 2_500_000_i64),
            (24_001, 2_400_000),
            (24_002, 2_600_000),
        ] {
            let first = e
                .step(&at(day, 0, base, base + 500))
                .expect("a sane candle");
            assert!(
                !first.get(above) && !first.get(below),
                "on the first bar of IST day {day} the session open is the bar's own open, \
                 which is not established until after the emit -- so neither side may be \
                 claimed. A set bit here means the comparison used another session's open."
            );
            // Two more bars so the session is a session, not a single stamp.
            let _ = e
                .step(&at(day, 1, base + 500, base + 900))
                .expect("a sane candle");
            let _ = e
                .step(&at(day, 2, base + 900, base + 200))
                .expect("a sane candle");
        }
    }

    /// A regular day is regular, and the calendar's zero padding is not a holiday.
    ///
    /// `Calendar` stores `[i64; 8]` with a `len`, and `charter()` fills six and leaves two
    /// zeros. Dropping `.take(self.len)` from `is_non_regular` makes **IST day 0** --
    /// 1970-01-01 -- non-regular, and nothing asserted that a regular day is regular. The
    /// observable consequence measured by an audit: a session on 1970-01-01 flips
    /// `has_yesterday` from true to false and `sessions_completed` from 1 to 0.
    ///
    /// Both directions are checked, and `all_regular()` too, because a calendar that says
    /// "no" to everything would satisfy only half of this.
    #[test]
    fn the_calendar_answers_both_ways_and_its_padding_is_not_a_date() {
        let charter = Calendar::charter();
        assert!(
            !charter.is_non_regular(0),
            "IST day 0 is 1970-01-01 and is not in the charter. A `true` here is the \
             zero padding being read as a date, which is what dropping `.take(len)` does."
        );
        for day in [1_i64, 18_579, 18_581, 19_288, 20_381, 20_383, 24_000] {
            assert!(
                !charter.is_non_regular(day),
                "IST day {day} is an ordinary session and the charter does not name it"
            );
        }
        for day in CHARTER_NON_REGULAR_IST_DAYS {
            assert!(
                charter.is_non_regular(day),
                "IST day {day} IS one of the charter's six and must be named"
            );
        }

        let none = Calendar::all_regular();
        assert!(!none.is_non_regular(0), "an empty calendar names no day");
        for day in CHARTER_NON_REGULAR_IST_DAYS {
            assert!(
                !none.is_non_regular(day),
                "`all_regular` must answer no even for a charter date, or the two \
                 constructors are not distinguishable and the Muhurat tests prove nothing"
            );
        }
    }

    /// Nothing outside the live vocabulary ever reaches a caller.
    ///
    /// A retired (6, 19, 25) or void (39 of them) position escaping would be swept
    /// as a real condition, which §3.8 forbids.
    #[test]
    fn only_live_positions_are_ever_emitted() {
        let mut e = fresh();
        for day in 0..3_i64 {
            for bar in &session(20_400 + day, 40) {
                let mask = e.step(bar).expect("a sane bar");
                // NOT `assert_eq!(only_live(mask), mask)`. That re-applies the function
                // under test to its own output, so it holds for whatever `stepped`
                // returned and cannot see `only_live` being removed from the emit. The
                // non-live set is enumerated instead.
                //
                // Said plainly, because it matters for what this test is worth: deleting
                // `only_live` from `Evaluator::stepped` is currently an EQUIVALENT
                // mutation, since no module in the crate emits a retired or void
                // position. The call is defence in depth against a future module that
                // does, and this assertion is what would catch that module.
                for index in 0..vocab::table::NEXT_FREE {
                    if !vocab::table::is_live(index) {
                        assert!(
                            !mask.get(u32::from(index)),
                            "non-live position {index} escaped into an emitted mask"
                        );
                    }
                }
                let claimed = Evaluator::positions();
                let mut bit: u32 = 0;
                while bit < vocab::ConditionMask::BITS {
                    if mask.get(bit) {
                        let index =
                            u16::try_from(bit).expect("bit < ConditionMask::BITS, and that is 384");
                        assert!(
                            claimed.contains(&index),
                            "bit {bit} was set but no module claims it"
                        );
                    }
                    bit = bit.saturating_add(1);
                }
            }
        }
    }

    /// The families needing yesterday are silent on day one and speak on day two.
    ///
    /// This is the bookkeeping this module exists for. Before it, nothing closed the
    /// books on a session, so `DailyLevels` and `Prev5` could never be fed at all.
    #[test]
    fn yesterday_arrives_only_after_a_session_completes() {
        let mut e = fresh();
        assert!(!e.has_yesterday(), "no session has completed yet");
        assert_eq!(e.sessions_completed(), 0);

        for bar in &session(20_500, 30) {
            let _ = e.step(bar).expect("a sane bar");
        }
        assert!(!e.has_yesterday(), "the first session has not ended yet");

        // The first bar of the next IST day closes the books on the first.
        let next = session(20_501, 1).first().copied().expect("one bar");
        let _ = e.step(&next).expect("a sane bar");
        assert!(e.has_yesterday(), "day two must see day one's levels");
        assert_eq!(e.sessions_completed(), 1);
    }

    /// Prev5 fills over five sessions and then holds.
    #[test]
    fn five_sessions_fill_the_prev5_window() {
        let mut e = fresh();
        for day in 0..8_i64 {
            for bar in &session(20_600 + day, 5) {
                let _ = e.step(bar).expect("a sane bar");
            }
        }
        assert_eq!(
            e.sessions_completed(),
            5,
            "the window saturates at five and does not grow"
        );
    }

    /// A corrupt record is refused once, centrally, before any module sees it.
    #[test]
    fn a_corrupt_record_is_refused_before_any_module_runs() {
        let mut e = fresh();
        let inverted = Candle {
            ts_micros: 0,
            open: 2_500_000,
            high: 2_499_000,
            low: 2_501_000,
            close: 2_500_000,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(e.step(&inverted), Err(Corrupt::HighBelowLow));
        assert!(!e.has_yesterday(), "a refused bar changed no state");
        assert_eq!(e.sessions_completed(), 0);

        let straddling = Candle {
            ts_micros: 0,
            open: 0,
            high: i64::MAX,
            low: i64::MIN,
            close: 0,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(e.step(&straddling), Err(Corrupt::RangeOverflows));
    }

    /// Two runs over one slice agree exactly — §3.5, measured rather than asserted.
    #[test]
    fn two_runs_agree_byte_for_byte() {
        let bars: Vec<Candle> = (0..4_i64).flat_map(|d| session(20_700 + d, 25)).collect();
        let once: Vec<ConditionMask> = {
            let mut e = fresh();
            bars.iter().filter_map(|b| e.step(b).ok()).collect()
        };
        for _ in 0..3 {
            let again: Vec<ConditionMask> = {
                let mut e = fresh();
                bars.iter().filter_map(|b| e.step(b).ok()).collect()
            };
            assert_eq!(again, once, "a rerun disagreed with the first run");
        }
    }

    /// With `Availability::Absent`, not one of the twenty VWAP positions is set.
    #[test]
    fn an_absent_volume_verdict_silences_the_whole_vwap_family() {
        let mut e = fresh();
        for bar in &session(20_800, 60) {
            let mask = e.step(bar).expect("a sane bar");
            for p in crate::vwap::positions() {
                assert!(!mask.get(u32::from(p)), "VWAP position {p} was set");
            }
        }
    }

    /// The two pinned widths are two different numbers, and each reaches its own
    /// family — which is the whole of D-0079.
    ///
    /// The Fibonacci ladders measure their band in thousandths of the **session
    /// range**; the pivot families measure it in thousandths of the **CPR width**. The
    /// two pins differ by a factor of fifty, so a transposed pair — `fib` carrying the
    /// pivot width — widens every rung band fiftyfold, and nothing here asserted which
    /// pin landed in which field.
    #[test]
    fn the_two_pinned_widths_are_two_different_numbers() {
        let w = widths();
        assert_eq!(
            w.fib.milli(),
            10,
            "the Fibonacci band is 10/1000 of the session range"
        );
        assert_eq!(
            w.pivot.milli(),
            500,
            "the pivot band is 500/1000 of the CPR width"
        );
        assert_ne!(w.fib, w.pivot, "one number cannot serve both bands");
    }

    /// A mis-assembled OHLC is refused on **each** of the four ways containment fails.
    ///
    /// `open` and `close` are ticks, so both must lie inside the extremes. When one
    /// does not, every wick length goes negative and every `_at_most` shape predicate
    /// is satisfied by a negative against a positive bound — the bar comes back
    /// labelled "no wicks" precisely because its wicks are impossible. The check is a
    /// chain of four `||` comparisons, so a case that moves one field can only ever
    /// prove one of them; each case below moves exactly one, and the base fixture is
    /// asserted legal so the four cannot be satisfied by refusing everything.
    #[test]
    fn each_of_the_four_containment_failures_is_refused() {
        let base = session(21_100, 1).first().copied().expect("one bar");
        let cases: [(&str, Candle); 4] = [
            (
                "open above the high",
                Candle {
                    open: base.high + 1,
                    ..base
                },
            ),
            (
                "close above the high",
                Candle {
                    close: base.high + 1,
                    ..base
                },
            ),
            (
                "open below the low",
                Candle {
                    open: base.low - 1,
                    ..base
                },
            ),
            (
                "close below the low",
                Candle {
                    close: base.low - 1,
                    ..base
                },
            ),
        ];
        for (name, bar) in cases {
            let mut e = fresh();
            assert_eq!(
                e.step(&bar),
                Err(Corrupt::PriceOutsideRange),
                "{name}: a mis-assembled OHLC was evaluated as a bar"
            );
            assert_eq!(
                e.sessions_completed(),
                0,
                "{name}: a refused bar started a session"
            );
        }
        let mut e = fresh();
        assert!(
            e.step(&base).is_ok(),
            "the base fixture must itself be legal, or the four cases prove nothing"
        );
    }

    /// A repeated or receding timestamp is refused, and refused **before** the
    /// rollover.
    ///
    /// The rollover triggers on `today != self.day` — an inequality — so a bar from an
    /// earlier IST day would close the books on the **later** session and install it
    /// as `yesterday`. Every pivot, previous-day and five-session level would then come
    /// from a session that has not happened yet, which is the look-ahead §3 rule 7
    /// requires a mechanism against rather than a review. `has_yesterday` is what
    /// carries that here: it is still false after the refusal.
    #[test]
    fn a_receding_timestamp_cannot_install_a_later_session_as_yesterday() {
        let mut e = fresh();
        let day_two = session(21_201, 1).first().copied().expect("one bar");
        let day_one = session(21_200, 1).first().copied().expect("one bar");
        let _ = e.step(&day_two).expect("a sane bar");
        assert_eq!(
            e.step(&day_one),
            Err(Corrupt::TimestampNotIncreasing),
            "a bar from the previous IST day was accepted"
        );
        assert!(
            !e.has_yesterday(),
            "the refused bar closed the books on a LATER session"
        );
        assert_eq!(
            e.sessions_completed(),
            0,
            "the refused bar completed a session"
        );
        assert_eq!(
            e.step(&day_two),
            Err(Corrupt::TimestampNotIncreasing),
            "a timestamp is not strictly after itself"
        );
    }

    /// A negative volume reaches the caller as a refusal, not as a mask with twenty
    /// positions quietly missing.
    ///
    /// `volume` is the one field the four checks above do not read, so the refusal
    /// comes back from the first module driven. It used to be discarded by an
    /// `if let Ok(v) = ...` around the VWAP call, which returned the bar as a
    /// **successful** evaluation indistinguishable from a run that legitimately has no
    /// volume — the §4 fallback that hides a failure. Zero volume is asserted legal in
    /// the same test, because §7 says zero is a real zero and refusing it would drop
    /// real index bars from every run.
    #[test]
    fn a_negative_volume_is_a_refusal_and_a_zero_volume_is_not() {
        let mut e = fresh();
        let mut bar = session(21_300, 1).first().copied().expect("one bar");
        bar.volume = -1;
        assert_eq!(
            e.step(&bar),
            Err(Corrupt::NegativeVolume),
            "a negative volume was evaluated as a bar"
        );

        bar.volume = 0;
        let after = e
            .step(&bar)
            .expect("zero volume is a real zero, not corruption");
        let mut clean = fresh();
        let untouched = clean
            .step(&bar)
            .expect("zero volume is a real zero, not corruption");
        assert_eq!(after, untouched, "the refused bar left state behind");
    }

    /// An overflowing volume-weighted accumulator reaches the caller as a refusal.
    ///
    /// This is the **second** thing that used to be swallowed by the `if let Ok(v)` on
    /// the VWAP call, and unlike a negative volume it is not checkable from the record
    /// alone — it depends on what the session has already accumulated, so it cannot be
    /// hoisted into `Candle::check` and this is the only place it can surface. A bar at
    /// 5 × 10^18 paisa makes `(high + low + close)²` about 2.25 × 10^38, past `i128`,
    /// and `Vwap::fold` refuses rather than wrapping.
    ///
    /// `Availability::Present` is load-bearing: on index spot the verdict is `Absent`,
    /// the accumulator is never touched, and this refusal cannot happen at all. The
    /// second half of the test is an ordinary volume-bearing bar, so the refusal is
    /// pinned to the magnitude rather than to the verdict.
    #[test]
    fn an_overflowing_accumulator_reaches_the_caller_as_a_refusal() {
        let mut e = Evaluator::new(widths(), Availability::Present, Thresholds::CLASSICAL);
        let open = 21_500 * DAY_MICROS + IST_OPEN_UTC_MICROS;
        let huge = Candle {
            ts_micros: open,
            open: 5_000_000_000_000_000_000,
            high: 5_000_000_000_000_000_000,
            low: 5_000_000_000_000_000_000,
            close: 5_000_000_000_000_000_000,
            volume: 1,
            open_interest: i64::MIN,
        };
        assert_eq!(
            e.step(&huge),
            Err(Corrupt::AccumulatorTooLarge),
            "a squared price past i128 came back as a successful evaluation"
        );

        let ordinary = Candle {
            ts_micros: open + MINUTE_MICROS,
            open: 2_500_000,
            high: 2_500_500,
            low: 2_499_500,
            close: 2_500_100,
            volume: 1_000,
            open_interest: i64::MIN,
        };
        // THE ASSERTION THIS TEST WAS MISSING. `is_ok()` proves the evaluator still
        // ANSWERS; it does not prove the refused bar left no trace, and that is exactly
        // the hole the refusal fell through for as long as this test existed. The only
        // definition of "changed nothing" that means anything is a comparison against a
        // FRESH evaluator fed the identical bars.
        let mut clean = fresh_with_volume();
        let after = e
            .step(&ordinary)
            .expect("an ordinary bar with volume is not refused");
        let want = clean
            .step(&ordinary)
            .expect("an ordinary bar with volume is not refused");
        assert_eq!(
            after.words(),
            want.words(),
            "the bar after the refusal emits a different mask than the same bar on an \
             evaluator that never saw the refused one, so the refusal left state behind. \
             Measured before the fix: four invented bits -- 1 and 3 are close-below-EMA20 \
             and close-below-EMA200, the poisoned accumulators -- plus 43 and 65."
        );

        // And it stays true. The poisoning was not a one-bar artefact: before the fix the
        // two evaluators never agreed again over 400 following bars.
        for m in 2..12_i64 {
            let next = Candle {
                ts_micros: open + m * MINUTE_MICROS,
                open: 2_500_000,
                high: 2_500_400 + m,
                low: 2_499_600 - m,
                close: 2_500_000 + m,
                volume: 500,
                open_interest: i64::MIN,
            };
            let a = e.step(&next).expect("an ordinary bar is not a refusal");
            let b = clean.step(&next).expect("an ordinary bar is not a refusal");
            assert_eq!(
                a.words(),
                b.words(),
                "bar {m} after the refusal still differs"
            );
        }
    }

    /// A refused bar does not roll a session over.
    ///
    /// The refusal fires AFTER the rollover, so a bar on a new IST day could install a
    /// session — `sessions_completed()` incremented and `has_yesterday()` flipped — on
    /// the strength of a bar the caller was told was refused. Measured before the fix:
    /// 0 → 1 and false → true.
    #[test]
    fn a_refused_bar_does_not_complete_a_session() {
        let open = IST_OPEN_UTC_MICROS;
        let mut e = fresh_with_volume();
        for m in 0..30_i64 {
            let bar = Candle {
                ts_micros: open + m * MINUTE_MICROS,
                open: 2_500_000,
                high: 2_500_400,
                low: 2_499_600,
                close: 2_500_000,
                volume: 500,
                open_interest: i64::MIN,
            };
            e.step(&bar).expect("an ordinary session bar");
        }
        let sessions = e.sessions_completed();
        let had = e.has_yesterday();

        // The next IST day, and a bar VWAP must refuse.
        let huge = Candle {
            ts_micros: open + DAY_MICROS,
            open: 5_000_000_000_000_000_000,
            high: 5_000_000_000_000_000_000,
            low: 5_000_000_000_000_000_000,
            close: 5_000_000_000_000_000_000,
            volume: 1,
            open_interest: i64::MIN,
        };
        assert_eq!(e.step(&huge), Err(Corrupt::AccumulatorTooLarge));
        assert_eq!(
            e.sessions_completed(),
            sessions,
            "a refused bar completed a session"
        );
        assert_eq!(
            e.has_yesterday(),
            had,
            "a refused bar installed the previous-day levels, so every pivot and \
             Fibonacci position from here is measured off a bar that was refused"
        );
    }

    /// A completed session whose own span leaves `i64` leaves `yesterday` **absent**
    /// rather than half-updated.
    ///
    /// Every bar is checked for `high - low` fitting `i64`, and that is not the same
    /// question as the **session's** span fitting: the running high and the running low
    /// come from two different bars, so two zero-range bars at opposite ends of the
    /// type make a session wider than the type. `DailyLevels` refuses such a session,
    /// and `close_the_books` then keeps the answer it had rather than writing half of a
    /// new one — stale-and-consistent beats fresh-and-partial. Both halves are
    /// asserted: `Prev5` does count the session, because it keeps only a pair of
    /// extremes, and the pivot ladder is not installed.
    #[test]
    fn a_session_wider_than_the_type_leaves_yesterday_absent() {
        const HALF: i64 = i64::MAX / 2;
        let day = 21_400_i64;
        let flat = |ts: i64, price: i64| Candle {
            ts_micros: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0,
            open_interest: i64::MIN,
        };
        let session_open = |d: i64| d * DAY_MICROS + IST_OPEN_UTC_MICROS;

        let mut e = fresh();
        let _ = e
            .step(&flat(session_open(day), -HALF - 2))
            .expect("a zero-range bar at the bottom of the type is a real bar");
        let _ = e
            .step(&flat(session_open(day) + MINUTE_MICROS, HALF + 2))
            .expect("a zero-range bar at the top of the type is a real bar");
        // The session now spans (HALF + 2) - (-HALF - 2), which is i64::MAX + 3, while
        // each of its two bars has a range of zero.
        let mask = e
            .step(&flat(session_open(day + 1), 2_500_000))
            .expect("a sane bar the next day");

        assert_eq!(
            e.sessions_completed(),
            1,
            "the completed session was not handed to Prev5"
        );
        assert!(
            !e.has_yesterday(),
            "a pivot ladder was built from a span that does not fit i64"
        );
        for p in crate::daily::positions() {
            assert!(
                !mask.get(u32::from(p)),
                "daily position {p} was set from a session with no usable levels"
            );
        }
    }

    /// A non-regular session emits its bars and never becomes an anchor.
    ///
    /// # The measurement this reproduces
    ///
    /// A 375-bar regular session, then a 60-bar session, then another 375-bar session. Before
    /// the calendar existed, **all 375 bars of the third session emitted a different mask**
    /// than the same session fed without the short day: the one-hour OHLC had become the
    /// previous-day anchor, entered `Prev5`, and become the gap family's previous-session
    /// edge. `docs/00-charter.md` §3 forbids all three by name and claimed the mechanism was
    /// "an anchor walk restricted to trading days". There was no walk.
    ///
    /// The test drives it both ways round the same evaluator type: once with the short day
    /// declared non-regular, once with `Calendar::all_regular`, and requires them to
    /// DISAGREE. That second half is what stops this passing for the wrong reason — if the
    /// calendar were ignored entirely both runs would agree and the assertion would be
    /// vacuous.
    #[test]
    fn a_non_regular_session_never_becomes_the_previous_day_anchor() {
        // Day 20_382 is 2025-10-21, the afternoon Muhurat, and the one of the charter's six
        // whose bars pass the pull's 09:15–15:30 window and therefore reach disk.
        let short_day = 20_382_i64;
        let session = |day: i64, bars: i64, base: i64| -> Vec<Candle> {
            (0..bars)
                .map(|m| {
                    let p = base + (m % 37) * 100;
                    Candle {
                        ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                        open: p,
                        high: p + 400,
                        low: p - 400,
                        close: p + 100,
                        volume: 0,
                        open_interest: i64::MIN,
                    }
                })
                .collect()
        };

        let before = session(short_day - 1, 375, 2_500_000);
        let shortd = session(short_day, 60, 2_900_000);
        let after = session(short_day + 1, 375, 2_500_000);

        let run = |calendar: Calendar, include_short: bool| -> Vec<[u64; 6]> {
            let mut e = Evaluator::with_calendar(
                widths(),
                Availability::Absent,
                Thresholds::CLASSICAL,
                calendar,
            );
            for c in &before {
                e.step(c).expect("a sane candle");
            }
            if include_short {
                for c in &shortd {
                    e.step(c).expect("a sane candle");
                }
            }
            after
                .iter()
                .map(|c| e.step(c).expect("a sane candle").words())
                .collect()
        };

        // WHAT THE CHARTER ACTUALLY FORBIDS, and what it does not.
        //
        // §3 names three things: the previous-day anchor, the multi-day rolling history, and
        // the previous-session edge. It says nothing about the EMA, the ATR, the swing ring
        // or VWAP — and it should not, because a Muhurat hour is real trading and a
        // 200-period average that skipped it would be a different kind of lie.
        //
        // So this compares the ANCHOR-DERIVED positions only. An earlier version of this
        // test compared the whole mask and failed, correctly: the trend accumulators had
        // legitimately seen the 60 bars. The test was wrong, not the fix.
        let anchored: Vec<u16> = crate::daily::positions().to_vec();
        let anchor_bits = |masks: &[[u64; 6]]| -> Vec<Vec<u16>> {
            masks
                .iter()
                .map(|w| {
                    let m = ConditionMask::from_words(*w);
                    anchored
                        .iter()
                        .copied()
                        .filter(|p| m.get(u32::from(*p)))
                        .collect()
                })
                .collect()
        };

        let with_short = run(Calendar::charter(), true);
        let without = run(Calendar::charter(), false);
        assert_eq!(with_short.len(), without.len(), "both runs emit 375 masks");
        assert_eq!(
            anchor_bits(&with_short),
            anchor_bits(&without),
            "the day after a non-regular session reports different PIVOT positions when that \
             session is fed, so its one-hour OHLC is still reaching the previous-day anchor \
             — which charter §3 forbids by name"
        );

        // And the calendar is doing the work. Declared regular, the same short day DOES
        // move the pivot ladder — the contaminated behaviour — and its presence here is what
        // proves the assertion above is not vacuous.
        let contaminated = run(Calendar::all_regular(), true);
        assert_ne!(
            anchor_bits(&contaminated),
            anchor_bits(&without),
            "declaring the short day regular changed no pivot position, so the calendar is \
             not being consulted and the assertion above would pass with the fix removed"
        );
    }

    /// A non-regular session does not advance the five-session rolling window.
    ///
    /// The second of the charter's three prohibitions, asserted directly rather than through
    /// the bits: `Prev5` is what positions 110–120 are measured against, and a one-hour OHLC
    /// entering it contaminates them for five more sessions.
    #[test]
    fn a_non_regular_session_does_not_advance_the_rolling_window() {
        let short_day = 20_382_i64;
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let feed = |e: &mut Evaluator, day: i64, bars: i64, base: i64| {
            for m in 0..bars {
                let p = base + (m % 37) * 100;
                let bar = Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                    open: p,
                    high: p + 400,
                    low: p - 400,
                    close: p + 100,
                    volume: 0,
                    open_interest: i64::MIN,
                };
                e.step(&bar).expect("a sane candle");
            }
        };

        feed(&mut e, short_day - 2, 375, 2_500_000);
        feed(&mut e, short_day - 1, 375, 2_600_000);
        let sessions_before = e.sessions_completed();
        let had_yesterday = e.has_yesterday();

        // The short day, then a bar on the day AFTER it — which is what triggers the
        // rollover that would push the short session into `Prev5`.
        feed(&mut e, short_day, 60, 2_900_000);
        feed(&mut e, short_day + 1, 1, 2_500_000);

        assert_eq!(
            e.sessions_completed(),
            sessions_before + 1,
            "the rolling window advanced twice across a non-regular session and the regular \
             day after it, so the one-hour OHLC entered Prev5 — the second of the three \
             things charter §3 forbids"
        );
        assert_eq!(
            e.has_yesterday(),
            had_yesterday,
            "yesterday came or went unexpectedly"
        );
    }

    /// The short session's own bars are still emitted.
    ///
    /// The fix must not silence a real hour of trading. Every intraday position on a
    /// Muhurat bar is a genuine measurement — what the charter forbids is that hour becoming
    /// an ANCHOR for the days after it.
    #[test]
    fn a_non_regular_session_still_emits_its_own_bars() {
        let short_day = 20_382_i64;
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let mut any = false;
        for m in 0..60_i64 {
            let p = 2_900_000 + (m % 37) * 100;
            let bar = Candle {
                ts_micros: short_day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                open: p,
                high: p + 400,
                low: p - 400,
                close: p + 100,
                volume: 0,
                open_interest: i64::MIN,
            };
            let mask = e.step(&bar).expect("a Muhurat bar is an ordinary bar");
            if mask.popcount() > 0 {
                any = true;
            }
        }
        assert!(
            any,
            "a non-regular session emitted nothing at all, so the fix silenced a real hour \
             of trading instead of merely keeping it out of the anchors"
        );
    }

    /// Whole families are silent until they warm, and the evaluator says when that stops.
    ///
    /// # The completeness loss this measures
    ///
    /// `min_hits` filters on measured support, and warm-up depresses measured support
    /// invisibly. `Prev5::extremes` is `None` until five sessions have been pushed, so
    /// positions 110–120 are false on 1,875 of 2,250 bars across six sessions — for a reason
    /// that is not a measurement. A condition genuinely present on three bars of every
    /// session then has a true support of 18 and a **measured support of 3**, is dropped at
    /// `min_hits = 10`, and by anti-monotonicity every combination containing it dies.
    ///
    /// The Apriori completeness proof cannot see this. The ladder correctly keeps every
    /// frequent set; the set was made infrequent before the ladder saw it.
    ///
    /// This test pins the two halves a caller depends on: the signal is FALSE while any
    /// family is silent, and TRUE once none is — with the five-session ladder as the binding
    /// constraint.
    #[test]
    fn the_evaluator_reports_when_every_family_can_finally_answer() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let feed = |e: &mut Evaluator, day: i64, bars: i64| {
            for m in 0..bars {
                let p = 2_500_000 + (m % 61) * 300 + day * 17;
                let bar = Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                    open: p,
                    high: p + 500,
                    low: p - 500,
                    close: p + 200,
                    volume: 0,
                    open_interest: i64::MIN,
                };
                e.step(&bar).expect("a sane candle");
            }
        };

        assert!(
            !e.every_family_can_answer(),
            "a fresh evaluator claims every family can answer, so the signal is useless"
        );
        assert_eq!(
            e.sessions_until_every_family_can_answer(),
            5,
            "a fresh evaluator needs five completed sessions for the Prev5 ladder"
        );

        // A session completes when a bar on the NEXT day rolls its books, so five full days
        // fed in order leaves FOUR completed. 375 bars a session is already well past the
        // 200-candle EMA and the 60-minute opening range, so Prev5 is the only family still
        // silent — which is the point: it is the slowest by a wide margin.
        for day in 21_000..=21_004 {
            feed(&mut e, day, 375);
        }
        assert_eq!(e.sessions_completed(), 4);
        assert!(
            !e.every_family_can_answer(),
            "four completed sessions is reported as fully warm, but the five-session \
             Fibonacci ladder cannot answer — positions 110-120 are still structurally \
             false and every support they touch is depressed"
        );
        assert_eq!(e.sessions_until_every_family_can_answer(), 1);

        // The sixth day's first bar rolls the fifth session's books, and that completes it.
        // 61 bars, not 60: the longest opening range is 60 minutes and closes at minute 60.
        feed(&mut e, 21_005, 61);
        assert_eq!(e.sessions_completed(), 5);
        assert_eq!(e.sessions_until_every_family_can_answer(), 0);
        assert!(
            e.warmed_up(),
            "five completed sessions and 1,875 bars folded, and the run still reports \
             itself unwarmed — a caller waiting on this signal would never start"
        );
        assert!(
            e.every_family_can_answer(),
            "the run is warm and the 60-minute window has closed, so every family can answer"
        );
    }

    /// The opening ranges re-open every session, so only ONE of the two signals is monotone.
    ///
    /// This is the distinction the first version of these signals collapsed, and a test
    /// caught it: at minute 0 of every trading day the 60-minute window has no level again.
    /// A signal that includes the opening ranges therefore goes false at every session start,
    /// which makes it a per-bar question rather than a warm-up boundary.
    ///
    /// Conflating them has a real cost either way round. Start a sweep on
    /// `every_family_can_answer` and it discards the first hour of every session — a quarter
    /// of the bars, and the most active quarter. Interpret a single bar with `warmed_up` and
    /// an `orb60_*` false reads as "price is elsewhere" when it means "the window has not
    /// closed".
    #[test]
    fn only_the_run_level_signal_is_monotone() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        // Takes a start minute as well as a count: a session must be fed in one monotone
        // sweep, and restarting a day at minute 0 is refused by the timestamp guard — which
        // is `Evaluator` doing its job and cost two attempts at this test to remember.
        let feed = |e: &mut Evaluator, day: i64, from: i64, bars: i64| {
            for m in from..from + bars {
                let p = 2_500_000 + (m % 61) * 300 + day * 17;
                let bar = Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                    open: p,
                    high: p + 500,
                    low: p - 500,
                    close: p + 200,
                    volume: 0,
                    open_interest: i64::MIN,
                };
                e.step(&bar).expect("a sane candle");
            }
        };
        for day in 23_000..=23_005 {
            feed(&mut e, day, 0, 375);
        }
        assert!(e.warmed_up(), "six sessions in, the run is warm");
        assert!(
            e.every_family_can_answer(),
            "and at the end of a full session every window has closed"
        );

        // The next session's FIRST bar. The run is still warm — that is what monotone means —
        // and the 60-minute window is not.
        feed(&mut e, 23_006, 0, 1);
        assert!(
            e.warmed_up(),
            "`warmed_up` went false at a session boundary, so it is not monotone and cannot \
             be used to choose where a sweep starts"
        );
        assert!(
            !e.every_family_can_answer(),
            "the 60-minute opening range reports itself closed on the first bar of a \
             session, so `every_window_closed` is not asking about this session"
        );

        // And it comes back once the longest window has closed.
        feed(&mut e, 23_006, 1, 60);
        assert!(
            e.every_family_can_answer(),
            "sixty-one bars into a session and the 60-minute window still reports itself open"
        );
    }

    /// The signal is a conjunction, so the slowest family decides.
    ///
    /// Without this, `every_family_can_answer` could be `prev5.filled() >= 5` alone and pass
    /// the test above: five sessions of 375 bars warms everything else on the way. This
    /// drives five very SHORT sessions instead — 60 bars each, the 2025 Muhurat's length — so
    /// `Prev5` fills while the 200-candle EMA has seen only 300 candles… which is enough.
    /// So it uses shorter sessions still: five 30-bar sessions is 150 candles, under 200.
    #[test]
    fn the_slowest_family_decides_and_it_is_not_always_prev5() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        for day in 22_000..22_005 {
            for m in 0..30_i64 {
                let p = 2_500_000 + (m % 17) * 200 + day * 11;
                let bar = Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                    open: p,
                    high: p + 400,
                    low: p - 400,
                    close: p + 100,
                    volume: 0,
                    open_interest: i64::MIN,
                };
                e.step(&bar).expect("a sane candle");
            }
        }
        // A bar on a sixth day, to roll the fifth session's books.
        for m in 0..1_i64 {
            let bar = Candle {
                ts_micros: 22_005 * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                open: 2_500_000,
                high: 2_500_400,
                low: 2_499_600,
                close: 2_500_100,
                volume: 0,
                open_interest: i64::MIN,
            };
            e.step(&bar).expect("a sane candle");
        }

        assert_eq!(e.sessions_completed(), 5, "five sessions were completed");
        // The premise is "the session COUNT is satisfied", and that is
        // `sessions_until_the_rolling_window_fills`. This assertion used to read
        // `sessions_until_every_family_can_answer() == 0` -- and every family demonstrably
        // CANNOT answer here, as the two assertions below say. So this test was asserting
        // the defect: the function reported zero remaining sessions in a state where the
        // evaluator could not answer, and a caller reading zero starts sweeping.
        assert_eq!(
            e.sessions_until_the_rolling_window_fills(),
            0,
            "the session count is satisfied"
        );
        assert_eq!(
            e.sessions_until_every_family_can_answer(),
            1,
            "and the honest remaining count is at least one, because the 200-period EMA and \
             the 60-minute opening range are both still silent"
        );
        // `warmed_up`, NOT `every_family_can_answer`. This test passed for the wrong reason
        // until a mutation proved it: with 30-bar sessions the 60-minute opening range never
        // closes, so `every_family_can_answer` returns false from the ORB regardless of what
        // the EMA does — and reducing `warmed_up` to `prev5.filled() >= 5` left every test
        // green. Asserting the run-level signal directly is what makes this about the EMA.
        assert!(
            !e.warmed_up(),
            "five sessions of 30 bars is 150 candles and the 200-period EMA cannot answer \
             yet, so `warmed_up` is only checking the session count and the conjunction it \
             claims to be is not one"
        );
        // And the per-bar signal is false too, for its own separate reason.
        assert!(
            !e.every_family_can_answer(),
            "a 30-bar session cannot have closed a 60-minute window"
        );
    }

    /// The position carrying `label`, resolved from the table rather than written down.
    ///
    /// A bare literal binds a test to an INDEX. Resolving by name binds it to a MEANING,
    /// which is what makes a name permutation in `vocab::table` fail rather than pass --
    /// see `a_positions_name_and_its_meaning_cannot_be_separated`.
    fn named_position(label: &str) -> u32 {
        let found = (0..vocab::table::NEXT_FREE).find(|i| vocab::table::name(*i) == Some(label));
        assert!(
            found.is_some(),
            "no position is named `{label}`, so the caller resolves nothing and would pass \
             by looking nowhere"
        );
        u32::from(found.unwrap_or(vocab::table::NEXT_FREE))
    }

    /// A position's NAME and the meaning the evaluator gives it are one thing.
    ///
    /// # The hole this closes
    ///
    /// §3.8 forbids renumbering, and `crates/vocab/tests/table.rs` guards indices hard: a
    /// moved index fails the contiguous-run test, the retired and void sets are pinned
    /// exactly, and every position below 74 is compared character for character against
    /// `docs/03-vocabulary.md`.
    ///
    /// None of it guards a NAME. An adversarial audit swapped `close_above_day_open` and
    /// `close_below_day_open` -- in `crates/vocab/src/table.rs` and `docs/03-vocabulary.md`
    /// together -- and the whole workspace stayed green, while the swap inverts the meaning
    /// of every stored mask carrying bit 276 or 277. At or above position 74 the document
    /// is DERIVED from the code, so the document test compares the code to a record of
    /// itself; the group check requires only the marker `day_open`, which both names
    /// carry; and uniqueness is untouched by a permutation. `git grep close_above_day_open`
    /// returns two hits -- the table and the document -- while the test below binds the
    /// semantics to the bare literal `276`. Nothing joined a name to a meaning.
    ///
    /// This resolves the index BY NAME and asserts what that index does, so a swap makes
    /// the lookup find the other position and the assertion fail.
    ///
    /// The same hole is still open for `274`/`275` (wide/neutral) and `278`/`279`
    /// (structure up/down in force) -- every pair whose members differ only in direction,
    /// which is every pair a swap can invert. They need the same treatment; that is
    /// recorded in `docs/11-findings.md` rather than claimed closed here.
    #[test]
    fn a_positions_name_and_its_meaning_cannot_be_separated() {
        let above = named_position("close_above_day_open");
        let below = named_position("close_below_day_open");
        assert_ne!(above, below, "two names must resolve to two positions");

        let day = 24_000_i64;
        let at = |m: i64, open: i64, close: i64| Candle {
            ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
            open,
            high: open.max(close) + 300,
            low: open.min(close) - 300,
            close,
            volume: 0,
            open_interest: i64::MIN,
        };

        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        // The seeding bar fixes the session open at 2_500_000 and sets neither bit.
        let _ = e.step(&at(0, 2_500_000, 2_500_000)).expect("a sane candle");

        let up = e.step(&at(1, 2_500_000, 2_501_500)).expect("a sane candle");
        assert!(
            up.get(above),
            "the position NAMED `close_above_day_open` is {above}, and a close 1500 paisa \
             ABOVE the session open did not set it. Swapping the two names in the table is \
             what fails here."
        );
        assert!(
            !up.get(below),
            "`close_below_day_open` is {below} and a close above the open must not set it"
        );

        let down = e.step(&at(2, 2_501_500, 2_499_000)).expect("a sane candle");
        assert!(
            down.get(below),
            "the position NAMED `close_below_day_open` is {below}, and a close 1000 paisa \
             BELOW the session open did not set it"
        );
        assert!(
            !down.get(above),
            "`close_above_day_open` is {above} and a close below the open must not set it"
        );
    }

    /// 276/277 compare the close to the SESSION's open, and a flat close sets neither.
    ///
    /// `running_open` was written on the first bar of every session and read nowhere in the
    /// repository — a field maintained for a question the vocabulary could not ask. Bits
    /// 40–43 give position within the day's RANGE, which is a different question, and 30/31
    /// compare the close to the BAR's own open.
    ///
    /// The flat case is the one that has gone wrong five times in this crate: equality swept
    /// into `else` made `close_below_*` fire on a bar that was exactly at the level. Here it
    /// sets neither, which is what D-0109 locked.
    #[test]
    fn the_close_against_the_session_open_sets_at_most_one_bit() {
        let day = 24_000_i64;
        let at = |m: i64, open: i64, close: i64| Candle {
            ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
            open,
            high: open.max(close) + 300,
            low: open.min(close) - 300,
            close,
            volume: 0,
            open_interest: i64::MIN,
        };

        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        // The session's first bar opens at 2_500_000. On it, `seeded` is false and there is
        // no session open to compare against yet, so neither bit may be set.
        let first = e.step(&at(0, 2_500_000, 2_500_900)).expect("a sane candle");
        assert!(
            !first.get(276) && !first.get(277),
            "the first bar of a session compared itself to a session open that did not exist \
             yet"
        );

        // Above.
        let up = e.step(&at(1, 2_500_900, 2_501_500)).expect("a sane candle");
        assert!(
            up.get(276),
            "a close above the session open did not set 276"
        );
        assert!(!up.get(277), "and must not also set 277");

        // Below.
        let down = e.step(&at(2, 2_501_500, 2_499_000)).expect("a sane candle");
        assert!(
            down.get(277),
            "a close below the session open did not set 277"
        );
        assert!(!down.get(276), "and must not also set 276");

        // Exactly ON it: neither.
        let flat = e.step(&at(3, 2_499_000, 2_500_000)).expect("a sane candle");
        assert!(
            !flat.get(276) && !flat.get(277),
            "a close exactly at the session open set a direction bit, which is the equality \
             swept into `else` that D-0109 records five occurrences of"
        );
    }

    /// 278/279 report the structure in force BETWEEN breaks, not only on them.
    ///
    /// Bits 56–59 are break events, each true on the handful of bars where a level was taken
    /// out. `Structure::last` held the regime between them and was unpublished — a sweep
    /// could ask "did structure break up on this bar" and could not ask "is the structure
    /// up". Neither bit is set before the first break, because there is no structure yet.
    #[test]
    fn the_structure_in_force_is_reported_between_breaks_and_not_before_the_first() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let mut saw_event_without_regime = false;
        let mut saw_regime_without_event = false;
        let mut ever_set = false;

        for m in 0..90_i64 {
            let within = m % 22;
            let close = match m / 22 {
                0 => 2_400_000 + within * 9_000,
                1 => 2_600_000 - within * 10_000,
                2 => 2_380_000 + within * 11_000,
                _ => 2_620_000 - within * 12_000,
            };
            let bar = Candle {
                ts_micros: 25_000 * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                open: close,
                high: close + 800,
                low: close - 800,
                close,
                volume: 0,
                open_interest: i64::MIN,
            };
            let mask = e.step(&bar).expect("a sane candle");
            let event = mask.get(56) || mask.get(57) || mask.get(58) || mask.get(59);
            let regime = mask.get(278) || mask.get(279);
            if regime {
                ever_set = true;
                if !event {
                    saw_regime_without_event = true;
                }
            }
            if event && !regime {
                saw_event_without_regime = true;
            }
            assert!(
                !(mask.get(278) && mask.get(279)),
                "both structure directions were in force at once on bar {m}"
            );
        }

        assert!(
            ever_set,
            "the structure was never in force, so this proves nothing"
        );
        assert!(
            saw_regime_without_event,
            "278/279 were set only on the bars where 56-59 fired, which makes them a \
             duplicate of the break events rather than the regime between them — the whole \
             reason they were appended"
        );
        assert!(
            !saw_event_without_regime,
            "a break event fired on a bar where no direction was in force, so the latch is \
             advancing after the emit rather than before"
        );
    }

    /// The six non-regular dates are the charter's six, derived rather than trusted.
    ///
    /// # Why this test exists
    ///
    /// `CHARTER_NON_REGULAR_IST_DAYS` is six hand-transcribed integers. A verification sweep
    /// changed 2023-11-12 from `19_673` to `19_674` in **both** spellings — so the const
    /// assertion that holds the two in agreement still passed — and **nothing caught it.**
    ///
    /// An off-by-one there is worse than the defect the calendar fixed: the engine would
    /// treat a REGULAR session as non-regular and discard a real anchor, while the actual
    /// Muhurat session went on poisoning the next day's. Two wrongs, in opposite directions,
    /// from one digit.
    ///
    /// So the days are computed from the calendar dates here, by an arithmetic that shares no
    /// code with the constant. `crates/indicators` declares one dependency and it is `vocab`,
    /// so there is no date library to reach for — which is why this is written out. The
    /// charter's §3 table is the source for the dates themselves.
    #[test]
    fn the_six_non_regular_days_are_the_charter_dates() {
        /// Days from 1970-01-01 to the given date, by Howard Hinnant's civil-from-days
        /// inverse. Shares nothing with the constant under test.
        const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
            let y = if m <= 2 { y - 1 } else { y };
            let era = if y >= 0 { y } else { y - 399 } / 400;
            let yoe = y - era * 400;
            let mp = (m + 9) % 12;
            let doy = (153 * mp + 2) / 5 + d - 1;
            let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
            era * 146_097 + doe - 719_468
        }

        // Sanity: the arithmetic itself. If these are wrong the comparison below is worthless.
        assert_eq!(days_from_civil(1970, 1, 1), 0, "the epoch is day zero");
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(
            days_from_civil(2000, 1, 1),
            10_957,
            "a date with a known answer"
        );

        // docs/00-charter.md §3, "Muhurat (Diwali) session ... Verified dates".
        let charter: [(i64, i64, i64); 6] = [
            (2020, 11, 14),
            (2021, 11, 4),
            (2022, 10, 24),
            (2023, 11, 12),
            (2024, 11, 1),
            (2025, 10, 21),
        ];
        for (i, (y, m, d)) in charter.iter().enumerate() {
            let want = days_from_civil(*y, *m, *d);
            let got = CHARTER_NON_REGULAR_IST_DAYS
                .get(i)
                .copied()
                .expect("six dates, six entries");
            assert_eq!(
                got, want,
                "entry {i} is {got} and {y}-{m:02}-{d:02} is day {want}. A transcription \
                 off-by-one makes the engine discard a REGULAR session's anchor while the \
                 real Muhurat session goes on poisoning the next day's."
            );
            // And the constant is actually consulted for that day.
            assert!(
                Calendar::charter().is_non_regular(want),
                "{y}-{m:02}-{d:02} is in the charter's six and the calendar does not know it"
            );
            // The day before and after are regular, which is what stops a fix that marks a
            // whole week non-regular from passing.
            assert!(!Calendar::charter().is_non_regular(want - 1));
            assert!(!Calendar::charter().is_non_regular(want + 1));
        }
    }

    /// 278 means UP and 279 means DOWN, and swapping them is caught.
    ///
    /// `the_structure_in_force_is_reported_between_breaks_..` asserts the two are never both
    /// set and that they fire between events — and it passes with the mapping SWAPPED, so the
    /// regime could have been reported permanently inverted. Found by a verification sweep
    /// applying that exact mutation.
    ///
    /// A sweep asking for "structure up" would have received every bar where the structure
    /// was down: not a missing combination but a false one, which is the worse of the two.
    #[test]
    fn the_structure_direction_bits_are_not_swapped() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        let mut last_break_up: Option<bool> = None;
        let mut compared = 0_u32;

        for m in 0..90_i64 {
            let within = m % 22;
            let close = match m / 22 {
                0 => 2_400_000 + within * 9_000,
                1 => 2_600_000 - within * 10_000,
                2 => 2_380_000 + within * 11_000,
                _ => 2_620_000 - within * 12_000,
            };
            let bar = Candle {
                ts_micros: 26_000 * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                open: close,
                high: close + 800,
                low: close - 800,
                close,
                volume: 0,
                open_interest: i64::MIN,
            };
            let mask = e.step(&bar).expect("a sane candle");

            // 56 bos_up and 58 choch_up are upward breaks; 57 and 59 are downward.
            if mask.get(56) || mask.get(58) {
                last_break_up = Some(true);
            } else if mask.get(57) || mask.get(59) {
                last_break_up = Some(false);
            }

            // The regime must AGREE with the direction of the most recent break.
            if let Some(up) = last_break_up {
                compared += 1;
                assert_eq!(
                    mask.get(278),
                    up,
                    "bar {m}: the last break was {} and 278 `structure_up_in_force` is {}",
                    if up { "UPWARD" } else { "downward" },
                    mask.get(278)
                );
                assert_eq!(
                    mask.get(279),
                    !up,
                    "bar {m}: 279 `structure_down_in_force` disagrees with the last break"
                );
            }
        }
        assert!(
            compared > 20,
            "only {compared} bars were compared against a known break direction, which is \
             too few for this to mean anything"
        );
    }

    /// `warmed_up` needs the previous day, not only the five sessions.
    ///
    /// Deleting `self.yesterday.is_some()` from `warmed_up` was not caught by anything. The
    /// four conditions look coupled — `close_the_books` pushes `prev5` and installs
    /// `yesterday` in the same call — but they are NOT: `yesterday` is installed only
    /// `if let Ok(levels)`, so a session whose levels are unusable fills the rolling window
    /// and leaves the pivot ladder absent.
    ///
    /// Five such sessions give `prev5.filled() == 5` with `has_yesterday() == false`, and a
    /// `warmed_up` that checked only the count would report a run ready to sweep while the
    /// whole 44-position pivot family was structurally silent.
    #[test]
    fn warmed_up_is_false_when_the_pivot_ladder_is_absent_despite_five_sessions() {
        let mut e = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
        // A session whose own span leaves i64: two zero-range bars at opposite ends. Each bar
        // is legal — `Candle::check` bounds `high - low` WITHIN a bar — but the session's
        // extremes come from two different bars, so `DailyLevels` refuses it.
        // 2 extreme bars to make the session unusable, then 40 ordinary ones so the 200-period
        // EMA warms. Both halves are load-bearing: WITHOUT the ordinary bars this test passed
        // for the wrong reason — 10 candles leaves the trend unwarmed, so `warmed_up` returned
        // false from the EMA and the assertion said nothing about `yesterday`. Found by
        // deleting `yesterday.is_some()` and watching it stay green.
        for day in 27_000..=27_005 {
            for m in 0..42_i64 {
                let price = match m {
                    0 => i64::MIN / 2,
                    1 => i64::MAX / 2,
                    _ => 2_500_000 + (m % 17) * 400,
                };
                let bar = Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * MINUTE_MICROS,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: 0,
                    open_interest: i64::MIN,
                };
                e.step(&bar).expect("each bar is individually legal");
            }
        }
        assert_eq!(
            e.sessions_completed(),
            5,
            "the rolling window did not fill, so this proves nothing about the decoupling"
        );
        assert!(
            !e.has_yesterday(),
            "the unusable sessions installed a pivot ladder, so the premise is wrong"
        );
        // The premise the earlier version of this test lacked: everything ELSE is warm, so a
        // false answer can only be coming from `yesterday`.
        // `sessions_until_the_rolling_window_fills`, not
        // `sessions_until_every_family_can_answer`. This assertion is a PREMISE -- it
        // establishes that the five-session ladder is full, so the falseness below can only
        // come from `yesterday`. The other function used to report the same arithmetic, and
        // this test asserting `== 0` here was one of the two states that proved it was
        // reporting the wrong thing: every family CANNOT answer in this state, which is
        // precisely what the next assertion says.
        assert_eq!(
            e.sessions_until_the_rolling_window_fills(),
            0,
            "the rolling window did not fill, so the assertion below is ambiguous"
        );
        assert_eq!(
            e.sessions_until_every_family_can_answer(),
            1,
            "the ladder is full but no session was USABLE, so at least one more is needed. \
             Zero here would tell a caller it can answer."
        );
        assert!(
            !e.warmed_up(),
            "five completed sessions with a warm trend and NO pivot ladder is reported as \
             warm, so `warmed_up` is the session count wearing a conjunction's name — the \
             44-position pivot family is structurally silent and every support it touches is \
             depressed"
        );
    }

    /// `Calendar::default()` is the charter's six, not an empty calendar.
    ///
    /// # Two things this closes, both found by a verification sweep
    ///
    /// `impl Default for Calendar` was the **only uncovered function** in these three crates —
    /// function coverage left 100.00% for the first time when it was added, because nothing
    /// called it. And its body was mutated from `Self::charter()` to `Self::all_regular()` and
    /// **nothing caught that either.**
    ///
    /// The two are the same hole. A `Default` nobody exercises is a `Default` whose value
    /// nobody has checked, and this one decides whether a caller who reaches for the obvious
    /// constructor gets the sourced behaviour or the contaminated one. An empty default would
    /// silently restore the defect D-0110 fixed for every consumer that wrote
    /// `Calendar::default()`.
    #[test]
    fn the_default_calendar_is_the_charters_six_and_not_an_empty_one() {
        let d = Calendar::default();
        assert_eq!(
            d,
            Calendar::charter(),
            "`Calendar::default()` is not the charter's six, so a caller reaching for the \
             obvious constructor gets a calendar that lets a Muhurat session become the \
             previous-day anchor — the defect D-0110 fixed"
        );
        assert_ne!(
            d,
            Calendar::all_regular(),
            "the default is the empty calendar, which is the contaminated behaviour wearing \
             the name of a safe one"
        );
        // And it really does know a charter date, rather than merely comparing equal to
        // something else that does not.
        assert!(
            d.is_non_regular(20_382),
            "the default calendar does not recognise 2025-10-21, the one Muhurat session whose \
             bars reach disk"
        );
    }
}
