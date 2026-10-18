//! The day index: everything the walker needs about one stock-day, built ONCE
//! per stock-day in O(bars + candles x widths + seconds of the day), so every
//! slot step afterwards is a fixed number of comparisons (FM-F030; D-6380).
//!
//! What it holds, per side (a short's prices are stored negated, so one state
//! machine walks both and the mirror is exact):
//!
//! * every bar below 15:10:00 IST (a later bar is dropped here and never read
//!   by any exit, FM-F039), with its prices on the exact common denominator
//!   (D-6383), its book sides, its print flag, its halt state, the silence
//!   before it, and whether that silence covers a traded Zerodha minute
//!   (FM-F041);
//! * the forced window's summary: its usable seconds, the seconds a position
//!   can exit into, their summed volume and their worst price (FM-F048), or
//!   stage A's 15:09 minute;
//! * suffix extremes, so the excursions to 15:09:59 are one read each;
//! * the confirmed swings of every timeframe at every width the exit set uses,
//!   with a refused swing across a withheld candle kept as a hole (FM-F045).
//!
//! The second-to-bar map and the minute-presence prefix make a slot's first
//! bar and a silence's Zerodha check one read each. Bounds are proved by
//! `crates/runner/tests/fib_walk.rs` and measured by
//! `crates/runner/benches/fib_rows/walk.rs`.

use core::mem::size_of;
use std::collections::VecDeque;

use indicators::fibflag::types::FibCandle;
use indicators::{Candle, IST_OFFSET_MICROS, ist_day};
use vocab::fib_cube::{
    MICROS_PER_SECOND, PERMILLE_PER_UNIT, SECONDS_PER_DAY, SECONDS_PER_MINUTE,
    SESSION_AUTO_EXIT_BY_SECS, SESSION_FORCED_WINDOW_START_SECS, Search,
};

use super::super::types::{ClockRule, CrossFeed, PriceMap, Resolution, SnapshotEdge, WalkProfile};
use super::event::Events;
use super::exits::ExitSet;
use super::{BookEvidence, BookMark, ClockLag, DayInput, HaltRange, TrailCandles, WalkRefusal};

/// One fact about one bar, as a bit of [`SideBar::bits`]. The declaration
/// order is the bit position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bit {
    /// The bar failed `Candle::check_evaluable` (FM-F044).
    Corrupt,
    /// The bar lies inside a recorded halt.
    Halted,
    /// A recorded halt started after the previous bar and at or before this.
    HaltCrossed,
    /// The side's entry has liquidity to take here (OF-1).
    EntryOk,
    /// The side's exit has liquidity to take here (OF-1).
    ExitOk,
    /// PREC-3 flagged this print.
    Flagged,
    /// Stage A: an entry here would buy into a minute locked at the running
    /// day extreme (FM-F042, UNVERIFIED proxy).
    EntryLocked,
    /// Stage A: an exit here would sell into a locked minute.
    ExitLocked,
    /// The silence before this bar covers a traded Zerodha minute (deep).
    GapHole,
    /// The bar's own Zerodha minute traded (deep).
    Witnessed,
}

/// One bar of one side, mirrored for a short.
#[derive(Clone, Copy, Debug)]
pub(super) struct SideBar {
    /// Seconds after IST midnight (a minute bar: its open second).
    pub(super) second: u32,
    /// The high on the common denominator, mirrored.
    pub(super) hi: i128,
    /// The low on the common denominator, mirrored.
    pub(super) lo: i128,
    /// The open on the common denominator, mirrored.
    pub(super) op: i128,
    /// The worst price a buy pays here, raw paisa, mirrored.
    pub(super) hi_raw: i64,
    /// The worst price a sell gets here, raw paisa, mirrored.
    pub(super) lo_raw: i64,
    /// The bar's volume.
    pub(super) volume: u64,
    /// The bar's facts, one [`Bit`] each.
    pub(super) bits: u16,
    /// Missing seconds between the previous bar and this one.
    pub(super) gap_missing: u32,
    /// The bar's Zerodha minute's converted low, mirrored (deep only).
    pub(super) witness_lo: i128,
    /// That minute's converted high, mirrored (deep only).
    pub(super) witness_hi: i128,
}

impl SideBar {
    /// Whether the bar carries `bit`.
    pub(super) const fn is(&self, bit: Bit) -> bool {
        self.bits & (1 << bit as u16) != 0
    }

    /// Whether a fill or exit may read the bar's prices.
    pub(super) const fn usable(&self) -> bool {
        !self.is(Bit::Corrupt) && !self.is(Bit::Halted)
    }
}

/// `bits` with `bit` set when `on`.
const fn set(bits: u16, bit: Bit, on: bool) -> u16 {
    if on { bits | (1 << bit as u16) } else { bits }
}

/// The forced window's summary for one side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Forced {
    /// Usable bars inside `[15:09:00, 15:10:00)`.
    pub(super) bars: u32,
    /// Those a position of this side can exit into.
    pub(super) capable: u32,
    /// Their summed volume.
    pub(super) capacity: u128,
    /// Their worst exit price, raw paisa, mirrored.
    pub(super) worst_raw: i64,
    /// The last of them: the forced order's last fill second.
    pub(super) last_second: u32,
    /// Stage A: the 15:09 minute's exit price, when the minute is there,
    /// usable and not locked.
    pub(super) minute: Option<i64>,
    /// The best buy-side price of every usable window bar, raw, mirrored
    /// (`i64::MIN` for an empty window): an open position's excursions run
    /// through the window in one read.
    pub(super) best_raw: i64,
    /// The first second that best price printed.
    pub(super) best_first: u32,
    /// The last second it printed.
    pub(super) best_last: u32,
    /// The worst sell-side price of every usable window bar, raw, mirrored.
    pub(super) low_raw: i64,
}

/// One confirmed swing of one timeframe at one width, or a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SwingEvent {
    /// The close of the confirming candle, the `width`-th after the swing.
    pub(super) confirm_second: u32,
    /// The close of the swing candle itself.
    pub(super) candle_close_second: u32,
    /// The stop level the swing raises to (the converted low less one tick),
    /// mirrored; `None` for a swing refused across a withheld candle.
    pub(super) level: Option<i128>,
}

/// One timeframe's swings at one width for one side.
#[derive(Clone, Debug)]
pub(super) struct SwingList {
    /// The events, ascending by confirm second.
    pub(super) events: Vec<SwingEvent>,
    /// For each minute of the day, the first event confirmed at or after it.
    pub(super) by_minute: Vec<u32>,
}

/// One side of the day.
#[derive(Clone, Debug)]
pub(super) struct SideIndex {
    /// `1` for a long, `-1` for a short: a real price is `sign x` a stored one.
    pub(super) sign: i64,
    /// The bars, mirrored.
    pub(super) bars: Vec<SideBar>,
    /// The best buy-side price of the usable bars from each index on, raw
    /// paisa, mirrored (`i64::MIN` past the last).
    pub(super) best_from: Vec<i64>,
    /// The worst sell-side price of the usable bars from each index on
    /// (`i64::MAX` past the last).
    pub(super) worst_from: Vec<i64>,
    /// The forced window.
    pub(super) forced: Forced,
    /// The swings, `[list x widths + width]`.
    pub(super) swings: Vec<SwingList>,
    /// The event walk's tables (FM-F030).
    pub(super) events: Events,
}

/// One stock-day, indexed once for every slot the walker steps (FM-F030).
#[derive(Clone, Debug)]
pub struct DayIndex {
    /// The walk's profile.
    pub(super) profile: WalkProfile,
    /// A decision-basis price times this is on the common denominator.
    pub(super) decision_scale: i128,
    /// A raw price times this is on the common denominator.
    pub(super) raw_scale: i128,
    /// The Zerodha close of the factor pair (one at stage A).
    pub(super) zerodha_close: i64,
    /// The GDFL close of the factor pair (one at stage A).
    pub(super) gdfl_close: i64,
    /// The walk's one-tick offset on the common denominator: the raw tick in
    /// deep, the decision tick at stage A.
    pub(super) tick: i128,
    /// The decision tick, in paisa.
    pub(super) decision_tick: i64,
    /// For each second of the day, the first bar at or after it.
    first_at: Vec<u32>,
    /// For each minute of the day, the traded Zerodha minutes before it.
    present_before: Vec<u32>,
    /// The halts, ascending.
    halts: Vec<HaltRange>,
    /// The committed past's longest silence, in missing seconds.
    pub(super) max_gap: u32,
    /// Seconds after the decision instant before a fill may start.
    pub(super) entry_shift: u32,
    /// Whether the day's book evidence was supplied.
    book_observed: bool,
    /// Whether the clock lag was walked unchecked.
    trust_unchecked: bool,
    /// The long side.
    long: SideIndex,
    /// The short side.
    short: SideIndex,
    /// Each trail list's timeframe.
    pub(super) trail_tfs: Vec<u16>,
    /// The exit set's swing widths.
    pub(super) widths: Vec<u16>,
    /// Bars that failed `Candle::check_evaluable`.
    corrupt_bars: u64,
    /// Bars at or after 15:10:00, dropped unread.
    dropped_after_auto_exit: u64,
    /// Swings refused across a withheld candle, both sides.
    trail_holes: u64,
}

/// Seconds after IST midnight of a timestamp, and its IST day.
fn clock(ts_micros: i64) -> (i64, u32) {
    let micros = i128::from(ts_micros) + i128::from(IST_OFFSET_MICROS);
    let day_micros = i128::from(SECONDS_PER_DAY) * i128::from(MICROS_PER_SECOND);
    let second = micros.rem_euclid(day_micros) / i128::from(MICROS_PER_SECOND);
    (
        ist_day(ts_micros),
        u32::try_from(second).unwrap_or(u32::MAX),
    )
}

/// Whether `ts_micros` lies on the resolution's grid.
fn on_grid(ts_micros: i64, resolution: Resolution) -> bool {
    let step = match resolution {
        Resolution::Minute => i128::from(SECONDS_PER_MINUTE) * i128::from(MICROS_PER_SECOND),
        Resolution::Second => i128::from(MICROS_PER_SECOND),
    };
    i128::from(ts_micros).rem_euclid(step) == 0
}

/// The minute of the day a second falls in.
fn minute_of(second: u32) -> usize {
    usize::try_from(u64::from(second) / SECONDS_PER_MINUTE).unwrap_or(usize::MAX)
}

/// How many minutes a day has.
fn minutes_per_day() -> usize {
    usize::try_from(SECONDS_PER_DAY / SECONDS_PER_MINUTE).unwrap_or(usize::MAX)
}

/// `value x scale`, refused unless one more factor of a million still fits:
/// the exact comparisons multiply a scaled price by at most one permille
/// factor, and a sum or difference of two.
fn scaled(value: i64, scale: i128) -> Result<i128, WalkRefusal> {
    let headroom = i128::from(PERMILLE_PER_UNIT) * i128::from(PERMILLE_PER_UNIT);
    let out = i128::from(value)
        .checked_mul(scale)
        .ok_or(WalkRefusal::PriceBeyondExactRange)?;
    out.checked_mul(headroom)
        .ok_or(WalkRefusal::PriceBeyondExactRange)?;
    Ok(out)
}

/// The factor pair, the entry shift and the trust label a profile and a day
/// give.
struct Basis {
    /// The Zerodha close of the pair.
    zerodha_close: i64,
    /// The GDFL close of the pair.
    gdfl_close: i64,
    /// Seconds after the decision before a fill may start.
    entry_shift: u32,
    /// Whether the clock lag was walked unchecked.
    trust_unchecked: bool,
}

/// The basis of a walk, refusing an impossible profile or day by name.
fn basis(profile: &WalkProfile, input: &DayInput<'_>) -> Result<Basis, WalkRefusal> {
    let deep = profile.resolution == Resolution::Second;
    let (zerodha_close, gdfl_close) = match profile.price_map {
        PriceMap::Identity => (1, 1),
        PriceMap::Factor {
            zerodha_close,
            gdfl_close,
        } => {
            if !deep {
                return Err(WalkRefusal::StageATakesIdentity);
            }
            if zerodha_close <= 0 || gdfl_close <= 0 {
                return Err(WalkRefusal::NonpositiveFactor);
            }
            (zerodha_close, gdfl_close)
        }
    };
    if input.raw_tick <= 0 || input.decision_tick <= 0 {
        return Err(WalkRefusal::NonpositiveTick);
    }
    let mut basis = Basis {
        zerodha_close,
        gdfl_close,
        entry_shift: 0,
        trust_unchecked: false,
    };
    if !deep {
        return Ok(basis);
    }
    if profile.cross_feed == CrossFeed::WorseOfBoth {
        return Err(WalkRefusal::CrossFeedNotBuilt);
    }
    if input.zerodha_minutes.is_empty() {
        return Err(WalkRefusal::ZerodhaMinutesAbsent);
    }
    if profile.snapshot_edge == SnapshotEdge::StampClosesInterval {
        basis.entry_shift = 1;
    }
    match input.clock_lag {
        ClockLag::Measured(0) => {}
        ClockLag::Measured(lag) => match profile.clock_rule {
            ClockRule::RefuseShifted => return Err(WalkRefusal::ClockShifted { lag }),
            ClockRule::Restamp => {
                basis.entry_shift += u32::try_from(lag.max(0)).unwrap_or(0);
            }
        },
        ClockLag::Unmeasured => return Err(WalkRefusal::ClockLagUnmeasured),
        ClockLag::TrustUnchecked => basis.trust_unchecked = true,
    }
    Ok(basis)
}

/// For each minute of the day, the traded minutes before it.
fn presence_prefix(present: &[Option<(i64, i64)>]) -> Vec<u32> {
    let mut before = Vec::with_capacity(minutes_per_day().saturating_add(1));
    before.push(0);
    let mut running = 0;
    for minute in 0..minutes_per_day() {
        if present.get(minute).is_some_and(Option::is_some) {
            running += 1;
        }
        before.push(running);
    }
    before
}

/// For each second of the day, the first kept bar at or after it.
fn first_at_map(kept: &[(usize, Candle, u32)]) -> Vec<u32> {
    let seconds = usize::try_from(SECONDS_PER_DAY)
        .unwrap_or(0)
        .saturating_add(1);
    let mut first_at = Vec::with_capacity(seconds);
    let mut next = 0;
    for second in 0..seconds {
        while kept
            .get(next)
            .is_some_and(|(_, _, s)| usize::try_from(*s).is_ok_and(|s| s < second))
        {
            next += 1;
        }
        first_at.push(u32::try_from(next).unwrap_or(u32::MAX));
    }
    first_at
}

impl DayIndex {
    /// Indexes one stock-day for `profile`, with the swing widths of `exits`.
    ///
    /// # Errors
    ///
    /// By name: unsorted or repeated bars, bars of two days, a bar off its
    /// grid, book marks not one per bar, invalid halts, a nonpositive tick or
    /// factor, a stage-A factor map, the unbuilt worse-of-both reading, a deep
    /// day with no Zerodha minute, a shifted or unmeasured clock, an invalid
    /// trail list, or a price beyond the exact range.
    pub fn build(
        profile: &WalkProfile,
        input: &DayInput<'_>,
        exits: &ExitSet,
    ) -> Result<Self, WalkRefusal> {
        let deep = profile.resolution == Resolution::Second;
        let basis = basis(profile, input)?;
        let permille = i128::from(PERMILLE_PER_UNIT);
        let raw_scale = i128::from(basis.zerodha_close) * permille * permille;
        let decision_scale = i128::from(basis.gdfl_close) * permille * permille;
        let walk_tick = if deep {
            input.raw_tick
        } else {
            input.decision_tick
        };
        let tick = scaled(walk_tick, raw_scale)?;
        let halts = checked_halts(input.halts)?;
        let book = match input.book {
            BookEvidence::Observed(marks) if marks.len() != input.bars.len() => {
                return Err(WalkRefusal::BookMarksLength);
            }
            BookEvidence::Observed(marks) => Some(marks),
            BookEvidence::NotObserved => None,
        };
        let clocks = checked_bars(input.bars, profile.resolution)?;
        let present = if deep {
            zerodha_presence(input.zerodha_minutes)?
        } else {
            Vec::new()
        };
        let present_before = presence_prefix(&present);
        let trail_tfs = checked_trails(input.trail)?;
        let mut corrupt_bars = 0;
        let mut kept = Vec::with_capacity(input.bars.len());
        for (at, (bar, second)) in input.bars.iter().zip(&clocks).enumerate() {
            if u64::from(*second) >= SESSION_AUTO_EXIT_BY_SECS {
                continue;
            }
            if bar.check_evaluable().is_err() {
                corrupt_bars += 1;
            }
            kept.push((at, *bar, *second));
        }
        let dropped_after_auto_exit = u64::try_from(input.bars.len() - kept.len()).unwrap_or(0);
        let shared = Shared {
            kept: &kept,
            book,
            halts: &halts,
            present: &present,
            present_before: &present_before,
            raw_scale,
            decision_scale,
            tick,
            deep,
            max_gap: input.max_second_gap,
        };
        let mut trail_holes = 0;
        let long = shared.side(Search::Long, input.trail, exits.widths(), &mut trail_holes)?;
        let short = shared.side(Search::Short, input.trail, exits.widths(), &mut trail_holes)?;
        Ok(Self {
            profile: *profile,
            decision_scale,
            raw_scale,
            zerodha_close: basis.zerodha_close,
            gdfl_close: basis.gdfl_close,
            tick,
            decision_tick: input.decision_tick,
            first_at: first_at_map(&kept),
            present_before,
            halts,
            max_gap: input.max_second_gap,
            entry_shift: basis.entry_shift,
            book_observed: book.is_some(),
            trust_unchecked: basis.trust_unchecked,
            long,
            short,
            trail_tfs,
            widths: exits.widths().to_vec(),
            corrupt_bars,
            dropped_after_auto_exit,
            trail_holes,
        })
    }

    /// The walk's resolution.
    #[must_use]
    pub const fn resolution(&self) -> Resolution {
        self.profile.resolution
    }

    /// Whether the day's book evidence was supplied: without it the
    /// one-sided-book rule could not apply, and every deep result of the day
    /// says so.
    #[must_use]
    pub const fn book_observed(&self) -> bool {
        self.book_observed
    }

    /// Whether the clock lag was walked unchecked (`--seconds-trust
    /// unchecked`), a label every result of the day carries.
    #[must_use]
    pub const fn seconds_trust_unchecked(&self) -> bool {
        self.trust_unchecked
    }

    /// Bars that failed `Candle::check_evaluable`: counted, and never priced.
    #[must_use]
    pub const fn corrupt_bars(&self) -> u64 {
        self.corrupt_bars
    }

    /// Bars at or after 15:10:00 IST, dropped unread (FM-F039).
    #[must_use]
    pub const fn dropped_after_auto_exit(&self) -> u64 {
        self.dropped_after_auto_exit
    }

    /// Swings refused across a withheld candle, both sides (FM-F045).
    #[must_use]
    pub const fn trail_holes(&self) -> u64 {
        self.trail_holes
    }

    /// The bars kept, at most one per second (or minute) below 15:10:00.
    #[must_use]
    pub fn bars(&self) -> usize {
        self.long.bars.len()
    }

    /// The bytes the index holds: its working set, for a memory budget.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        let words =
            |n: usize, width: usize| u64::try_from(n.saturating_mul(width)).unwrap_or(u64::MAX);
        let side = |s: &SideIndex| {
            let swings: u64 = s
                .swings
                .iter()
                .map(|l| {
                    words(l.events.len(), size_of::<SwingEvent>())
                        .saturating_add(words(l.by_minute.len(), size_of::<u32>()))
                })
                .sum();
            words(s.bars.len(), size_of::<SideBar>())
                .saturating_add(words(s.best_from.len(), size_of::<i64>()))
                .saturating_add(words(s.worst_from.len(), size_of::<i64>()))
                .saturating_add(swings)
                .saturating_add(u64::try_from(s.events.bytes()).unwrap_or(u64::MAX))
        };
        words(self.first_at.len(), size_of::<u32>())
            .saturating_add(words(self.present_before.len(), size_of::<u32>()))
            .saturating_add(words(self.halts.len(), size_of::<HaltRange>()))
            .saturating_add(side(&self.long))
            .saturating_add(side(&self.short))
    }

    /// One side.
    pub(super) const fn side(&self, direction: Search) -> &SideIndex {
        match direction {
            Search::Long => &self.long,
            Search::Short => &self.short,
        }
    }

    /// The first bar at or after `second`.
    pub(super) fn first_bar_at(&self, second: u32) -> usize {
        usize::try_from(second)
            .ok()
            .and_then(|s| self.first_at.get(s))
            .and_then(|at| usize::try_from(*at).ok())
            .unwrap_or_else(|| self.bars())
    }

    /// Whether a traded Zerodha minute covers any second of `[from, to]`.
    pub(super) fn traded_between(&self, from: u32, to: u32) -> bool {
        let read = |minute: usize| self.present_before.get(minute).copied().unwrap_or(0);
        from <= to && read(minute_of(to).saturating_add(1)) > read(minute_of(from))
    }

    /// Whether a recorded halt starts at or before `second`.
    pub(super) fn halted_by(&self, second: u32) -> bool {
        self.halts
            .first()
            .is_some_and(|halt| halt.start_second <= second)
    }
}

/// What both sides of one build share.
struct Shared<'a> {
    /// The kept bars: input index, bar, second.
    kept: &'a [(usize, Candle, u32)],
    /// The book marks, by input index.
    book: Option<&'a [BookMark]>,
    /// The halts.
    halts: &'a [HaltRange],
    /// Each minute's Zerodha low and high, when it traded.
    present: &'a [Option<(i64, i64)>],
    /// The traded minutes before each minute.
    present_before: &'a [u32],
    /// The raw scale.
    raw_scale: i128,
    /// The decision scale.
    decision_scale: i128,
    /// The one-tick offset.
    tick: i128,
    /// Whether this is a deep walk.
    deep: bool,
    /// The committed past's longest silence.
    max_gap: u32,
}

/// What the bar walk of one side carries from one bar to the next.
struct Running {
    /// The next halt that has not ended.
    halt_end: usize,
    /// The next halt that has not started.
    halt_start: usize,
    /// The previous bar's second.
    previous: Option<u32>,
    /// The running day high of the usable bars, mirrored.
    hi: Option<i128>,
    /// The running day low.
    lo: Option<i128>,
}

/// A bar's prices on the common denominator and raw, mirrored.
struct Prices {
    /// The high, scaled.
    hi: i128,
    /// The low, scaled.
    lo: i128,
    /// The open, scaled.
    op: i128,
    /// The high, raw.
    hi_raw: i64,
    /// The low, raw.
    lo_raw: i64,
}

impl Shared<'_> {
    /// One side's index.
    fn side(
        &self,
        direction: Search,
        trails: &[TrailCandles<'_>],
        widths: &[u16],
        holes: &mut u64,
    ) -> Result<SideIndex, WalkRefusal> {
        let long = direction == Search::Long;
        let sign: i64 = if long { 1 } else { -1 };
        let mut running = Running {
            halt_end: 0,
            halt_start: 0,
            previous: None,
            hi: None,
            lo: None,
        };
        let mut bars = Vec::with_capacity(self.kept.len());
        for (at, bar, second) in self.kept {
            bars.push(self.side_bar(long, *at, bar, *second, &mut running)?);
        }
        let (best_from, worst_from) = suffixes(&bars);
        let forced = forced_window(&bars, self.deep);
        let events = Events::build(&bars, self.max_gap, self.deep);
        let mut swings = Vec::with_capacity(trails.len() * widths.len());
        for list in trails {
            for width in widths {
                swings.push(self.swings(list.candles, *width, long, holes)?);
            }
        }
        Ok(SideIndex {
            sign,
            bars,
            best_from,
            worst_from,
            forced,
            swings,
            events,
        })
    }

    /// A bar's mirrored prices.
    fn prices(&self, long: bool, bar: &Candle) -> Result<Prices, WalkRefusal> {
        let (hi_raw, lo_raw, open) = if long {
            (bar.high, bar.low, bar.open)
        } else {
            (
                bar.low.saturating_neg(),
                bar.high.saturating_neg(),
                bar.open.saturating_neg(),
            )
        };
        Ok(Prices {
            hi: scaled(hi_raw, self.raw_scale)?,
            lo: scaled(lo_raw, self.raw_scale)?,
            op: scaled(open, self.raw_scale)?,
            hi_raw,
            lo_raw,
        })
    }

    /// One bar of one side.
    fn side_bar(
        &self,
        long: bool,
        at: usize,
        bar: &Candle,
        second: u32,
        running: &mut Running,
    ) -> Result<SideBar, WalkRefusal> {
        let Prices {
            hi,
            lo,
            op,
            hi_raw,
            lo_raw,
        } = self.prices(long, bar)?;
        while self
            .halts
            .get(running.halt_end)
            .is_some_and(|h| h.end_second <= second)
        {
            running.halt_end += 1;
        }
        let halted = self
            .halts
            .get(running.halt_end)
            .is_some_and(|h| h.start_second <= second);
        let starts_before = running.halt_start;
        while self
            .halts
            .get(running.halt_start)
            .is_some_and(|h| h.start_second <= second)
        {
            running.halt_start += 1;
        }
        let mark = self
            .book
            .and_then(|marks| marks.get(at))
            .copied()
            .unwrap_or(BookMark {
                ask: true,
                bid: true,
                flagged_print: false,
            });
        let (entry_ok, exit_ok) = if long {
            (mark.ask, mark.bid)
        } else {
            (mark.bid, mark.ask)
        };
        let corrupt = bar.check_evaluable().is_err();
        let usable = !corrupt && !halted;
        if usable {
            running.hi = Some(running.hi.map_or(hi, |r| r.max(hi)));
            running.lo = Some(running.lo.map_or(lo, |r| r.min(lo)));
        }
        let locked = !self.deep && usable && hi == lo;
        let gap_missing = running
            .previous
            .map_or(0, |p| second.saturating_sub(p).saturating_sub(1));
        let gap_hole = self.deep
            && gap_missing > 0
            && running.previous.is_some_and(|p| {
                let read = |m: usize| self.present_before.get(m).copied().unwrap_or(0);
                let from = minute_of(p.saturating_add(1));
                read(minute_of(second.saturating_sub(1)).saturating_add(1)) > read(from)
            });
        running.previous = Some(second);
        let minute = self.present.get(minute_of(second)).copied().flatten();
        let (witness_lo, witness_hi) = match minute {
            Some((z_low, z_high)) if long => (
                scaled(z_low, self.decision_scale)?,
                scaled(z_high, self.decision_scale)?,
            ),
            Some((z_low, z_high)) => (
                scaled(z_high.saturating_neg(), self.decision_scale)?,
                scaled(z_low.saturating_neg(), self.decision_scale)?,
            ),
            None => (0, 0),
        };
        let mut bits = set(0, Bit::Corrupt, corrupt);
        bits = set(bits, Bit::Halted, halted);
        bits = set(bits, Bit::HaltCrossed, running.halt_start > starts_before);
        bits = set(bits, Bit::EntryOk, entry_ok);
        bits = set(bits, Bit::ExitOk, exit_ok);
        bits = set(bits, Bit::Flagged, mark.flagged_print);
        bits = set(bits, Bit::EntryLocked, locked && running.hi == Some(hi));
        bits = set(bits, Bit::ExitLocked, locked && running.lo == Some(lo));
        bits = set(bits, Bit::GapHole, gap_hole);
        bits = set(bits, Bit::Witnessed, minute.is_some());
        Ok(SideBar {
            second,
            hi,
            lo,
            op,
            hi_raw,
            lo_raw,
            volume: u64::try_from(bar.volume).unwrap_or(0),
            bits,
            gap_missing,
            witness_lo,
            witness_hi,
        })
    }

    /// One timeframe's swings at one width, mirrored for a short (FM-F045):
    /// a candle whose low is below the `width` candles on each side, all
    /// usable and contiguous on the 09:15 grid. A window that would hold a
    /// swing but crosses a withheld or unusable candle is a hole. Sliding
    /// minima make it O(candles) for any width.
    fn swings(
        &self,
        candles: &[FibCandle],
        width: u16,
        long: bool,
        holes: &mut u64,
    ) -> Result<SwingList, WalkRefusal> {
        let w = usize::from(width);
        let n = candles.len();
        let lows: Vec<i64> = candles
            .iter()
            .map(|c| {
                if !usable_candle(c) {
                    i64::MAX
                } else if long {
                    c.candle.low
                } else {
                    c.candle.high.saturating_neg()
                }
            })
            .collect();
        let closes: Vec<u32> = candles.iter().map(|c| clock(c.close_ts_micros).1).collect();
        let run_start = run_starts(candles);
        let win_min = window_minima(&lows, w);
        let mut events = Vec::new();
        for j in w..n.saturating_sub(w) {
            // `w <= j < n - w`, so all three are present; a missing one reads
            // as no swing.
            let low = lows.get(j).copied().unwrap_or(i64::MAX);
            let left = win_min.get(j - 1).copied().unwrap_or(i64::MIN);
            let right = win_min.get(j + w).copied().unwrap_or(i64::MIN);
            if low == i64::MAX || low >= left || low >= right {
                continue;
            }
            let contiguous = run_start.get(j + w).is_some_and(|s| *s <= j - w);
            let level = if contiguous {
                Some(scaled(low, self.decision_scale)? - self.tick)
            } else {
                *holes += 1;
                None
            };
            events.push(SwingEvent {
                confirm_second: closes.get(j + w).copied().unwrap_or(u32::MAX),
                candle_close_second: closes.get(j).copied().unwrap_or(u32::MAX),
                level,
            });
        }
        let mut by_minute = Vec::with_capacity(minutes_per_day().saturating_add(1));
        let mut at = 0;
        for minute in 0..=minutes_per_day() {
            let start = u64::try_from(minute).unwrap_or(u64::MAX) * SECONDS_PER_MINUTE;
            while events
                .get(at)
                .is_some_and(|e| u64::from(e.confirm_second) < start)
            {
                at += 1;
            }
            by_minute.push(u32::try_from(at).unwrap_or(u32::MAX));
        }
        Ok(SwingList { events, by_minute })
    }
}

/// Whether a trail candle may be read: complete, with no missing minute, and
/// evaluable.
fn usable_candle(c: &FibCandle) -> bool {
    c.complete && c.missing_minutes == 0 && c.candle.check_evaluable().is_ok()
}

/// For each candle, where the contiguous usable run ending at it starts (one
/// past it when it is unusable itself).
fn run_starts(candles: &[FibCandle]) -> Vec<usize> {
    let mut run_start: Vec<usize> = Vec::with_capacity(candles.len());
    let mut previous: Option<&FibCandle> = None;
    for (k, candle) in candles.iter().enumerate() {
        let joined = previous
            .is_some_and(|p| p.close_ts_micros == candle.candle.ts_micros && usable_candle(p));
        let start = if !usable_candle(candle) {
            k.saturating_add(1)
        } else if joined {
            run_start.last().copied().unwrap_or(k)
        } else {
            k
        };
        run_start.push(start);
        previous = Some(candle);
    }
    run_start
}

/// For each index `e`, the least of the `w` values ending at `e`: a monotone
/// deque of at most `w + 1` indices, amortised O(1) per value (the index
/// build it is part of is row C-F5-06 of `crates/runner/benches/fib.rs`).
fn window_minima(lows: &[i64], w: usize) -> Vec<i64> {
    let mut win_min = vec![i64::MAX; lows.len()];
    let mut deque: VecDeque<usize> = VecDeque::with_capacity(w.saturating_add(1));
    for (e, low) in lows.iter().enumerate() {
        while deque
            .back()
            .is_some_and(|b| lows.get(*b).is_some_and(|l| l >= low))
        {
            deque.pop_back();
        }
        deque.push_back(e);
        while deque.front().is_some_and(|f| f.saturating_add(w) <= e) {
            deque.pop_front();
        }
        if let (Some(slot), Some(front)) = (win_min.get_mut(e), deque.front()) {
            *slot = lows.get(*front).copied().unwrap_or(i64::MAX);
        }
    }
    win_min
}

/// The best buy-side and worst sell-side prices of the usable bars from each
/// index on.
fn suffixes(bars: &[SideBar]) -> (Vec<i64>, Vec<i64>) {
    let mut best_from = vec![i64::MIN; bars.len().saturating_add(1)];
    let mut worst_from = vec![i64::MAX; bars.len().saturating_add(1)];
    let mut best = i64::MIN;
    let mut worst = i64::MAX;
    for (k, bar) in bars.iter().enumerate().rev() {
        if bar.usable() {
            best = best.max(bar.hi_raw);
            worst = worst.min(bar.lo_raw);
        }
        if let Some(slot) = best_from.get_mut(k) {
            *slot = best;
        }
        if let Some(slot) = worst_from.get_mut(k) {
            *slot = worst;
        }
    }
    (best_from, worst_from)
}

/// The forced window's summary for one side.
fn forced_window(bars: &[SideBar], deep: bool) -> Forced {
    let mut forced = Forced {
        bars: 0,
        capable: 0,
        capacity: 0,
        worst_raw: i64::MAX,
        last_second: 0,
        minute: None,
        best_raw: i64::MIN,
        best_first: 0,
        best_last: 0,
        low_raw: i64::MAX,
    };
    for bar in bars {
        if u64::from(bar.second) < SESSION_FORCED_WINDOW_START_SECS || !bar.usable() {
            continue;
        }
        forced.bars += 1;
        if bar.hi_raw > forced.best_raw {
            forced.best_raw = bar.hi_raw;
            forced.best_first = bar.second;
            forced.best_last = bar.second;
        } else if bar.hi_raw == forced.best_raw {
            forced.best_last = bar.second;
        }
        forced.low_raw = forced.low_raw.min(bar.lo_raw);
        if deep {
            if bar.is(Bit::ExitOk) {
                forced.capable += 1;
                forced.capacity += u128::from(bar.volume);
                forced.worst_raw = forced.worst_raw.min(bar.lo_raw);
                forced.last_second = bar.second;
            }
        } else if u64::from(bar.second) == SESSION_FORCED_WINDOW_START_SECS
            && !bar.is(Bit::ExitLocked)
        {
            forced.minute = Some(bar.lo_raw);
        }
    }
    forced
}

/// Each bar's second, after checking order, uniqueness, one IST day and the
/// grid.
fn checked_bars(bars: &[Candle], resolution: Resolution) -> Result<Vec<u32>, WalkRefusal> {
    let mut seconds = Vec::with_capacity(bars.len());
    let mut day = None;
    let mut previous: Option<i64> = None;
    for (at, bar) in bars.iter().enumerate() {
        if previous.is_some_and(|p| p >= bar.ts_micros) {
            return Err(WalkRefusal::BarsNotSortedUnique { at });
        }
        if !on_grid(bar.ts_micros, resolution) {
            return Err(WalkRefusal::BarOffGrid { at });
        }
        let (d, second) = clock(bar.ts_micros);
        if day.is_some_and(|first| first != d) {
            return Err(WalkRefusal::BarsSpanDays);
        }
        day = Some(d);
        previous = Some(bar.ts_micros);
        seconds.push(second);
    }
    Ok(seconds)
}

/// The halts, checked ascending, non-empty and disjoint.
fn checked_halts(halts: &[HaltRange]) -> Result<Vec<HaltRange>, WalkRefusal> {
    let mut previous_end = None;
    for halt in halts {
        if halt.start_second >= halt.end_second
            || previous_end.is_some_and(|end| end > halt.start_second)
        {
            return Err(WalkRefusal::HaltsInvalid);
        }
        previous_end = Some(halt.end_second);
    }
    Ok(halts.to_vec())
}

/// Each minute of the day's Zerodha low and high, when it traded (a minute
/// with volume above zero that passes `Candle::check_evaluable`).
fn zerodha_presence(minutes: &[Candle]) -> Result<Vec<Option<(i64, i64)>>, WalkRefusal> {
    let mut present = vec![None; minutes_per_day()];
    for (at, minute) in minutes.iter().enumerate() {
        if !on_grid(minute.ts_micros, Resolution::Minute) {
            return Err(WalkRefusal::BarOffGrid { at });
        }
        if minute.volume <= 0 || minute.check_evaluable().is_err() {
            continue;
        }
        if let Some(slot) = present.get_mut(minute_of(clock(minute.ts_micros).1)) {
            *slot = Some((minute.low, minute.high));
        }
    }
    Ok(present)
}

/// Each trail list's timeframe, after checking a positive width and candles
/// ascending by close.
fn checked_trails(trails: &[TrailCandles<'_>]) -> Result<Vec<u16>, WalkRefusal> {
    let mut tfs = Vec::with_capacity(trails.len());
    for (list, trail) in trails.iter().enumerate() {
        let mut previous: Option<i64> = None;
        let refused = WalkRefusal::TrailListInvalid { list };
        if trail.tf_minutes == 0 {
            return Err(refused);
        }
        for candle in trail.candles {
            if previous.is_some_and(|p| p >= candle.close_ts_micros)
                || candle.candle.ts_micros >= candle.close_ts_micros
            {
                return Err(refused);
            }
            previous = Some(candle.close_ts_micros);
        }
        tfs.push(trail.tf_minutes);
    }
    Ok(tfs)
}
