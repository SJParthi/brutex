//! Bars in, condition bits out.
//!
//! This is the crate `CLAUDE.md` §5 names and that did not exist until now, so
//! **eleven of the vocabulary's 232 live positions become computable here and
//! the other 221 remain names.** That ratio is the honest state of the engine and
//! is stated first rather than buried.
//!
//! # What is implemented
//!
//! | Module | Positions | Count |
//! |---|---|---:|
//! | [`CurDayFib`] — the current-session Fibonacci ladder | 121–131 | 11 |
//! | [`daily`] — the pivot ladder, the CPR, and yesterday's high and low | 7–18, 54–55, 60–62, 74–85, 178–189 | 41 |
//! | [`orb`] — the opening range at four windows | 86–105 | 20 |
//! | [`fib`] — previous-day ladders, both directions | 19–29, 69–70, 106–109 | 15 |
//! | [`fib::Prev5`] — the five completed sessions | 110–120 | 11 |
//! | [`pattern`] — all 62 candlestick patterns | 153–177, 198–234 | 62 |
//! | [`session`] — bar shape, prior run, day position, time of day, day type, opening gap | 30–51, 66–68 | 25 |
//! | [`vwap`] — session VWAP and three sigma bands | 52–53, 143–152, 190–197 | 20 |
//!
//! **52 of the vocabulary'''s 232 live positions are computable. 180 remain names.**
//!
//! # The three rules this crate exists to keep
//!
//! 1. **No look-ahead** (§3 rule 7). At bar *N* the evaluator may read bars
//!    `0..=N` and nothing later. [`PastPrefix`] enforces it by removing the
//!    future from the borrow, so a look-ahead read is a compile error rather than
//!    a bounds check that might be skipped.
//! 2. **An anchor never includes the bar it is measured against.** The order is
//!    reset, then emit, then fold — fused into [`CurDayFib::step`] so a caller
//!    cannot transpose them. Folding first puts the close inside its own anchor
//!    range by construction, which kills every rung outside that range.
//! 3. **Integers only** (§7). Not one `f32` or `f64`, and no division on the
//!    evaluation path: the rung test is cross-multiplied, so there is no rounding
//!    policy for two platforms to disagree about.
//!
//! # Cost
//!
//! Per bar: one session comparison, eleven cross-multiplied rung tests, two
//! extreme updates. No loop whose length depends on the data, no allocation, and
//! a fixed state whose size is asserted at compile time. Measured on the proof
//! harness at 24.4 → 22.4 nanoseconds per bar across a 100× larger input — level,
//! which is the evidence for §3 rule 4 rather than a claim about it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod anchored;
pub mod column;
pub mod daily;
pub mod evaluator;
pub mod fib;
pub mod gap;
pub mod orb;
pub mod pattern;
pub mod session;
pub mod trend;
pub mod vwap;

use vocab::{ConditionMask, Tolerance};

/// The first vocabulary position of the current-day Fibonacci ladder.
///
/// The ladder occupies 121–131 in `docs/03-vocabulary.md` §7 and this crate does
/// **not** get to choose that. The constant is here so a caller cannot compute a
/// position number by arithmetic and drift from the table.
pub const CURDAY_FIRST: u16 = 121;

/// The eleven rungs, as exact integer numerators over 1000, in the table's order.
///
/// The ladder is a decimal specification, so `p/1000` is exact — there is no
/// irrational to approximate and no square root. The order **must** match
/// positions 121–131, and `indicators::curday::the_rungs_match_the_vocabulary`
/// asserts it against `vocab`'s own names rather than trusting this comment.
pub const CURDAY_RUNGS: [i32; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

/// IST is UTC+05:30 exactly, and India observes no daylight saving.
///
/// `pub` so `crates/runner`'s resampler anchors its bucket grid on the SAME
/// offset rather than a second copy of the number. `crates/pull/src/fold.rs`
/// records what a UTC-anchored grid cost when it shipped: every daily bar moved
/// back one calendar day, and the store held 20 records stamped on a SUNDAY on
/// an exchange that trades Monday to Friday. One definition, three crates.
pub const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;
const MICROS_PER_DAY: i64 = 86_400 * 1_000_000;

/// The IST calendar-day number for a timestamp.
///
/// `div_euclid`, never `/`. Rust's `/` truncates toward zero, so for a pre-epoch
/// timestamp it rounds *up* and silently merges that bar into the 1970-01-01
/// session. `div_euclid` floors, which is what a day number means.
/// `a_pre_epoch_timestamp_floors_into_its_own_session` pins that, one microsecond
/// either side of the boundary.
///
/// `saturating_add`, never `wrapping_add`, and the difference is a SIGN. With the
/// wrap, `ist_day(i64::MAX)` returned **`-106_751_991`** where the true day is
/// +`106_751_991`, so a LATER timestamp produced a SMALLER day number and the
/// rollover in `Evaluator::stepped` saw a session boundary that is not there.
/// Saturating keeps the function monotone, which is the only property the rollover
/// depends on: `today != self.day` must mean the day genuinely changed.
///
/// The saturated answer is not a real date and cannot be. A stamp within 5h30m of
/// `i64::MAX` is the year `292_277_026_596`, so the choice is between a monotone lie
/// and a non-monotone one, and only the monotone lie leaves the rollover sound.
#[inline]
#[must_use]
pub fn ist_day(ts_micros: i64) -> i64 {
    ts_micros
        .saturating_add(IST_OFFSET_MICROS)
        .div_euclid(MICROS_PER_DAY)
}

/// Which weekday a bar falls on, as the vocabulary position that names it.
///
/// # Days since the epoch, and the epoch was a Thursday
///
/// `ist_day` returns days since 1970-01-01 in IST, and **1970-01-01 was a
/// Thursday**. So `day.rem_euclid(7)` gives 0 for Thursday, 1 for Friday, 2 for
/// Saturday, and so on. `rem_euclid` and not `%`, because a pre-epoch timestamp
/// gives a negative remainder under `%` and would index the wrong day — the
/// store holds nothing before 1970, but a `%` here would be correct only by
/// accident of the data.
///
/// NSE trades Monday to Friday, and a Saturday or Sunday bar sets NOTHING
/// rather than being folded into an adjacent day: `docs/03-vocabulary.md` §4 —
/// an unknowable condition is unset, not guessed.
///
/// # The reason this used to give was false, and the charter says so
///
/// It read *"a weekend timestamp in an equity series is a store defect, and
/// silently calling it Friday would hide the one symptom that could reveal
/// it."* The behaviour is right and that justification is not. NSE trades on a
/// weekend more often than the sentence allows, and `docs/00-charter.md` §3
/// records three of them as VERIFIED full 375-bar sessions — the Budget
/// Saturdays 2020-02-01 and 2025-02-01 and the Budget Sunday 2026-02-01. Two
/// more are on disk: the disaster-recovery Saturdays 2024-03-02 and 2024-05-18,
/// 105 bars each. Five weekend sessions, **1,335 real trading bars**, measured
/// in the operator's own store — none of them a defect.
///
/// # What the behaviour actually costs, stated rather than denied
///
/// On those 1,335 bars all five weekday positions are false. That is not
/// *wrong* under §4 — a session the vocabulary cannot name is honestly unnamed
/// — but it is not free either: a weekday-conditioned candidate has those bars
/// in its support DENOMINATOR and never in its numerator, so its measured
/// support is diluted by 0.21% of the series.
///
/// Naming a weekend session would need a sixth and seventh position, and
/// `CLAUDE.md` §3 rule 8 makes appending bits a decision rather than a tidy-up.
/// Until that decision is taken this is the honest statement of the limit; what
/// is fixed here is the claim that there was no limit.
#[must_use]
pub const fn weekday_bit(ts_micros: i64) -> Option<u16> {
    // Inlined rather than calling `ist_day`, which is not `const`.
    let day = ts_micros
        .saturating_add(IST_OFFSET_MICROS)
        .div_euclid(MICROS_PER_DAY);
    match day.rem_euclid(7) {
        0 => Some(368), // Thursday
        1 => Some(369), // Friday
        4 => Some(365), // Monday
        5 => Some(366), // Tuesday
        6 => Some(367), // Wednesday
        // 2 and 3 are Saturday and Sunday: NSE does not trade them.
        _ => None,
    }
}
/// A borrow of the bars up to and including index `n`, and no further.
///
/// This is §3 rule 7 expressed as a **type** rather than a review comment. The
/// future is not hidden behind a bounds check — it is not inside the borrow at
/// all, so a look-ahead read cannot be written, let alone run.
#[derive(Clone, Copy, Debug)]
pub struct PastPrefix<'a> {
    seen: &'a [Candle],
}

impl<'a> PastPrefix<'a> {
    /// Borrow `bars[0..=n]`, or `None` when `n` is past the end.
    #[must_use]
    pub fn upto(bars: &'a [Candle], n: usize) -> Option<Self> {
        bars.get(..=n).map(|seen| Self { seen })
    }

    /// The bar at index `n` — the latest one this prefix can see.
    #[must_use]
    pub fn current(&self) -> Option<&'a Candle> {
        self.seen.last()
    }

    /// Every bar this prefix can see. There is no accessor that returns more.
    #[must_use]
    pub const fn as_slice(&self) -> &'a [Candle] {
        self.seen
    }

    /// How many bars are visible.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.seen.len()
    }

    /// True when nothing is visible.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// Which extreme was made later. This, not the pair of extremes, picks the ladder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Leg {
    /// No range yet, or one bar strictly engulfed the running range, so the two
    /// extremes share a bar and no temporal order exists.
    #[default]
    Undetermined,
    /// The high is the later extreme. Retracements measure *down* from it.
    Up,
    /// The low is the later extreme. Retracements measure *up* from it.
    Down,
}

/// A record that is not a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corrupt {
    /// `high < low`. A market can print a zero range; it cannot print a negative
    /// one, so this is a broken record and not a state to be tolerated. `R == 0`
    /// and `R < 0` deliberately do not share a branch.
    HighBelowLow,
    /// `high - low` does not fit in an `i64`.
    ///
    /// It cannot happen on real market data — the widest Indian index span is
    /// about 10^7 paisa against an `i64` ceiling of 9.2 x 10^18. It can only
    /// happen from a bar this crate was handed directly, never from one that
    /// came off disk.
    ///
    /// **This paragraph previously said `ohlc_is_sane` "checks field ORDER only".
    /// That was false**, and the falsehood had a cost: it is why [`Evaluator`] was
    /// written without a containment check of its own. `ohlc_is_sane` checks full
    /// containment — `high >= open`, `high >= low`, `high >= close`, `low <= open`,
    /// `low <= close` — so the store would have refused the mis-assembled OHLC that
    /// this crate accepted. See [`Corrupt::PriceOutsideRange`].
    ///
    /// **It also previously offered `low = open = close = i64::MIN` as a record
    /// that satisfies `ohlc_is_sane` while overflowing the subtraction. That is
    /// no longer true either**, and this time the predicate moved rather than the
    /// description being wrong: D-0143 added `open|high|low|close >= 0` to
    /// `ohlc_is_sane`, because the containment clauses above are all RELATIVE and
    /// `-100/-100/-100/-100` satisfies every one of them.
    ///
    /// That has a consequence worth stating plainly, because it is easy to read
    /// as making this guard redundant: with `high >= low >= 0`, the difference
    /// `high - low` lies in `0 ..= i64::MAX` and CANNOT overflow. Every bar that
    /// crossed `BarFile::append` is therefore safe here.
    ///
    /// **The guard stays anyway, and the reason is the one that mattered the
    /// first time.** `ohlc_is_sane` is a guarantee about the WRITE BOUNDARY, not
    /// about the `Bar` type — the struct's four fields are plain `i64` and still
    /// hold anything. This crate is handed bars by callers that never went near a
    /// store file, and the last time this crate trusted a neighbouring crate's
    /// predicate to be the whole story, it shipped without a containment check.
    /// A guarantee that holds only for values that arrived by one particular
    /// route is not a guarantee this crate may assume.
    ///
    /// Refused loudly rather than saturated. A saturating range would silently
    /// answer a different question than the one asked, which is the fallback
    /// `CLAUDE.md` §4 bans.
    RangeOverflows,
    /// `open` or `close` lies outside `[low, high]`.
    ///
    /// # Why this needs its own refusal rather than tolerance
    ///
    /// A bar is a summary of ticks, and `open` and `close` are two of those ticks, so
    /// both must lie within the extremes. When they do not, the record was assembled
    /// wrongly — the classic cause is an aggregator dropping or reordering a tick —
    /// and every shape predicate built on it becomes meaningless in a way that reads
    /// as meaningful.
    ///
    /// Concretely, with `open = 2_600_000, high = 2_500_000, low = 2_400_000,
    /// close = 2_300_000`: body is 300,000 against a range of 100,000, and both wicks
    /// come out at **minus** 100,000. Every `_at_most` predicate is a `<=` against a
    /// positive bound, which a negative satisfies, so the bar was labelled a **long
    /// bearish marubozu — "no wicks" — precisely because its wicks were impossible**,
    /// and `session` called it a large body on a bar whose body is three times its own
    /// range.
    ///
    /// Refused rather than clamped. Flooring the wicks at zero would fabricate a shape
    /// the ticks never made, which is the fallback `CLAUDE.md` §4 bans — the same
    /// reason [`Corrupt::RangeOverflows`] refuses instead of saturating.
    PriceOutsideRange,
    /// This bar's timestamp is not strictly after the last accepted one.
    ///
    /// Refused because the session rollover keys on the IST day *changing*, not
    /// increasing. A receding timestamp therefore closes the books on a **later**
    /// session and installs it as "yesterday", so the CPR, the S1–S5 / R1–R5 ladder,
    /// the fifteen previous-day rungs and the five-session rungs are all computed
    /// from a session that has not happened yet. That is look-ahead, and §3 rule 7
    /// requires a mechanism rather than an assumption.
    ///
    /// The store guarantees monotonic append, but `crates/indicators` no longer
    /// depends on the store — and a live consumer holding a replayed or reordered
    /// feed does not control its ordering. So the guarantee has to live here.
    TimestampNotIncreasing,
    /// `volume` is negative. Zero is a real zero (§7); negative is corruption.
    ///
    /// Reached through the aggregate evaluator, which previously discarded it. Only the
    /// VWAP family can detect it — the other eight modules never read `volume` — and
    /// discarding it made a bar VWAP called corruption come back as a successful
    /// evaluation with twenty positions silently absent.
    NegativeVolume,
    /// The volume-weighted accumulator would leave the range this crate can prove safe.
    ///
    /// Unreachable on any price a market prints; reachable from a corrupt record. Same
    /// reason as [`Corrupt::NegativeVolume`] for why it now surfaces rather than being
    /// swallowed.
    AccumulatorTooLarge,
    /// A price is not strictly positive. An instrument this crate sweeps cannot
    /// trade at or below zero.
    ///
    /// # The all-zero bar the store ACCEPTS
    ///
    /// [`Corrupt::RangeOverflows`] records that D-0143 added
    /// `open|high|low|close >= 0` to `store::format::ohlc_is_sane`, which stops
    /// a negative bar at the write boundary. It does not stop a ZERO one, and
    /// that predicate's own doc says so in as many words: an all-zero record
    /// satisfies it. So `o = h = l = c = 0` reaches the sweep *through the
    /// store*, not merely from a caller that skipped it.
    ///
    /// Every ordering guard above passes it. `high < low` is `0 < 0`, false.
    /// The subtraction is `0 - 0`. Containment is four comparisons of zero
    /// against zero. It is a well-formed bar by every test this function had.
    ///
    /// # What it costs, measured
    ///
    /// One such bar placed at day 3 of a twenty-session fixture, swept at a
    /// support floor of 150 hits:
    ///
    /// | | clean | one zero bar |
    /// |---|---|---|
    /// | records refused | 0 | **0** |
    /// | `Outcome::is_complete()` | true | **true** |
    /// | masks differing | — | **1,467 of 5,624** |
    /// | one condition's support | 210 hits | 154 |
    /// | that condition's best-case P&L | 237,810 paisa | **174,394** |
    ///
    /// **A 26.7% divergence in reported profit, with a refusal count of zero
    /// and a complete outcome.** The contamination window measured at exactly
    /// five sessions, which is the `prev5` depth. The mechanism is that a zero
    /// low becomes the session extreme, so the Fibonacci ladder, CPR, ORB and
    /// every derived level are measured against a range of millions of paisa
    /// instead of about fourteen hundred — and `close_the_books` carries it
    /// forward into `yesterday` and `prev5`.
    ///
    /// That is the failure `CLAUDE.md` §4 bans by name: not a crash, not a
    /// refusal, but a wrong number wearing a complete run's clothes.
    ///
    /// # Not currently live, and the guard is the point
    ///
    /// All 4,887,858 bars in the 2,707 files of the operator's store were
    /// scanned: zero all-zero records, zero negative, zero ordering breaks. The
    /// defect was the missing guard rather than the data — and a guarantee that
    /// holds only because nobody has yet written such a record is the kind
    /// [`Corrupt::RangeOverflows`] already refuses to rely on.
    ///
    /// All four fields are tested, not `low` alone, for the reason
    /// `ohlc_is_sane` gives for the same choice: this predicate must not depend
    /// on another clause of itself being true, or a later edit to the ordering
    /// guards silently widens what a price may be.
    PriceNotPositive,
}

/// The current-session Fibonacci anchors, and the eleven bits they decide.
///
/// Fixed size, no allocation, and it does not grow with the number of bars seen —
/// which is the whole of the constant-space argument for this family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurDayFib {
    session_day: i64,
    hi: i64,
    lo: i64,
    leg: Leg,
    bars_folded: u32,
    live: bool,
}

const _: () = assert!(core::mem::size_of::<CurDayFib>() <= 48);

impl Default for CurDayFib {
    fn default() -> Self {
        Self::new()
    }
}

impl CurDayFib {
    /// An empty state, before any session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_day: i64::MIN,
            hi: 0,
            lo: 0,
            leg: Leg::Undetermined,
            bars_folded: 0,
            live: false,
        }
    }

    /// Which extreme is the later one.
    #[must_use]
    pub const fn leg(&self) -> Leg {
        self.leg
    }

    /// The IST day this state is accumulating, or `None` before the first bar.
    #[must_use]
    pub const fn session_day(&self) -> Option<i64> {
        if self.live {
            Some(self.session_day)
        } else {
            None
        }
    }

    /// How many bars have been folded into the current session.
    #[must_use]
    pub const fn bars_folded(&self) -> u32 {
        self.bars_folded
    }

    /// The session range so far, or 0 when no range has formed.
    ///
    /// `checked_sub`, not `-`. `hi - lo` overflows for a corrupt record carrying
    /// `i64::MIN`, and in a debug build that is a panic on the evaluation path.
    /// [`Self::step`] refuses such a bar before it can reach here; this returns 0
    /// so a caller that reads the range directly gets "no range" rather than a
    /// crash, and 0 already means every bit is false.
    #[must_use]
    pub const fn range(&self) -> i64 {
        if self.live {
            match self.hi.checked_sub(self.lo) {
                Some(r) => r,
                None => 0,
            }
        } else {
            0
        }
    }

    /// **The full per-bar step, in the one order that is correct: reset (if the
    /// session changed) → emit → fold.**
    ///
    /// The emit reads state built from bars `s..=N-1` only; bar *N* is folded
    /// after. The three steps are fused into one method precisely so a caller
    /// cannot transpose them — see rule 2 in the module documentation for what
    /// folding first would cost.
    ///
    /// # Errors
    ///
    /// [`Corrupt::HighBelowLow`] for a record whose high is below its low. The
    /// bar is neither emitted for nor folded in.
    pub fn step(&mut self, bar: &Candle, tolerance: Tolerance) -> Result<ConditionMask, Corrupt> {
        // `check_evaluable`, not `check`: this module must refuse exactly what
        // `Evaluator::stepped` refuses or the mask is a mixture of two answers.
        bar.check_evaluable()?;
        let day = ist_day(bar.ts_micros);
        if day != self.session_day {
            *self = Self {
                session_day: day,
                ..Self::new()
            };
        }
        let bits = self.bits(bar.close, tolerance);
        self.fold(bar);
        Ok(bits)
    }

    /// Fold one bar into the anchors. Strict `>` and `<` are load-bearing.
    ///
    /// Under `>=`, a market at one price merely *re-touching* the session high would move
    /// the later extreme and flip the leg — remapping all eleven rungs with no
    /// change to the range at all. Under `>`, the leg changes if and only if the
    /// range changes.
    fn fold(&mut self, bar: &Candle) {
        if !self.live {
            self.hi = bar.high;
            self.lo = bar.low;
            self.live = true;
            self.bars_folded = 1;
            self.leg = Leg::Undetermined;
            return;
        }
        let new_high = bar.high > self.hi;
        let new_low = bar.low < self.lo;
        if new_high {
            self.hi = bar.high;
        }
        if new_low {
            self.lo = bar.low;
        }
        self.leg = match (new_high, new_low) {
            (true, true) => Leg::Undetermined,
            (true, false) => Leg::Up,
            (false, true) => Leg::Down,
            (false, false) => self.leg,
        };
        self.bars_folded = self.bars_folded.saturating_add(1);
    }

    /// The level of rung `p/1000`, in paisa, or `None` when there is no ladder.
    ///
    /// Materialising a level needs a division, so this is for **display and
    /// tests only** and is never on the evaluation path. [`Self::bits`] uses the
    /// cross-multiplied form instead, which has no rounding at all.
    #[must_use]
    pub fn level(&self, p: i32) -> Option<i64> {
        let r = self.range();
        if !self.live || r <= 0 {
            return None;
        }
        let step = i128::from(p) * i128::from(r) / 1000;
        let level = match self.leg {
            Leg::Undetermined => return None,
            Leg::Up => i128::from(self.hi) - step,
            Leg::Down => i128::from(self.lo) + step,
        };
        i64::try_from(level).ok()
    }

    /// The eleven-rung ladder as vocabulary positions 121–131, set on a mask.
    ///
    /// Delegates the band test to `vocab::table::set_near`, so the position
    /// numbers, the `near_*` kind check and the tolerance all stay owned by the
    /// crate that defines them. A wrong index here is a `VocabError`, not a
    /// silently mis-set bit.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        let r = self.range();
        if !self.live || r <= 0 {
            return mask;
        }
        // ONE test for an undetermined leg, and it is this match. The guard above
        // used to carry `self.leg == Leg::Undetermined` as well, which made the
        // third arm below unreachable while the guard was correct — a region no
        // input could execute, and therefore one no test could cover. Deleting the
        // duplicate rather than the arm keeps every leg named explicitly at the
        // one place the anchor is decided; the returned mask is empty either way,
        // so nothing a caller can observe changed.
        let anchor = match self.leg {
            Leg::Up => self.hi,
            Leg::Down => self.lo,
            Leg::Undetermined => return mask,
        };
        let mut i = 0;
        while i < CURDAY_RUNGS.len() {
            #[expect(
                clippy::indexing_slicing,
                reason = "i < CURDAY_RUNGS.len() is the loop condition"
            )]
            let p = CURDAY_RUNGS[i];
            let index = CURDAY_FIRST + u16::try_from(i).unwrap_or(u16::MAX);
            // ONE level, ONE test. `set_near` re-tests with exactly the
            // `tolerance.covers(close, level, r)` used on the line above, so the two
            // cannot disagree. A refusal from `set_near` means the table and
            // CURDAY_FIRST disagree, which `the_rungs_match_the_vocabulary` makes a
            // test failure rather than a runtime surprise.
            if let Some(level) = self.rung_level(anchor, p, r)
                && tolerance.covers(close, level, r)
                && let Ok(next) = vocab::table::set_near(mask, index, tolerance, close, level, r)
            {
                mask = next;
            }
            i += 1;
        }
        mask
    }

    /// The rung's price level, materialised exactly once.
    ///
    /// `None` while the leg is undetermined, or if the level leaves `i64`.
    ///
    /// # Two bugs lived here, and the second was the serious one
    ///
    /// This replaces a pair of functions: an exact cross-multiplied test that
    /// decided whether the bit belonged, and a separate materialisation that handed
    /// `vocab` a level — under a comment claiming "a rounding difference here cannot
    /// change which bit is set".
    ///
    /// It could. `vocab::table::set_near` re-tests
    /// `|close − level| · 1000 ≤ tol · range` against the **truncated** level, while
    /// the exact test compared against the unrounded one, so at the band edge the two
    /// disagreed and the bit was dropped. The caller cannot tell "not near" from
    /// "set", because `set_near` returns the mask unchanged either way.
    ///
    /// Worse: the materialisation computed `anchor − step` for **both** legs, while
    /// the exact test implies `anchor + step` on a Down leg. Measured at
    /// `anchor = 2_500_000`, `range = 100_000`, `p = 618`: the exact test accepts a
    /// close of `2_561_800`, the level handed to `vocab` was `2_438_200`, and
    /// `covers` then compared `123_600_000` against a band of `1_000_000` — **123×
    /// over**. Every Down-leg rung with `p != 0` was silently dropped.
    ///
    /// One level and one test cannot disagree, which is why this returns the level
    /// rather than a verdict.
    fn rung_level(&self, anchor: i64, p: i32, r: i64) -> Option<i64> {
        let step = (i128::from(p) * i128::from(r)).div_euclid(1000);
        let a = i128::from(anchor);
        let level = match self.leg {
            Leg::Up => a - step,
            Leg::Down => a + step,
            Leg::Undetermined => return None,
        };
        // `ok()` and not `unwrap_or(i64::MIN)`: §7 reserves `i64::MIN` as the
        // open-interest null sentinel, and clamping an out-of-range level onto it
        // would put a sentinel where a price goes.
        i64::try_from(level).ok()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test that \
              cannot panic cannot fail — and here it buys a second thing. \
              `unreachable!` expands to a panic inside THIS crate, so llvm-cov \
              counts a region that can never run and the coverage gate can never \
              reach 100 on this file. `.expect` panics inside core, which is not \
              instrumented: the same failure, with the refused value printed \
              beside the message, and no dead region left behind."
)]
mod tests {
    use super::*;

    fn bar(ts: i64, o: i64, h: i64, l: i64, c: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the fib width is pinned")
    }

    /// Step a bar that must be legal. A helper so that the twenty call sites below
    /// read as one line each, and so that a fixture this crate calls sane failing
    /// the sanity check names itself once rather than twenty times.
    fn ok(s: &mut CurDayFib, b: &Candle) -> ConditionMask {
        s.step(b, tol()).expect("this fixture bar is sane")
    }

    /// A bar that only RE-TOUCHES an extreme does not move the leg.
    ///
    /// `fold`'s comparisons are strict on both sides, and its doc calls that load-bearing.
    /// Relaxing either to `>=` survived the whole suite: no fixture ever re-touched an
    /// extreme without also widening the range. A bar that merely equals the running high
    /// then flips `Leg`, which remaps all eleven rungs of the current-day ladder.
    #[test]
    fn a_bar_that_only_retouches_an_extreme_does_not_move_the_leg() {
        let mut s = CurDayFib::default();
        // Bar 0 establishes the range through the reset branch.
        let _ = ok(&mut s, &bar(0, 2_500_000, 2_510_000, 2_490_000, 2_500_000));
        let before = s.leg();

        // Bar 1 touches the high again -- exactly, not past it -- and its low is strictly
        // inside. Neither extreme moves, so neither does the leg.
        let _ = ok(
            &mut s,
            &bar(60_000_000, 2_500_000, 2_510_000, 2_495_000, 2_500_000),
        );
        assert_eq!(
            s.leg(),
            before,
            "a bar that equalled the running high and stayed inside the low moved the leg. \
             `>` became `>=`, and all eleven rungs are now measured from the wrong end."
        );

        // And the LOW side, which needs its own bar. The bar above has a low strictly
        // INSIDE the range, so `bar.low < self.lo` and `bar.low <= self.lo` agree on it and
        // the `<=` mutation survives -- it did, on the first run of this test. This bar
        // equals the running low exactly, which is the only input that separates them.
        let _ = ok(
            &mut s,
            &bar(120_000_000, 2_500_000, 2_505_000, 2_490_000, 2_500_000),
        );
        assert_eq!(
            s.leg(),
            before,
            "a bar that equalled the running LOW and stayed inside the high moved the leg. \
             `<` became `<=`, and the ladder is measured from the wrong end."
        );

        // The strictly-higher case still moves it, or a fold that never updated would
        // satisfy every assertion above.
        let _ = ok(
            &mut s,
            &bar(180_000_000, 2_500_000, 2_512_000, 2_495_000, 2_500_000),
        );
        assert_eq!(
            s.leg(),
            Leg::Up,
            "a bar that genuinely made a new high must set the leg up"
        );
    }

    /// `ist_day` is MONOTONE across the whole `i64` range, including its ends.
    ///
    /// The rollover in `Evaluator::stepped` fires on `today != self.day`, so the one
    /// property it depends on is that a later timestamp never yields a smaller day. With
    /// `wrapping_add` it did: `ist_day(i64::MAX)` returned **`-106_751_991`** where the true
    /// day is +`106_751_991`, so a stamp within 5h30m of the type's ceiling fabricated a
    /// session boundary. `saturating_add` keeps the order.
    ///
    /// The saturated answer is not a real date and cannot be -- that stamp is the year
    /// `292_277_026_596` -- so the choice is between a monotone lie and a non-monotone one,
    /// and only the monotone one leaves the rollover sound.
    #[test]
    fn the_ist_day_never_goes_backwards_as_the_timestamp_goes_forwards() {
        let probes = [
            i64::MIN,
            i64::MIN + 1,
            -MICROS_PER_DAY,
            -19_800_000_001,
            -19_800_000_000,
            0,
            MICROS_PER_DAY,
            i64::MAX - MICROS_PER_DAY,
            i64::MAX - IST_OFFSET_MICROS,
            i64::MAX - 1,
            i64::MAX,
        ];
        let mut previous = ist_day(probes[0]);
        for ts in probes {
            let day = ist_day(ts);
            assert!(
                day >= previous,
                "ist_day({ts}) is {day}, below {previous} from an EARLIER stamp. A later \
                 timestamp yielding a smaller day makes the evaluator's rollover see a \
                 session boundary that is not there."
            );
            previous = day;
        }
        // BOUND FIRST, and not for tidiness. An argument in an assert's format list is
        // evaluated only when the assertion FAILS, so while the code is correct it is a
        // region no run can enter and llvm-cov reports the line uncovered forever.
        // `docs/06-limits.md` §55 records the same defect leaking into `pattern.rs` from
        // the coverage work itself, and the same repair -- hoist the value to a local.
        // It also removes a second call that could, in principle, answer differently
        // from the one that was tested.
        let at_ceiling = ist_day(i64::MAX);
        assert!(
            at_ceiling > 0,
            "ist_day(i64::MAX) is {at_ceiling}, and a positive stamp cannot be a negative day"
        );
    }

    /// A pre-epoch timestamp gets its own session, because `ist_day` floors.
    ///
    /// `ist_day` uses `div_euclid`, and its own doc names the hazard: truncating division
    /// "rounds up and silently merges that bar into the 1970-01-01 session". No test fed a
    /// pre-1970 bar, so swapping `div_euclid` for `/` survived -- and the audit measured
    /// the consequence: IST day -1 and IST day 0 merge into one session, dropping
    /// `sessions_completed` from 1 to 0 and `has_yesterday` from true to false.
    ///
    /// The witness is one microsecond wide. `-19_800_000_001` is the last microsecond
    /// before the IST epoch day begins; `-19_800_000_000` is its first.
    #[test]
    fn a_pre_epoch_timestamp_floors_into_its_own_session() {
        assert_eq!(
            ist_day(-19_800_000_001),
            -1,
            "one microsecond before the IST epoch day belongs to the day before it. \
             Truncating division returns 0 here and merges two sessions into one."
        );
        assert_eq!(
            ist_day(-19_800_000_000),
            0,
            "and the first microsecond of the IST epoch day is day 0"
        );
        assert_eq!(
            ist_day(-19_800_000_001 - MICROS_PER_DAY),
            -2,
            "the floor keeps going down rather than collapsing toward zero"
        );
    }

    /// The rung order here MUST be the table's order, or every emitted bit means
    /// the wrong rung. Checked against `vocab`'s own names, not against a comment.
    #[test]
    fn the_rungs_match_the_vocabulary() {
        // The suffix is the TABLE's spelling and this crate does not get to
        // derive it. Written out rather than computed: 500 is spelled `50` and
        // 1000 is spelled `100`, so any rule clever enough to produce both is
        // clever enough to be wrong silently.
        const EXPECTED: [(i32, &str); 11] = [
            (0, "near_fib_curday_0"),
            (236, "near_fib_curday_236"),
            (382, "near_fib_curday_382"),
            (500, "near_fib_curday_50"),
            (618, "near_fib_curday_618"),
            (786, "near_fib_curday_786"),
            (1000, "near_fib_curday_100"),
            (1272, "near_fib_curday_1272"),
            (1618, "near_fib_curday_1618"),
            (2000, "near_fib_curday_200"),
            (2618, "near_fib_curday_2618"),
        ];
        for (i, (p, want)) in EXPECTED.iter().enumerate() {
            let offset = u16::try_from(i).expect("eleven fits in a u16");
            let index = CURDAY_FIRST + offset;
            let def = vocab::table::definition(index).expect("121..=131 are allocated");
            assert_eq!(
                CURDAY_RUNGS.get(i).copied(),
                Some(*p),
                "this crate's rung {i} is not {p}",
            );
            assert_eq!(
                def.name, *want,
                "rung {p} at position {index} is `{}` in the table",
                def.name,
            );
        }
    }

    /// `div_euclid` and not `/`: a pre-epoch stamp must not land on 1970-01-01.
    #[test]
    fn the_session_day_floors_for_negative_stamps() {
        assert_eq!(ist_day(0), 0, "epoch is inside IST day 0");
        assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
        assert!(
            ist_day(-MICROS_PER_DAY * 3) < 0,
            "three days before the epoch is a negative day number"
        );
        // Monotone, which truncating division would break across zero.
        let mut previous = ist_day(-MICROS_PER_DAY * 5);
        for k in -4..5 {
            let d = ist_day(MICROS_PER_DAY * k);
            assert!(d >= previous, "day numbers went backwards at k = {k}");
            previous = d;
        }
    }

    /// The first bar of a session emits nothing: there is no prior range to
    /// measure against, and a bit that cannot be evaluated is false.
    #[test]
    fn the_first_bar_of_a_session_emits_nothing() {
        let mut s = CurDayFib::new();
        let m = s
            .step(&bar(0, 100, 110, 90, 105), tol())
            .expect("a sane bar");
        assert!(
            m.is_empty(),
            "the first bar had no anchor and still emitted"
        );
        assert_eq!(s.bars_folded(), 1);
        assert_eq!(s.leg(), Leg::Undetermined);
    }

    /// A corrupt record is refused, and it is neither emitted for nor folded in.
    #[test]
    fn a_high_below_its_low_is_refused_and_changes_nothing() {
        let mut s = CurDayFib::new();
        let before = s;
        assert_eq!(
            s.step(&bar(0, 100, 90, 110, 105), tol()),
            Err(Corrupt::HighBelowLow)
        );
        assert_eq!(s, before, "a refused bar still changed the state");
    }

    /// Zero range emits nothing and never divides.
    #[test]
    fn a_flat_session_emits_nothing_all_day() {
        let mut s = CurDayFib::new();
        for k in 0..20 {
            let m = s
                .step(&bar(k * 60_000_000, 100, 100, 100, 100), tol())
                .unwrap_or(ConditionMask::ZERO);
            assert!(m.is_empty(), "bar {k} of a flat session emitted a bit");
        }
        assert_eq!(s.range(), 0);
    }

    /// A later high alone makes the high the later extreme.
    #[test]
    fn a_new_high_alone_is_an_up_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 120, 95, 115));
        assert_eq!(s.leg(), Leg::Up);
    }

    /// A later low alone makes the low the later extreme.
    #[test]
    fn a_new_low_alone_is_a_down_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 108, 80, 85));
        assert_eq!(s.leg(), Leg::Down);
    }

    /// One bar engulfing the whole running range leaves no temporal order.
    #[test]
    fn one_bar_engulfing_the_range_is_undetermined() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 130, 70, 100));
        assert_eq!(s.leg(), Leg::Undetermined);
        let m = s.bits(100, tol());
        assert!(m.is_empty(), "an undetermined leg still emitted");
    }

    /// Re-touching the session high must NOT flip the leg: strict `>` is what
    /// makes the ladder a function of the range's history rather than of how many
    /// times a level was tapped.
    #[test]
    fn re_touching_the_high_does_not_flip_the_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 120, 95, 115));
        assert_eq!(s.leg(), Leg::Up);
        let range_before = s.range();
        // high EQUALS the running high, so nothing moved.
        let _ = ok(&mut s, &bar(120_000_000, 115, 120, 100, 118));
        assert_eq!(s.leg(), Leg::Up, "a re-touch flipped the leg");
        assert_eq!(s.range(), range_before, "a re-touch changed the range");
    }

    /// The session resets on the IST day boundary and yesterday's extremes go.
    #[test]
    fn a_new_ist_day_discards_the_previous_session() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 500, 50, 200));
        let day_one = s.session_day().expect("a folded bar leaves the state live");
        let _ = ok(&mut s, &bar(MICROS_PER_DAY, 100, 110, 90, 105));
        let day_two = s.session_day().expect("a folded bar leaves the state live");
        assert_ne!(day_two, day_one);
        assert_eq!(s.bars_folded(), 1, "the new session kept old bars");
        assert_eq!(s.range(), 20, "the new session kept the old range");
    }

    /// Same bars twice, byte-identical bit streams. §3 rule 5.
    #[test]
    fn the_same_session_twice_gives_the_same_bits() {
        let bars: Vec<Candle> = (0..200)
            .map(|k| {
                let base = 2_500_000 + (k * 37) % 900;
                bar(
                    k * 60_000_000,
                    base,
                    base + 40,
                    base - 40,
                    base + ((k * 11) % 61) - 30,
                )
            })
            .collect();
        let run = || {
            let mut s = CurDayFib::new();
            let mut out = Vec::new();
            for b in &bars {
                out.push(ok(&mut s, b).words());
            }
            out
        };
        assert_eq!(run(), run(), "two identical runs disagreed");
        // THE SEVENTH SITE, AND THE AUDIT THAT FIXED THE OTHERS SAID "ALL SIX".
        //
        // The equality above holds for ANY deterministic body -- including
        // `bits() -> ConditionMask::ZERO` and any constant -- so it cannot see
        // the one mutation that matters. That was found once and repaired in
        // `daily`, `orb`, `fib`, `session`, `vwap` and `pattern`, each of which
        // carries this guard and a comment counting SIX. There are seven:
        // `assert_eq!(run(), run()` appears once more, here, and this is the
        // module the census missed -- the other six live in named modules and
        // `CurDayFib` sits in `lib.rs`.
        //
        // Measured: `grep -c "later != earlier"` answered 1 in each of the six
        // and 0 here.
        //
        // This closes it without needing to know what the bits SHOULD be, which
        // is what the sibling tests are for. A body that ignores its input emits
        // the same value on every bar, and the fixture above deliberately varies
        // them by `(k * 11) % 61`. So: the run must not be constant.
        let observed = run();
        assert!(
            observed
                .iter()
                .skip(1)
                .zip(observed.iter())
                .any(|(later, earlier)| later != earlier),
            "every bar produced the same words, so a body ignoring its input \
             would pass the equality above"
        );
    }

    /// Every bit this crate sets is a current-day rung and nothing else. A stray
    /// position would be a mask that means something the caller never asked for.
    #[test]
    fn only_positions_121_to_131_are_ever_set() {
        let mut s = CurDayFib::new();
        let mut union = ConditionMask::ZERO;
        for k in 0..400 {
            let base = 2_500_000 + (k * 53) % 1_500;
            let b = bar(
                k * 60_000_000,
                base,
                base + 60,
                base - 60,
                base + ((k * 17) % 121) - 60,
            );
            union = union.union(&ok(&mut s, &b));
        }
        for index in 0..vocab::ConditionMask::BITS {
            if union.get(index) {
                assert!(
                    (u32::from(CURDAY_FIRST)..u32::from(CURDAY_FIRST) + 11).contains(&index),
                    "position {index} was set and is not a current-day rung",
                );
            }
        }
    }

    /// At most one rung can fire on a bar. The proof is arithmetic — two rungs
    /// need a numerator gap of at most twice the width, and the ladder's smallest
    /// gap is 118 against a width of 10 — and this is the check on the proof.
    #[test]
    fn at_most_one_rung_fires_on_any_bar() {
        let mut s = CurDayFib::new();
        for k in 0..600 {
            let base = 2_500_000 + (k * 71) % 2_000;
            let b = bar(
                k * 60_000_000,
                base,
                base + 80,
                base - 80,
                base + ((k * 23) % 161) - 80,
            );
            let m = ok(&mut s, &b);
            // `popcount` once, into a name the message interpolates. As a second call
            // in the message arguments it was a region evaluated ONLY on failure, and a
            // region that cannot run while the test passes is one no coverage run can
            // ever reach. The bound and the failure text are unchanged.
            let lit = m.popcount();
            assert!(lit <= 1, "bar {k} lit {lit} rungs at once");
        }
    }

    /// A RUNG FIRES, WHICH IS THE ONE THING NEITHER TEST ABOVE ASKS FOR.
    ///
    /// `only_positions_121_to_131_are_ever_set` and `at_most_one_rung_fires_on_any_bar`
    /// carry real assertions, and **an empty mask satisfies both of them**: "no stray
    /// position is set" and "at most one is set" are vacuously true of nothing. A
    /// four-hundred-bar walk and a six-hundred-bar walk both passed without the ladder
    /// ever lighting. That is the §4 shape — a test that asserts nothing — wearing two
    /// assertions, and it is what let `bits` be replaced wholesale by
    /// `Default::default()` with the suite green, along with the loop bound, the
    /// liveness guard and the index arithmetic that reaches the vocabulary.
    ///
    /// So this names a level and asks for the bit at it. The ladder is built to be read
    /// off by hand: a low of `2_400_000` and a LATER high of `2_500_000` leave `Leg::Up`
    /// with a range of `100_000`, so rung `p` sits at `2_500_000 - p * 100` and the band
    /// is `TOL_FIB_MILLI * range / 1000` = `1_000` paisa either side.
    ///
    /// The emit bar is FLAT at its close. `step` emits before it folds, so the bar's
    /// own shape cannot reach the ladder it is measured against — and a flat bar says
    /// that in the fixture rather than in a comment.
    #[test]
    fn a_close_on_a_rung_lights_that_rung_and_nothing_else() {
        // Bar 0 opens the session: hi 2_450_000, lo 2_400_000, leg Undetermined.
        // Bar 1 makes a new high and NO new low, which is the only route to `Leg::Up`.
        let ladder = || {
            let mut s = CurDayFib::new();
            let _ = ok(&mut s, &bar(0, 2_420_000, 2_450_000, 2_400_000, 2_440_000));
            let _ = ok(
                &mut s,
                &bar(60_000_000, 2_440_000, 2_500_000, 2_420_000, 2_490_000),
            );
            s
        };
        let mut s = ladder();
        assert_eq!(s.leg(), Leg::Up, "the later extreme was not the high");
        assert_eq!(s.range(), 100_000, "the session range is not hi - lo");

        // Rung 618 of an Up leg retraces DOWN from the high: 2_500_000 - 61_800.
        let level = 2_438_200;
        assert_eq!(
            s.level(618),
            Some(level),
            "rung 618 is not where this test reads it"
        );
        let emit = |s: &mut CurDayFib, close: i64| -> ConditionMask {
            ok(s, &bar(120_000_000, close, close, close, close))
        };
        let rung_618 = u32::from(CURDAY_FIRST) + 4;
        let m = emit(&mut s, level);
        assert!(
            m.get(rung_618),
            "a close exactly on rung 618 did not light position 125"
        );
        assert_eq!(
            m.popcount(),
            1,
            "one close on one rung lit more than one rung"
        );

        // The band's far edge, and one paisa past it.
        assert!(
            emit(&mut ladder(), level - 1_000).get(rung_618),
            "a close on the band's own edge is covered; `<=` must not become `<`"
        );
        assert!(
            !emit(&mut ladder(), level - 1_001).get(rung_618),
            "a close one paisa outside the band lit the rung anyway"
        );

        // And the anchor itself is rung 0, at the other end of the same ladder.
        assert!(
            emit(&mut ladder(), 2_500_000).get(u32::from(CURDAY_FIRST)),
            "a close at the session high did not light rung 0"
        );
    }

    /// The barrier cannot name a bar past `n`, and refuses an out-of-range `n`
    /// rather than clamping.
    #[test]
    fn the_prefix_cannot_see_past_its_own_end() {
        let bars: Vec<Candle> = (0..5).map(|k| bar(k, 1, 2, 0, 1)).collect();
        let p = PastPrefix::upto(&bars, 2).expect("2 is in range");
        assert_eq!(p.len(), 3);
        let cur = p.current().expect("the prefix is non-empty");
        assert_eq!(cur.ts_micros, 2);
        assert_eq!(p.as_slice().len(), 3, "the slice reached past n");
        assert!(
            PastPrefix::upto(&bars, 5).is_none(),
            "n == len was accepted"
        );
        assert!(PastPrefix::upto(&bars, 99).is_none());
    }

    /// Prices at the edge of the type do not panic and do not overflow.
    #[test]
    fn extreme_prices_neither_panic_nor_overflow() {
        let mut s = CurDayFib::new();
        // Field order is sane, and D-0143 means `ohlc_is_sane` would now REFUSE
        // this bar anyway, for the `i64::MIN` low. That is not why this test
        // exists and it does not weaken it: this crate is handed bars by
        // callers that never crossed `BarFile::append`, so the range guard must
        // hold on its own. The bar must be REFUSED here, not saturated.
        //
        // The comment used to say `ohlc_is_sane` "would pass it", which stopped
        // being true the day the sign check went in.
        assert_eq!(
            s.step(&bar(0, 0, i64::MAX, i64::MIN, 0), tol()),
            Err(Corrupt::RangeOverflows),
        );
        assert_eq!(s.bars_folded(), 0, "a refused bar was folded in");

        // A wide but representable range must still work, at either price edge.
        //
        // OPEN AND CLOSE ARE `-+ i64::MAX / 8` AND WERE BOTH `0`, which this test
        // never meant. The wide HIGH and LOW are what it is about, and they are
        // unchanged; the zeros were there only because nothing had cause to
        // object to one. `Candle::check_evaluable` now refuses a zero price, so
        // the fixture names an extreme open and close inside the same range
        // rather than an absent pair.
        let mut t = CurDayFib::new();
        let _ = ok(
            &mut t,
            &bar(
                0,
                -(i64::MAX / 8),
                i64::MAX / 4,
                -(i64::MAX / 4),
                i64::MAX / 8,
            ),
        );
        let _ = ok(
            &mut t,
            &bar(
                60_000_000,
                -(i64::MAX / 8),
                i64::MAX / 2,
                -(i64::MAX / 4),
                i64::MAX / 8,
            ),
        );
        let _ = t.bits(0, tol());
        let _ = t.bits(i64::MAX, tol());
        let _ = t.bits(i64::MIN, tol());
    }

    /// Before its first bar the state names no session, no range and no ladder.
    ///
    /// `session_day` is an `Option` for exactly this reason: the field holds
    /// `i64::MIN` until a bar arrives, so returning `Some(i64::MIN)` would hand a
    /// caller §7's null sentinel dressed as a day number. Nothing had ever called
    /// the accessor before the first bar, so nothing had ever checked which of the
    /// two it returns.
    #[test]
    fn a_state_before_its_first_bar_has_no_session_and_no_ladder() {
        let s = CurDayFib::new();
        assert_eq!(s.session_day(), None, "an empty state named a session day");
        assert_eq!(s.range(), 0, "an empty state reported a range");
        assert_eq!(s.bars_folded(), 0, "an empty state had folded a bar");
        assert_eq!(s.leg(), Leg::Undetermined, "an empty state picked a leg");
        assert_eq!(s.level(0), None, "an empty state materialised a level");
        assert!(
            s.bits(2_500_000, tol()).is_empty(),
            "an empty state emitted a rung"
        );
    }

    /// `Default` must be `new`, or a caller writing `CurDayFib::default()` starts a
    /// run in a state nothing in this file reasons about.
    #[test]
    fn the_default_state_is_the_empty_one() {
        assert_eq!(
            CurDayFib::default(),
            CurDayFib::new(),
            "`Default` drifted from `new`"
        );
    }

    /// A span wider than `i64` reports "no range" rather than panicking.
    ///
    /// `step` refuses such a record — `extreme_prices_neither_panic_nor_overflow`
    /// proves that — so this state is reachable only from inside the crate, which is
    /// exactly the reader `range`'s `checked_sub` is there for. Written as `hi - lo`
    /// it is a panic in **every** profile, because this workspace leaves overflow
    /// checks on in release too.
    #[test]
    fn a_span_wider_than_the_type_reports_no_range_rather_than_panicking() {
        let s = CurDayFib {
            hi: i64::MAX,
            lo: i64::MIN,
            leg: Leg::Up,
            live: true,
            ..CurDayFib::new()
        };
        assert_eq!(s.range(), 0, "hi - lo overflowed and was not caught");
        assert_eq!(
            s.level(618),
            None,
            "a state with no range still materialised a level"
        );
        assert!(
            s.bits(0, tol()).is_empty(),
            "a state with no range emitted a rung"
        );
    }

    /// The displayed level is the level the bit is tested against, on both legs.
    ///
    /// `level` is the display path and `rung_level` is the evaluation path, and two
    /// bugs already lived in that pair — see `rung_level`'s documentation: a
    /// truncation that disagreed at the band edge, and a sign that put every
    /// Down-leg rung on the wrong side of the anchor. `level` had no caller at all,
    /// in this crate or any other. The hand-computed numbers come first so this is
    /// not two implementations agreeing with each other and nothing else: over a
    /// 100,000-paisa session, rung 618 sits exactly 61,800 paisa from the anchor.
    #[test]
    fn the_displayed_level_is_the_level_the_rung_is_tested_against() {
        // Bar two takes the high alone, so the high is the later extreme.
        let mut up = CurDayFib::new();
        let _ = ok(&mut up, &bar(0, 2_450_000, 2_490_000, 2_400_000, 2_450_000));
        let _ = ok(
            &mut up,
            &bar(60_000_000, 2_450_000, 2_500_000, 2_410_000, 2_460_000),
        );
        assert_eq!(up.leg(), Leg::Up, "a new high alone is an Up leg");
        assert_eq!(
            up.range(),
            100_000,
            "the session spans 2,400,000..2,500,000"
        );
        assert_eq!(up.level(0), Some(2_500_000), "rung 0 is the anchor itself");
        assert_eq!(
            up.level(618),
            Some(2_438_200),
            "an Up-leg rung retraces DOWN from the high"
        );
        assert_eq!(
            up.level(1000),
            Some(2_400_000),
            "rung 1000 is the far extreme"
        );

        // Bar two takes the low alone, so the low is the later extreme.
        let mut down = CurDayFib::new();
        let _ = ok(
            &mut down,
            &bar(0, 2_450_000, 2_500_000, 2_410_000, 2_450_000),
        );
        let _ = ok(
            &mut down,
            &bar(60_000_000, 2_450_000, 2_490_000, 2_400_000, 2_420_000),
        );
        assert_eq!(down.leg(), Leg::Down, "a new low alone is a Down leg");
        assert_eq!(
            down.range(),
            100_000,
            "the session spans 2,400,000..2,500,000"
        );
        assert_eq!(
            down.level(0),
            Some(2_400_000),
            "rung 0 is the anchor itself"
        );
        assert_eq!(
            down.level(618),
            Some(2_461_800),
            "a Down-leg rung retraces UP from the low"
        );
        assert_eq!(
            down.level(1000),
            Some(2_500_000),
            "rung 1000 is the far extreme"
        );

        // And every rung of the ladder displays what it is tested against. This is
        // the disagreement that silently dropped every Down-leg bit.
        for (state, anchor, name) in [(up, 2_500_000_i64, "Up"), (down, 2_400_000_i64, "Down")] {
            for p in CURDAY_RUNGS {
                assert_eq!(
                    state.level(p),
                    state.rung_level(anchor, p, 100_000),
                    "{name} leg: rung {p} displays one level and is tested against another"
                );
            }
        }
    }

    /// An undetermined leg has a range but no ladder, and emits nothing.
    ///
    /// A bar that engulfs the running range moves both extremes, so they share one
    /// bar and no temporal order exists. `bits` decides that in the single match
    /// that picks the anchor, and this is the input that reaches it: live, with a
    /// real range, and with no later extreme. Without such an input that arm is a
    /// region no test executes.
    #[test]
    fn an_undetermined_leg_has_a_range_but_no_level() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 2_450_000, 2_490_000, 2_400_000, 2_450_000));
        let _ = ok(
            &mut s,
            &bar(60_000_000, 2_450_000, 2_600_000, 2_300_000, 2_450_000),
        );
        assert_eq!(
            s.leg(),
            Leg::Undetermined,
            "an engulfing bar left a temporal order behind"
        );
        assert_eq!(
            s.range(),
            300_000,
            "the engulfing bar's own span is the session range"
        );
        assert_eq!(
            s.level(618),
            None,
            "an undetermined leg materialised a level"
        );
        assert!(
            s.bits(2_450_000, tol()).is_empty(),
            "an undetermined leg emitted a rung"
        );
    }

    /// A level outside `i64` is refused by the display path too, not clamped.
    ///
    /// `an_out_of_range_level_is_refused_not_clamped` proves it for the evaluation
    /// path, and both must refuse for the same reason: clamping would put §7's
    /// open-interest null sentinel where a price goes. Rung 0 of the same state is
    /// still representable, which is what makes this the refusal of one rung rather
    /// than of the whole ladder.
    #[test]
    fn a_displayed_level_outside_the_type_is_refused_not_clamped() {
        let s = CurDayFib {
            hi: i64::MAX,
            lo: i64::MAX / 2,
            leg: Leg::Down,
            live: true,
            ..CurDayFib::new()
        };
        assert_eq!(
            s.level(0),
            Some(i64::MAX / 2),
            "rung 0 is the anchor and fits"
        );
        assert_eq!(
            s.level(2618),
            None,
            "a level 2.618 half-i64 ranges above the anchor was returned anyway"
        );
    }

    /// `is_empty` is the accessor clippy demands beside `len`, and nothing called it.
    ///
    /// The answer it must give is not obvious from the type: an empty prefix is not
    /// constructible at all, because `upto` on an empty history is `None` rather
    /// than a prefix of nothing. Both halves are asserted, because a prefix that
    /// reported itself empty would make a caller skip the bar it does hold.
    #[test]
    fn a_prefix_always_holds_at_least_the_bar_it_names() {
        let bars: Vec<Candle> = (0..3).map(|k| bar(k, 1, 2, 0, 1)).collect();
        let p = PastPrefix::upto(&bars, 0).expect("bar 0 is in range");
        assert_eq!(p.len(), 1, "the prefix at bar 0 saw more than bar 0");
        assert!(
            !p.is_empty(),
            "a prefix holding bar 0 reported itself empty"
        );
        let nothing: [Candle; 0] = [];
        assert!(
            PastPrefix::upto(&nothing, 0).is_none(),
            "bar 0 of an empty history is not a bar and must not be a prefix"
        );
    }

    /// The three legs are three distinct keys, and the default leg emits nothing.
    ///
    /// `Leg` is `Hash` so a caller can key results by leg, and a hash that
    /// disagreed with `Eq` would give it two entries for one leg. `Default` matters
    /// for the reason `CurDayFib::new` starts undetermined: a leg that defaulted to
    /// `Up` would emit a ladder off an anchor no bar established.
    #[test]
    fn the_three_legs_are_three_distinct_keys() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        assert!(seen.insert(Leg::Up), "an empty set already held Up");
        assert!(!seen.insert(Leg::Up), "one leg produced two entries");
        assert!(seen.insert(Leg::Down), "Down collided with Up");
        assert!(
            seen.insert(Leg::Undetermined),
            "Undetermined collided with a determined leg"
        );
        assert_eq!(seen.len(), 3, "the three legs did not stay three keys");
        assert_eq!(
            Leg::default(),
            Leg::Undetermined,
            "the default leg must be the one with no anchor"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "same exception, and the same second reason, as `mod tests` above: \
              `unreachable!` would leave a region in this crate that no input can \
              reach, and `.expect` leaves none."
)]
mod zero_range {
    use super::*;

    fn flat(ts: i64, price: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// `high == low` is a real bar and must be ACCEPTED, not refused.
    ///
    /// This test exists to kill a specific surviving mutant. `CurDayFib::step` and
    /// `SessionState::step` both guard with `if bar.high < bar.low`. Changing that
    /// `<` to `<=` refuses every zero-range bar, and the whole suite stayed green,
    /// because the only test that fed a zero-range bar swallowed the result with
    /// `unwrap_or` and never asserted which way it went.
    ///
    /// A zero-range minute is not pathological — it is what a limit-locked or
    /// untraded minute looks like, and refusing it would silently drop real bars
    /// from a run while reporting nothing.
    #[test]
    fn a_zero_range_bar_is_accepted_by_curday_fib() {
        let tolerance = vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid");
        let mut f = CurDayFib::new();
        assert!(
            f.step(&flat(0, 2_500_000), tolerance).is_ok(),
            "high == low is a real bar; `<` must not become `<=`"
        );
    }

    /// The guard still refuses a genuinely inverted bar.
    ///
    /// Without this half, the test above could be satisfied by deleting the guard.
    #[test]
    fn an_inverted_bar_is_still_refused_by_curday_fib() {
        let tolerance = vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid");
        let mut f = CurDayFib::new();
        let inverted = Candle {
            ts_micros: 0,
            open: 2_500_000,
            high: 2_499_000,
            low: 2_501_000,
            close: 2_500_000,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(f.step(&inverted, tolerance), Err(Corrupt::HighBelowLow));
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "same exception, and the same second reason, as `mod tests` above: \
              `unreachable!` would leave a region in this crate that no input can \
              reach, and `.expect` leaves none."
)]
mod down_leg {
    use super::*;

    /// A Down-leg rung fires. Before the level and the test were unified, none did.
    ///
    /// `set_curday` materialised `anchor - step` for both legs while the exact test
    /// implied `anchor + step` on a Down leg, so `vocab::table::set_near` was handed
    /// a level on the wrong side of the anchor and refused — silently, because it
    /// returns the mask unchanged whether it sets a bit or declines. Measured on the
    /// fixture below: the comparison was 123,600,000 against a band of 1,000,000.
    ///
    /// Both legs are asserted. Checking only the Down leg would pass if the sign
    /// were simply flipped the other way.
    #[test]
    fn both_legs_can_set_a_rung() {
        let tolerance = vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid");
        let (anchor, range, p) = (2_500_000_i64, 100_000_i64, 618_i32);
        let step = i64::from(p) * range / 1000;

        for (leg, expected) in [(Leg::Up, anchor - step), (Leg::Down, anchor + step)] {
            let f = CurDayFib {
                leg,
                ..CurDayFib::new()
            };
            assert_eq!(
                f.rung_level(anchor, p, range),
                Some(expected),
                "{leg:?} leg put the level on the wrong side of the anchor"
            );
            assert!(
                tolerance.covers(expected, expected, range),
                "{leg:?}: a close exactly on the level must be covered"
            );
        }
    }

    /// An undetermined leg has no level, and therefore sets nothing.
    #[test]
    fn an_undetermined_leg_has_no_level() {
        let f = CurDayFib {
            leg: Leg::Undetermined,
            ..CurDayFib::new()
        };
        assert_eq!(f.rung_level(2_500_000, 618, 100_000), None);
    }

    /// A level outside `i64` is refused, not clamped onto the OI null sentinel.
    #[test]
    fn an_out_of_range_level_is_refused_not_clamped() {
        let f = CurDayFib {
            leg: Leg::Down,
            ..CurDayFib::new()
        };
        assert_eq!(
            f.rung_level(i64::MAX, 1000, i64::MAX),
            None,
            "clamping to i64::MIN would put §7's null sentinel where a price goes"
        );
    }
}

/// One period of price, and the only input the indicator layer needs.
///
/// # Why this exists instead of `Candle`
///
/// `crates/indicators` used to take `Candle`, which coupled the whole
/// condition layer to **brutex's on-disk file format** for the sake of a seven-field
/// struct. That coupling is what stopped any other project using it: a live-trading
/// consumer holds ticks from a socket, not records from a fixed-stride file, and it
/// should not have to link a storage engine to ask "is this candle a hammer".
///
/// So the indicator layer declares the shape it needs and depends on `vocab` alone.
/// `crates/vocab`, `crates/indicators` and `crates/engine` now form a closure that
/// takes **no brutex-specific dependency at all** — see the crate documentation.
/// A caller holding a `Candle`, a socket tick, or a row from someone
/// else's database converts it at its own boundary, which is where the knowledge of
/// that format already lives.
///
/// The fields are deliberately identical to a stored bar, so the conversion is a
/// struct literal and not a decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Candle {
    /// Microseconds since the Unix epoch, UTC — the **open** (left edge) of the
    /// period. `docs/00-charter.md` verified this against 1,706,290 lake bars.
    pub ts_micros: i64,
    /// Open, in paisa. §7: never a float.
    pub open: i64,
    /// High, in paisa.
    pub high: i64,
    /// Low, in paisa.
    pub low: i64,
    /// Close, in paisa.
    pub close: i64,
    /// Contracts or shares. `0` is a real zero, not an absence.
    pub volume: i64,
    /// Open interest, or [`OI_NULL`] when absent. §7 reserves `i64::MIN` for the
    /// null; zero means zero.
    pub open_interest: i64,
}

/// The open-interest null sentinel. §7 reserves `i64::MIN`; zero means zero.
pub const OI_NULL: i64 = i64::MIN;

impl Candle {
    /// A candle from its seven fields.
    #[must_use]
    pub const fn new(
        ts_micros: i64,
        open: i64,
        high: i64,
        low: i64,
        close: i64,
        volume: i64,
        open_interest: i64,
    ) -> Self {
        Self {
            ts_micros,
            open,
            high,
            low,
            close,
            volume,
            open_interest,
        }
    }

    /// Is this a candle at all?
    ///
    /// # Why this lives on the type and not in nine modules
    ///
    /// Nine modules each need to know whether a record is a bar, and each used to
    /// re-derive it. Nine derivations is nine chances to disagree, and a disagreement
    /// means a **partially evaluated** bar: some families emit, others refuse, and the
    /// mask is a mixture of two answers with nothing recording which.
    ///
    /// A design review found exactly that. The aggregate evaluator was given a
    /// containment check and the nine modules were not, so a mis-assembled OHLC was
    /// refused by the whole and accepted by every part — and a consumer calling
    /// `Patterns::step` directly still got a bar labelled "no wicks" because its wicks
    /// were negative. One definition, nine call sites, is the fix.
    ///
    /// # Errors
    ///
    /// [`Corrupt::HighBelowLow`] when the extremes are inverted, which a market cannot
    /// print — it can print a zero range, never a negative one.
    /// [`Corrupt::RangeOverflows`] when `high - low` leaves `i64`.
    /// [`Corrupt::PriceOutsideRange`] when `open` or `close` sits outside the extremes,
    /// which makes every wick length negative and every shape predicate meaningless.
    pub const fn check(&self) -> Result<(), Corrupt> {
        if self.high < self.low {
            return Err(Corrupt::HighBelowLow);
        }
        if self.high.checked_sub(self.low).is_none() {
            return Err(Corrupt::RangeOverflows);
        }
        if self.open > self.high
            || self.close > self.high
            || self.open < self.low
            || self.close < self.low
        {
            return Err(Corrupt::PriceOutsideRange);
        }
        // Volume is checked HERE, not inside the one module that reads it, and the
        // reason is torn state rather than tidiness.
        //
        // The aggregate evaluator drives nine modules in sequence. VWAP is the only one
        // that reads `volume`, and it ran LAST — so a negative volume was detected after
        // the other eight had already folded the bar into their own accumulators. The
        // evaluator then returned an error, and `a_refused_candle_changes_nothing` went
        // red: the refusal was real but eight modules had already moved. A refusal that
        // leaves state behind is worse than none, because the next bar is evaluated
        // against a session that absorbed a record nobody accepted.
        //
        // Anything checkable from the record alone belongs before the first module runs.
        if self.volume < 0 {
            return Err(Corrupt::NegativeVolume);
        }
        Ok(())
    }

    /// [`Self::check`], plus the zero-price refusal an EVALUATION requires.
    ///
    /// # Why this is a second function and not a fifth clause in `check`
    ///
    /// `check` answers *"is this record structurally sane"* and its contract
    /// deliberately admits an all-zero candle —
    /// `a_defaulted_candle_is_all_zero_and_its_open_interest_is_a_real_zero`
    /// pins it in those words: *"an all-zero candle is sane: zero range, zero
    /// volume, prices contained."* Fourteen callers rely on that reading, and
    /// three fixtures build deliberately extreme bars on top of it. Adding the
    /// clause there refused all four and would have been a silent narrowing of a
    /// tested contract.
    ///
    /// Evaluation asks a stricter question. `5815922c` added the zero refusal
    /// because *"one all-zero bar moved a condition's reported profit by 26.7%
    /// and refused nothing"* — but it wrote it into `Evaluator::stepped`, which
    /// does not call `check` at all: it INLINES its own `HighBelowLow`,
    /// `RangeOverflows` and `PriceOutsideRange` clauses. So the seven modules
    /// that DO call `check` never received it, and `CurDayFib`, `Patterns`,
    /// `Orb`, `SessionState`, `GapFib`, `TrendState` and `Vwap` each accepted a
    /// bar the aggregate refused.
    ///
    /// `every_module_refuses_exactly_what_the_evaluator_refuses` exists to catch
    /// exactly that, and its doc prices it: *"a disagreement means a
    /// partially-evaluated bar: some families emitted, others refused, and the
    /// mask is a mixture of two answers."* This function is where the two
    /// readings are reconciled without either losing its own.
    ///
    /// # Zero and not `<= 0`
    ///
    /// Deliberately the weaker test, and `Evaluator::stepped`'s own doc argues
    /// it: `ohlc_is_sane` refuses a negative bar at the write boundary (D-0143),
    /// so zero is the case REACHABLE THROUGH THE STORE and the case that was
    /// measured. Widening it would make `close_the_books`' span-overflow
    /// handling unreachable through `step`, which §9's coverage floor turns into
    /// a cost rather than a preference. This clause matches that one exactly
    /// rather than restating it differently.
    ///
    /// # Errors
    ///
    /// Everything [`Self::check`] refuses, then [`Corrupt::PriceNotPositive`]
    /// for a record any of whose four prices is zero.
    pub const fn check_evaluable(&self) -> Result<(), Corrupt> {
        // LAST, so it stays strictly additive: every record refused before is
        // still refused for the reason it always was, and this catches only what
        // passed all of `check`.
        if let Err(why) = self.check() {
            return Err(why);
        }
        if self.open == 0 || self.high == 0 || self.low == 0 || self.close == 0 {
            return Err(Corrupt::PriceNotPositive);
        }
        Ok(())
    }

    /// `high - low`, or `None` when the subtraction leaves `i64`.
    ///
    /// A market can print a zero range; it cannot print a negative one, so
    /// `high < low` is a broken record rather than a state to tolerate.
    #[must_use]
    pub const fn range(&self) -> Option<i64> {
        if self.high < self.low {
            return None;
        }
        self.high.checked_sub(self.low)
    }
}

#[cfg(test)]
mod candle {
    use super::*;

    /// The constructor fills the fields in the documented order.
    ///
    /// Seven `i64` arguments in a row is exactly the shape a transposition hides in,
    /// and `Candle::new` had no caller anywhere in the workspace: the bench and every
    /// test build the struct with a literal, where the field names catch a swap. A
    /// caller that does use the constructor would get its high and low exchanged with
    /// nothing to notice it — every wick negative, every `_at_most` predicate
    /// satisfied by a negative, which is the failure `Corrupt::PriceOutsideRange`
    /// documents. The seven values are distinct so no swap can pass.
    #[test]
    fn the_constructor_fills_the_fields_in_the_documented_order() {
        let c = Candle::new(11, 2_450_000, 2_500_000, 2_400_000, 2_460_000, 7, OI_NULL);
        assert_eq!(c.ts_micros, 11, "argument 1 is the timestamp");
        assert_eq!(c.open, 2_450_000, "argument 2 is the open");
        assert_eq!(c.high, 2_500_000, "argument 3 is the high");
        assert_eq!(c.low, 2_400_000, "argument 4 is the low");
        assert_eq!(c.close, 2_460_000, "argument 5 is the close");
        assert_eq!(c.volume, 7, "argument 6 is the volume");
        assert_eq!(
            c.open_interest, OI_NULL,
            "argument 7 is the open interest, and the null sentinel was not carried"
        );
        assert_eq!(c.check(), Ok(()), "a sane candle was refused");
        assert_eq!(
            c.range(),
            Some(100_000),
            "2,500,000 - 2,400,000 is 100,000 paisa"
        );
    }

    /// A range is refused when it would be negative and when it would not fit, and
    /// zero is neither of those.
    ///
    /// A limit-locked or untraded minute prints `high == low`, and answering `None`
    /// for it would drop a real bar from a run while reporting nothing. Nothing in
    /// the workspace called `Candle::range` at all, so none of its three answers had
    /// ever been checked.
    #[test]
    fn a_candle_range_refuses_a_negative_or_unrepresentable_span_and_keeps_zero() {
        let inverted = Candle::new(0, 2_450_000, 2_400_000, 2_500_000, 2_450_000, 0, OI_NULL);
        assert_eq!(
            inverted.range(),
            None,
            "a negative span was reported as a range"
        );
        let straddling = Candle::new(0, 0, i64::MAX, i64::MIN, 0, 0, OI_NULL);
        assert_eq!(
            straddling.range(),
            None,
            "i64::MAX - i64::MIN wrapped instead of refusing"
        );
        let flat = Candle::new(0, 2_500_000, 2_500_000, 2_500_000, 2_500_000, 0, OI_NULL);
        assert_eq!(flat.range(), Some(0), "a zero-range minute is a real bar");
        let wide = Candle::new(0, 2_400_000, 2_500_000, 2_400_000, 2_500_000, 0, OI_NULL);
        assert_eq!(
            wide.range(),
            Some(100_000),
            "the span of a 2,400,000..2,500,000 bar is 100,000 paisa"
        );
    }

    /// EACH OF THE FOUR PRICE CLAUSES REFUSES ON ITS OWN, AND EACH IS STRICT.
    ///
    /// The guard is `open > high || close > high || open < low || close < low`, and
    /// `&&` binds tighter than `||` — so flipping the *k*-th operator does not disable
    /// the chain, it fuses operand *k* with operand *k+1* into one conjunct. Killing
    /// that mutation needs a bar where operand *k* is true and operand *k+1* is false,
    /// which is one bar per clause with the other three satisfied.
    ///
    /// `close > high` with the open INSIDE the range is the one nothing had written,
    /// and the bar it accepts is not exotic: a close above the session high makes
    /// `upper` negative, which satisfies every `_at_most` shadow predicate in
    /// `pattern` and turns the bar into whatever shape is asked of it.
    ///
    /// Each clause is then asked at its boundary. `open == high` is a marubozu open
    /// and a real bar; `>=` would refuse it, and refusing it silently drops the
    /// strongest bars in a trend from every run.
    #[test]
    fn each_price_clause_refuses_alone_and_admits_its_own_boundary() {
        let sane = |open: i64, close: i64| {
            Candle::new(0, open, 2_500_000, 2_400_000, close, 0, OI_NULL).check()
        };
        assert_eq!(
            sane(2_450_000, 2_460_000),
            Ok(()),
            "the fixture must be accepted first -- breaking one clause of a bar \
             that was already refused proves nothing about that clause"
        );

        for (open, close, clause) in [
            (2_500_001, 2_460_000, "an open ABOVE the high"),
            (2_450_000, 2_500_001, "a close ABOVE the high"),
            (2_399_999, 2_460_000, "an open BELOW the low"),
            (2_450_000, 2_399_999, "a close BELOW the low"),
        ] {
            assert_eq!(
                sane(open, close),
                Err(Corrupt::PriceOutsideRange),
                "{clause} was accepted, and every other clause of this bar holds"
            );
        }

        // The four boundaries themselves. All four comparisons are strict, and a bar
        // that opens or closes exactly ON an extreme is the commonest bar there is.
        for (open, close, edge) in [
            (2_500_000, 2_460_000, "an open exactly AT the high"),
            (2_450_000, 2_500_000, "a close exactly AT the high"),
            (2_400_000, 2_460_000, "an open exactly AT the low"),
            (2_450_000, 2_400_000, "a close exactly AT the low"),
        ] {
            assert_eq!(
                sane(open, close),
                Ok(()),
                "{edge} is a real bar; `>` must not become `>=`, nor `<` become `<=`"
            );
        }
    }

    /// A defaulted candle carries a **real** zero open interest, not the null.
    ///
    /// `Candle` derives `Default`, and §7 reserves `i64::MIN` for "absent" while zero
    /// means zero — so `Candle::default()` states that the instrument has no open
    /// contracts, not that its open interest is unknown. Nothing in the workspace
    /// calls it, so nothing had ever said which of the two it means.
    ///
    /// This records the behaviour rather than endorsing it: if the derive is ever
    /// replaced by a hand-written `Default` that uses [`OI_NULL`], this test goes red
    /// and the choice gets made deliberately instead of by a derive.
    #[test]
    fn a_defaulted_candle_is_all_zero_and_its_open_interest_is_a_real_zero() {
        let d = Candle::default();
        assert_eq!(
            d,
            Candle::new(0, 0, 0, 0, 0, 0, 0),
            "the derived default is not all-zero"
        );
        assert_ne!(
            d.open_interest, OI_NULL,
            "a defaulted candle claimed an ABSENT open interest"
        );
        assert_eq!(
            d.check(),
            Ok(()),
            "an all-zero candle is sane: zero range, zero volume, prices contained"
        );
        assert_eq!(d.range(), Some(0), "an all-zero candle has a zero range");
    }

    /// ANY ONE of the four prices at zero refuses, not only all four together.
    ///
    /// # The three surviving mutants this kills, and why they survived
    ///
    /// `check_evaluable`'s guard is a four-way `||`. Mutation testing replaced
    /// each `||` with `&&` in turn and **three of the three survived**, because
    /// every test that reached the clause used the ALL-ZERO candle — and
    /// `open == 0 && high == 0 && low == 0 && close == 0` is true of that bar
    /// too. A guard that fires on one zero and a guard that fires only on four
    /// are different refusals, and nothing here could tell them apart.
    ///
    /// # Each fixture isolates ONE zero, and `check` must still accept it
    ///
    /// The point is to reach `check_evaluable`'s own clause rather than trip an
    /// earlier one, so every bar below passes all four of `check`: `high >= low`,
    /// the range is representable, `open` and `close` are inside `[low, high]`,
    /// and `volume` is not negative. That is why the non-zero prices go NEGATIVE
    /// where they have to — an `open` of zero with a positive `low` is
    /// `PriceOutsideRange`, which would prove nothing about this guard.
    ///
    /// Negative prices are legal to `check` by construction (D-0143 refuses them
    /// at the write boundary instead), which is exactly what makes them usable
    /// as the non-zero half of these fixtures.
    #[test]
    fn one_zero_price_is_enough_to_refuse_an_evaluable_candle() {
        for (name, candle) in [
            // open alone: `low` is negative so a zero open stays inside the range.
            ("open", Candle::new(0, 0, 100, -10, 50, 0, OI_NULL)),
            // high alone: every price is at or below zero, so `high == 0` is the
            // only zero and `low` is far from it.
            ("high", Candle::new(0, -50, 0, -100, -50, 0, OI_NULL)),
            // low alone, with a positive body above it.
            ("low", Candle::new(0, 50, 100, 0, 50, 0, OI_NULL)),
            // close alone.
            ("close", Candle::new(0, 50, 100, -10, 0, 0, OI_NULL)),
        ] {
            assert_eq!(
                candle.check(),
                Ok(()),
                "{name}: the fixture must reach the zero clause, not an earlier one"
            );
            assert_eq!(
                candle.check_evaluable(),
                Err(Corrupt::PriceNotPositive),
                "{name}: a single zero price must refuse evaluation on its own"
            );
        }
    }

    /// The weekday map, checked against dates a reader can verify.
    ///
    /// 1970-01-01 was a Thursday, which is what makes `day % 7 == 0` Thursday.
    /// A map built on the wrong anchor is off by a constant and every assertion
    /// below would move together, so the fixtures are real dates rather than
    /// offsets from one another.
    #[test]
    fn the_weekday_map_is_anchored_on_a_real_calendar() {
        // 2026-08-24 is a Monday; 09:15 IST is 03:45 UTC.
        let monday = 1_787_000_000_000_000_i64;
        // Walk a whole week from a known Monday and check the sequence.
        let day = 86_400_000_000_i64;
        let base = (monday / day) * day + 4 * 3_600_000_000; // mid-session IST
        // Find a Monday by scanning at most a week.
        let mut start = base;
        for _ in 0..7 {
            if crate::weekday_bit(start) == Some(365) {
                break;
            }
            start = start.saturating_add(day);
        }
        assert_eq!(crate::weekday_bit(start), Some(365), "monday");
        assert_eq!(crate::weekday_bit(start + day), Some(366), "tuesday");
        assert_eq!(crate::weekday_bit(start + 2 * day), Some(367), "wednesday");
        assert_eq!(crate::weekday_bit(start + 3 * day), Some(368), "thursday");
        assert_eq!(crate::weekday_bit(start + 4 * day), Some(369), "friday");
        // NSE DOES NOT TRADE THESE, so nothing is set rather than a neighbour
        // being guessed. A weekend bar in an equity series is a store defect and
        // folding it into Friday would hide the only symptom of it.
        assert_eq!(crate::weekday_bit(start + 5 * day), None, "saturday");
        assert_eq!(crate::weekday_bit(start + 6 * day), None, "sunday");
        // And the week closes.
        assert_eq!(
            crate::weekday_bit(start + 7 * day),
            Some(365),
            "next monday"
        );
    }
}
