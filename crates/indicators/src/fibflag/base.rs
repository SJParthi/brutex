//! The base detector: micro bases of two and three candles and rests, on one
//! closed candle of one width of one stock-day at a time (FIB-3; D-6340,
//! D-6342, D-6344, D-6345, D-6347).
//!
//! # Bases anywhere, and never inside one candle (D-6340)
//!
//! At each closed candle `t` the bases that END at `t` are emitted (candle
//! `t + 1` is the trigger, FIB-4's), in both directions, wherever they form: on
//! a level, between levels or at the day extreme (operator rule c). Per
//! direction, in this order:
//!
//! * `micro2`, candles `t - 1, t`, and `micro3`, candles `t - 2 .. t` (the
//!   operator's pattern sizes, `vocab::fib_cube::MICRO2_CANDLES` and
//!   `MICRO3_CANDLES`);
//! * the maximal REST: the longest suffix ending at `t` whose candles share a
//!   common price overlap (the highest low at most the lowest high), of at
//!   least two candles;
//! * one rest suffix for every generated rest length of the direction strictly
//!   between three candles and the maximal rest's length (17:15 IST item 2).
//!
//! A single candle is never a base, so the high and the pullback inside one
//! candle are never a retracement, and the ladder a base is measured against is
//! the day's extremes after `t` folded: finished candles only (SEQUENTIAL
//! CANDLES).
//!
//! # Contiguity
//!
//! Every base needs its candles contiguous on the width's 09:15-anchored grid.
//! The caller passes, with each candle, the close instant the next candle has on
//! that grid within the calendar's session; a candle that arrives with another
//! close instant opens a hole: a micro base across it is refused `gap_in_base`
//! and the rest restarts.
//!
//! # Cost (D-6342, D-6344; `docs/06-limits.md` L-FIB3-01 to L-FIB3-04)
//!
//! The maximal rest keeps two monotonic deques (the highest low, the lowest
//! high) over the ring with a start pointer that only advances: adding a candle
//! can only shrink the overlap, so the valid start is monotone. Each rest
//! length keeps its own deques of the highest high, lowest low, highest and
//! lowest close and deepest poke. Every sum is a prefix sum by day index, so a
//! measurement is O(1) as the start moves. Amortised O(1) per candle per
//! length; the worst single candle is O(session candles), when the start jumps
//! or the touches recount, and that is bounded by the session.
//!
//! Touches (FM-F022) are a pure function of the window: the candles whose high
//! touches the window's highest high at the direction's tolerance, one count per
//! generated level `q`. A count slides with the window while that highest high
//! stands and is recounted over the window when it changes.
//!
//! # No allocation per day (FM-F023)
//!
//! Every buffer is allocated once, in [`Detector::new`], at the span's longest
//! session ([`DetectorParams::ring_capacity`]) and the run's generated list
//! sizes, and cleared each day. A longer session is refused by name.
//!
//! # Learned values are per direction (17:15 IST item 4)
//!
//! The rest lengths and the touch tolerances come from each direction's own
//! past ([`DirectionDay`]); only the geometry is mirrored, the short view being
//! the long view's code on negated prices (`super::measure`).

use vocab::fib_cube::{
    BASE_KIND_ROW, MICRO2_CANDLES, MICRO3_CANDLES, SHAPE_HALVES, SHAPE_THIRDS, max_width_minutes,
};

use super::extremes::{DayExtremes, Ladder, Prior, WithheldExtremes};
use super::measure::{self, Bar, Ratio, Window};
use super::shape;
use super::types::{
    BaseKind, BaseSnapshot, DetectorParams, FibCandle, Refusal, RefusalCounts, TradeDir,
    ZeroWidthReading,
};
use crate::ist_day;

/// How many kinds of base there are: the most bases of a direction that end
/// at a candle beside its generated rest lengths (two micro bases and the
/// maximal rest).
const KINDS: usize = BASE_KIND_ROW.labels.len();

/// One direction's generated values for one day (17:15 IST items 1, 2 and 4),
/// each from that direction's own committed past.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DirectionDay<'a> {
    /// The generated rest lengths, in candles, strictly ascending: each
    /// `base_length` cut converted through the direction's own-past
    /// histogram, once per day.
    pub rest_lengths: &'a [u16],
    /// The touch tolerance at each generated level `q`, in basis points of
    /// the edge (FM-F017); `None` when the direction's step measure has no
    /// committed observation (FM-F015): its bases are counted
    /// `no_touch_tolerance` and carry no touch count.
    pub touch_bp: Option<&'a [u64]>,
}

/// What one stock-day of one width is begun with: every value from the
/// calendar or the stock's own past, none typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DayInput<'a> {
    /// The IST day ([`crate::ist_day`]).
    pub ist_day: i64,
    /// This width's candles in today's session, from the calendar.
    pub session_candles: u16,
    /// The observed price tick of the stock, in paisa (FIB-2's running gcd).
    pub observed_tick: i64,
    /// The long direction's generated values.
    pub long: DirectionDay<'a>,
    /// The short direction's generated values.
    pub short: DirectionDay<'a>,
}

/// One closed candle as the caller hands it over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandleInput {
    /// The candle, folded from Zerodha one-minute bars.
    pub candle: FibCandle,
    /// The close instant the NEXT candle of this width has on the
    /// 09:15-anchored grid within the calendar's session: a candle arriving
    /// with another close instant opens a hole.
    pub next_close_ts_micros: i64,
    /// A minute of the candle was marked suspect (FM-F096).
    pub suspect_print: bool,
}

/// What the detector counted beside its refusals: plain fields, no static and
/// no atomic (gate 17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DetectorCounts {
    /// Snapshots emitted.
    pub snapshots: u64,
    /// Emitted bases of zero width, flagged (FM-F024).
    pub zero_width_bases: u64,
    /// Emitted bases whose squeeze divided a positive range by zero.
    pub ratio_over_zero: u64,
    /// Emitted bases whose ladder took an extreme from a withheld bucket.
    pub extreme_from_withheld_bucket: u64,
    /// Emitted bases whose ladder took an extreme from a suspect print.
    pub frame_from_suspect_print: u64,
    /// Extremes-only updates folded (FM-F025).
    pub withheld_updates: u64,
    /// Holes that restarted the rest.
    pub rest_resets: u64,
    /// Touch counts recounted over their window because its highest high
    /// changed (FM-F022).
    pub touch_recounts: u64,
    /// Candles visited by those recounts.
    pub touch_recount_candles: u64,
}

impl DetectorCounts {
    /// Every count at zero.
    const fn zero() -> Self {
        Self {
            snapshots: 0,
            zero_width_bases: 0,
            ratio_over_zero: 0,
            extreme_from_withheld_bucket: 0,
            frame_from_suspect_print: 0,
            withheld_updates: 0,
            rest_resets: 0,
            touch_recounts: 0,
            touch_recount_candles: 0,
        }
    }
}

/// One accepted candle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rec {
    /// Open, high, low and close, in paisa.
    open: i64,
    /// The high.
    high: i64,
    /// The low.
    low: i64,
    /// The close.
    close: i64,
    /// The close instant.
    close_ts: i64,
    /// Contiguous with the candle before it on the grid.
    linked: bool,
    /// The depth of a poke below the day low before it that closed back
    /// above that low, else 0.
    poke_long: i64,
    /// The depth of a poke above the day high before it that closed back
    /// below that high, else 0.
    poke_short: i64,
}

/// Running sums, exclusive prefix by day index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sums {
    /// Ranges.
    range: i128,
    /// Bodies.
    body: i128,
    /// Highs.
    high: i128,
    /// Lows.
    low: i128,
    /// Closes.
    close: i128,
    /// `x` times the high.
    x_high: i128,
    /// `x` times the low.
    x_low: i128,
    /// `x` times the close.
    x_close: i128,
    /// `x` squared times the close.
    xx_close: i128,
    /// Candles inside the linked candle before.
    inside: u32,
    /// Candles with a higher low than the linked candle before.
    rise_low: u32,
    /// Candles with a lower high than the linked candle before.
    fall_high: u32,
}

/// No candle.
const NO_SUMS: Sums = Sums {
    range: 0,
    body: 0,
    high: 0,
    low: 0,
    close: 0,
    x_high: 0,
    x_low: 0,
    x_close: 0,
    xx_close: 0,
    inside: 0,
    rise_low: 0,
    fall_high: 0,
};

impl Sums {
    /// The sums of the candles after `earlier` up to `self`.
    fn minus(self, earlier: Self) -> Self {
        Self {
            range: self.range - earlier.range,
            body: self.body - earlier.body,
            high: self.high - earlier.high,
            low: self.low - earlier.low,
            close: self.close - earlier.close,
            x_high: self.x_high - earlier.x_high,
            x_low: self.x_low - earlier.x_low,
            x_close: self.x_close - earlier.x_close,
            xx_close: self.xx_close - earlier.xx_close,
            inside: self.inside.saturating_sub(earlier.inside),
            rise_low: self.rise_low.saturating_sub(earlier.rise_low),
            fall_high: self.fall_high.saturating_sub(earlier.fall_high),
        }
    }
}

/// Exclusive prefix sums of the day index `x` to the fourth power: the same
/// every day, computed once (no closed form, which would type its divisors).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Powers {
    /// `x`.
    x: i128,
    /// `x` squared.
    xx: i128,
    /// `x` cubed.
    xxx: i128,
    /// `x` to the fourth.
    xxxx: i128,
}

/// No index.
const NO_POWERS: Powers = Powers {
    x: 0,
    xx: 0,
    xxx: 0,
    xxxx: 0,
};

/// A monotonic deque of `(day index, key)` in a buffer allocated once at the
/// ring's capacity. A candle is pushed once per clear and an entry before the
/// front stays in its slot until the clear, so the buffer never grows past the
/// capacity and never reallocates.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Monotone {
    /// The entries; those before `head` have expired.
    slots: Vec<(u32, i64)>,
    /// The front.
    head: usize,
    /// The front is the largest key (else the smallest).
    largest: bool,
}

impl Monotone {
    /// An empty deque with room for `capacity` candles.
    fn new(capacity: usize, largest: bool) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            head: 0,
            largest,
        }
    }

    /// Empties it without freeing.
    fn clear(&mut self) {
        self.slots.clear();
        self.head = 0;
    }

    /// Pushes a key, dropping every live key behind it that it dominates.
    /// Returns the slots touched.
    fn push(&mut self, index: u32, key: i64) -> u64 {
        let mut work = 1;
        while let Some(&(_, back)) = self.slots.last().filter(|_| self.slots.len() > self.head) {
            let dominated = if self.largest {
                back <= key
            } else {
                back >= key
            };
            if !dominated {
                break;
            }
            self.slots.pop();
            work += 1;
        }
        self.slots.push((index, key));
        work
    }

    /// Drops every front entry whose index is before `start`. Returns the
    /// entries dropped.
    fn expire(&mut self, start: u32) -> u64 {
        let mut work = 0;
        while let Some(&(index, _)) = self.slots.get(self.head) {
            if index >= start {
                break;
            }
            self.head += 1;
            work += 1;
        }
        work
    }

    /// The front key, the window's largest (or smallest); `alone` for an
    /// empty deque, which is the key of a window holding only that candle.
    fn front_or(&self, alone: i64) -> i64 {
        self.slots.get(self.head).map_or(alone, |&(_, key)| key)
    }

    /// The front key, when there is one.
    fn front(&self) -> Option<i64> {
        self.slots.get(self.head).map(|&(_, key)| key)
    }

    /// The bytes its buffer holds.
    fn bytes(&self) -> usize {
        self.slots.capacity() * core::mem::size_of::<(u32, i64)>()
    }
}

/// One direction's touch counts over one lane's window.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Touch {
    /// The counts match `edge` and `start`.
    valid: bool,
    /// The window's highest view high they were counted against.
    edge: i64,
    /// The window's first index they cover.
    start: usize,
    /// One count per generated level `q`.
    counts: Vec<u16>,
}

impl Touch {
    /// No count, with room for `levels` levels.
    fn new(levels: usize) -> Self {
        Self {
            valid: false,
            edge: 0,
            start: 0,
            counts: Vec::with_capacity(levels),
        }
    }

    /// No count, `levels` zeroes, within the room it was made with.
    fn reset(&mut self, levels: usize) {
        self.valid = false;
        self.counts.clear();
        self.counts.resize(levels, 0);
    }
}

/// One rest window of the day: the maximal rest, or one generated length.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lane {
    /// The generated length, in candles (unused by the maximal rest).
    length: usize,
    /// The long direction emits this window.
    long_uses: bool,
    /// The short direction emits this window.
    short_uses: bool,
    /// The highest high.
    high: Monotone,
    /// The lowest low.
    low: Monotone,
    /// The highest close.
    close_high: Monotone,
    /// The lowest close.
    close_low: Monotone,
    /// The deepest long-view poke.
    poke_long: Monotone,
    /// The deepest short-view poke.
    poke_short: Monotone,
    /// The long view's touches.
    long: Touch,
    /// The short view's touches.
    short: Touch,
}

impl Lane {
    /// An unused lane over a ring of `capacity`.
    fn new(capacity: usize, long_levels: usize, short_levels: usize) -> Self {
        Self {
            length: 0,
            long_uses: false,
            short_uses: false,
            high: Monotone::new(capacity, true),
            low: Monotone::new(capacity, false),
            close_high: Monotone::new(capacity, true),
            close_low: Monotone::new(capacity, false),
            poke_long: Monotone::new(capacity, true),
            poke_short: Monotone::new(capacity, true),
            long: Touch::new(long_levels),
            short: Touch::new(short_levels),
        }
    }

    /// Sets today's length and users and zeroes its counts.
    fn reset(&mut self, length: usize, uses: (bool, bool), levels: (usize, usize)) {
        self.clear();
        self.length = length;
        (self.long_uses, self.short_uses) = uses;
        self.long.reset(levels.0);
        self.short.reset(levels.1);
    }

    /// Empties every deque and invalidates the touches.
    fn clear(&mut self) {
        self.high.clear();
        self.low.clear();
        self.close_high.clear();
        self.close_low.clear();
        self.poke_long.clear();
        self.poke_short.clear();
        self.long.valid = false;
        self.short.valid = false;
    }

    /// Pushes candle `rec` at `index` and expires everything before `start`.
    fn push(&mut self, index: u32, rec: &Rec, start: u32) -> u64 {
        self.high.push(index, rec.high)
            + self.low.push(index, rec.low)
            + self.close_high.push(index, rec.close)
            + self.close_low.push(index, rec.close)
            + self.poke_long.push(index, rec.poke_long)
            + self.poke_short.push(index, rec.poke_short)
            + self.high.expire(start)
            + self.low.expire(start)
            + self.close_high.expire(start)
            + self.close_low.expire(start)
            + self.poke_long.expire(start)
            + self.poke_short.expire(start)
    }

    /// The raw extremes of its window, which ends at candle `last`.
    fn extremes(&self, last: &Rec) -> Extremes {
        Extremes {
            high: self.high.front_or(last.high),
            low: self.low.front_or(last.low),
            close_high: self.close_high.front_or(last.close),
            close_low: self.close_low.front_or(last.close),
            poke_long: self.poke_long.front_or(last.poke_long),
            poke_short: self.poke_short.front_or(last.poke_short),
        }
    }

    /// Whether `direction` emits this window.
    const fn uses(&self, direction: TradeDir) -> bool {
        match direction {
            TradeDir::Long => self.long_uses,
            TradeDir::Short => self.short_uses,
        }
    }

    /// `direction`'s touch counts.
    const fn touch(&self, direction: TradeDir) -> &Touch {
        match direction {
            TradeDir::Long => &self.long,
            TradeDir::Short => &self.short,
        }
    }

    /// The bytes its buffers hold.
    fn bytes(&self) -> usize {
        self.high.bytes()
            + self.low.bytes()
            + self.close_high.bytes()
            + self.close_low.bytes()
            + self.poke_long.bytes()
            + self.poke_short.bytes()
            + (self.long.counts.capacity() + self.short.counts.capacity())
                * core::mem::size_of::<u16>()
    }
}

/// A window's raw extremes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Extremes {
    /// The highest high.
    high: i64,
    /// The lowest low.
    low: i64,
    /// The highest close.
    close_high: i64,
    /// The lowest close.
    close_low: i64,
    /// The deepest long-view poke.
    poke_long: i64,
    /// The deepest short-view poke.
    poke_short: i64,
}

impl Extremes {
    /// The extremes of a few candles, read directly.
    fn of(recs: &[Rec]) -> Self {
        recs.iter().fold(
            Self {
                high: i64::MIN,
                low: i64::MAX,
                close_high: i64::MIN,
                close_low: i64::MAX,
                poke_long: 0,
                poke_short: 0,
            },
            |e, r| Self {
                high: e.high.max(r.high),
                low: e.low.min(r.low),
                close_high: e.close_high.max(r.close),
                close_low: e.close_low.min(r.close),
                poke_long: e.poke_long.max(r.poke_long),
                poke_short: e.poke_short.max(r.poke_short),
            },
        )
    }
}

/// A view's high of a candle: the high, or minus the low for a short.
const fn view_high(rec: &Rec, direction: TradeDir) -> i64 {
    match direction {
        TradeDir::Long => rec.high,
        TradeDir::Short => -rec.low,
    }
}

/// A candle in a direction's view.
const fn view_bar(rec: &Rec, direction: TradeDir) -> Bar {
    match direction {
        TradeDir::Long => Bar {
            high: rec.high,
            low: rec.low,
        },
        TradeDir::Short => Bar {
            high: -rec.low,
            low: -rec.high,
        },
    }
}

/// Adds (or removes) the touches of `recs` against `edge` to each level's
/// count.
fn count_touches<'c>(
    counts: impl Iterator<Item = &'c mut u16>,
    recs: &[Rec],
    direction: TradeDir,
    edge: i64,
    tolerance: &[u64],
    add: bool,
) {
    for (count, tolerance) in counts.zip(tolerance) {
        let hits = recs
            .iter()
            .filter(|rec| measure::touches(view_high(rec, direction), edge, *tolerance))
            .count();
        let hits = u16::try_from(hits).unwrap_or(u16::MAX);
        *count = if add {
            count.saturating_add(hits)
        } else {
            count.saturating_sub(hits)
        };
    }
}

/// Slides or recounts one direction's touches over the window `[start, t]`
/// whose highest view high is `edge`: slid while the edge stands, recounted
/// over the window when it moves (FM-F022). Returns whether it recounted and
/// the candles it visited.
fn update_touch(
    touch: &mut Touch,
    recs: &[Rec],
    direction: TradeDir,
    tolerance: &[u64],
    edge: i64,
    window: (usize, usize),
) -> (bool, usize) {
    let (start, t) = window;
    let recount = !touch.valid || touch.edge != edge;
    let visited = if recount {
        touch.counts.fill(0);
        let all = recs.get(start..=t).unwrap_or(&[]);
        count_touches(
            touch.counts.iter_mut(),
            all,
            direction,
            edge,
            tolerance,
            true,
        );
        all.len()
    } else {
        let leaving = recs.get(touch.start..start).unwrap_or(&[]);
        count_touches(
            touch.counts.iter_mut(),
            leaving,
            direction,
            edge,
            tolerance,
            false,
        );
        let entering = recs.get(t..=t).unwrap_or(&[]);
        count_touches(
            touch.counts.iter_mut(),
            entering,
            direction,
            edge,
            tolerance,
            true,
        );
        leaving.len() + entering.len()
    };
    touch.valid = true;
    touch.edge = edge;
    touch.start = start;
    (recount, visited)
}

/// Candle `given` as the tape keeps it: its prices, whether it is linked to
/// the candle before, and its pokes past the day's extremes before it.
fn record(given: &FibCandle, prior: Option<Prior>, linked: bool) -> Rec {
    let c = given.candle;
    let poke_long = prior.map_or(0, |p| {
        if c.low < p.low && c.close > p.low {
            p.low - c.low
        } else {
            0
        }
    });
    let poke_short = prior.map_or(0, |p| {
        if c.high > p.high && c.close < p.high {
            c.high - p.high
        } else {
            0
        }
    });
    Rec {
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        close_ts: given.close_ts_micros,
        linked,
        poke_long,
        poke_short,
    }
}

/// One direction's values for the begun day.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectionState {
    /// The touch tolerance of each level `q`, in basis points.
    tolerance: Vec<u64>,
    /// The direction has a touch tolerance today.
    known: bool,
}

/// The begun day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Day {
    /// The IST day.
    number: i64,
    /// This width's candles in the session.
    session: usize,
    /// The observed tick.
    tick: i64,
}

/// The day's candles and every running sum over them: read-only while a
/// candle's bases are emitted.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tape {
    /// The accepted candles.
    recs: Vec<Rec>,
    /// The exclusive prefix sums by day index.
    prefix: Vec<Sums>,
    /// The exclusive prefix sums of the powers of the day index.
    powers: Box<[Powers]>,
    /// The last index that made the day high at its close.
    made_high: Option<usize>,
    /// The last index that made the day low at its close.
    made_low: Option<usize>,
    /// The shape halves and thirds.
    parts: (usize, usize),
    /// The three-candle micro size: a window that long has a middle.
    micro3: usize,
}

impl Tape {
    /// Candles `s..=t`.
    fn slice(&self, s: usize, t: usize) -> &[Rec] {
        self.recs.get(s..=t).unwrap_or(&[])
    }

    /// Appends candle `rec`'s prefix sums.
    fn push(&mut self, rec: Rec, previous: Option<&Rec>, index: u32) {
        let last = self.prefix.last().copied().unwrap_or(NO_SUMS);
        let x = i128::from(index);
        let (open, high, low, close) = (
            i128::from(rec.open),
            i128::from(rec.high),
            i128::from(rec.low),
            i128::from(rec.close),
        );
        let flag =
            |test: fn(&Rec, &Rec) -> bool| u32::from(previous.is_some_and(|p| test(&rec, p)));
        self.prefix.push(Sums {
            range: last.range + (high - low),
            body: last.body + (close - open).abs(),
            high: last.high + high,
            low: last.low + low,
            close: last.close + close,
            x_high: last.x_high + x * high,
            x_low: last.x_low + x * low,
            x_close: last.x_close + x * close,
            xx_close: last.xx_close + x * x * close,
            inside: last.inside + flag(|r, p| r.high <= p.high && r.low >= p.low),
            rise_low: last.rise_low + flag(|r, p| r.low > p.low),
            fall_high: last.fall_high + flag(|r, p| r.high < p.high),
        });
        self.recs.push(rec);
    }

    /// The aggregates of day indices `s..=t` in `direction`'s view.
    fn window(&self, direction: TradeDir, s: usize, t: usize, ext: &Extremes) -> Window {
        let at = |i: usize| self.prefix.get(i).copied().unwrap_or(NO_SUMS);
        let (halves, thirds) = self.parts;
        let n = t + 1 - s;
        let half = n / halves;
        let third = n / thirds;
        let sums = at(t + 1).minus(at(s));
        let pairs = at(t + 1).minus(at(s + 1));
        let powers = {
            let p = |i: usize| self.powers.get(i).copied().unwrap_or(NO_POWERS);
            let (end, begin) = (p(t + 1), p(s));
            Powers {
                x: end.x - begin.x,
                xx: end.xx - begin.xx,
                xxx: end.xxx - begin.xxx,
                xxxx: end.xxxx - begin.xxxx,
            }
        };
        let range_first_half = at(s + half).range - at(s).range;
        let close_first = at(s + third).close - at(s).close;
        let close_last = at(t + 1).close - at(t + 1 - third).close;
        let holds_high = self.made_high.is_some_and(|i| i >= s);
        let holds_low = self.made_low.is_some_and(|i| i >= s);
        let bar = |i: usize| {
            self.recs
                .get(i)
                .map_or(Bar { high: 0, low: 0 }, |r| view_bar(r, direction))
        };
        let long = Window {
            candles: u32::try_from(n).unwrap_or(u32::MAX),
            first_index: u32::try_from(s).unwrap_or(u32::MAX),
            high: ext.high,
            low: ext.low,
            close_high: ext.close_high,
            close_low: ext.close_low,
            sum_high: sums.high,
            sum_low: sums.low,
            sum_close: sums.close,
            sum_x_high: sums.x_high,
            sum_x_low: sums.x_low,
            sum_x_close: sums.x_close,
            sum_xx_close: sums.xx_close,
            sum_x: powers.x,
            sum_xx: powers.xx,
            sum_xxx: powers.xxx,
            sum_xxxx: powers.xxxx,
            sum_range: sums.range,
            sum_body: sums.body,
            range_first_half,
            range_last_half: sums.range - range_first_half,
            close_first_third: close_first,
            close_last_third: close_last,
            inside_pairs: pairs.inside,
            rising_lows: pairs.rise_low,
            poke: ext.poke_long,
            holds_level0: holds_high,
            holds_level1: holds_low,
            first: bar(s),
            middle: (n == self.micro3).then(|| bar(s + 1)),
            last: bar(t),
        };
        match direction {
            TradeDir::Long => long,
            TradeDir::Short => Window {
                high: -ext.low,
                low: -ext.high,
                close_high: -ext.close_low,
                close_low: -ext.close_high,
                sum_high: -sums.low,
                sum_low: -sums.high,
                sum_close: -sums.close,
                sum_x_high: -sums.x_low,
                sum_x_low: -sums.x_high,
                sum_x_close: -sums.x_close,
                sum_xx_close: -sums.xx_close,
                close_first_third: -close_first,
                close_last_third: -close_last,
                rising_lows: pairs.fall_high,
                poke: ext.poke_short,
                holds_level0: holds_low,
                holds_level1: holds_high,
                ..long
            },
        }
    }

    /// The bytes its buffers hold.
    fn bytes(&self) -> usize {
        self.recs.capacity() * core::mem::size_of::<Rec>()
            + self.prefix.capacity() * core::mem::size_of::<Sums>()
            + core::mem::size_of_val(&*self.powers)
    }
}

/// What one candle's bases are measured with.
struct Context<'a> {
    /// The day's candles and sums.
    tape: &'a Tape,
    /// The ladder after the candle folded.
    ladder: &'a Ladder,
    /// The direction being emitted.
    direction: TradeDir,
    /// That direction's values today.
    state: &'a DirectionState,
    /// What a zero-width base gets.
    zero_width: ZeroWidthReading,
}

/// Where a base's touch counts come from.
#[derive(Clone, Copy, Debug)]
enum Counted<'a> {
    /// Counted over its few candles directly (a micro base).
    Direct,
    /// A rest lane's running counts.
    Running(&'a Touch),
}

/// What the detector emits and counts.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Out {
    /// The candle's snapshots.
    snapshots: Vec<BaseSnapshot>,
    /// Their touch counts, end to end.
    touches: Vec<u16>,
    /// Each snapshot's slice of `touches`.
    spans: Vec<(usize, usize)>,
    /// Refusals, by reason.
    refusals: RefusalCounts,
    /// The other counts.
    counts: DetectorCounts,
}

impl Out {
    /// Measures and emits one base over day indices `span`.
    fn emit(
        &mut self,
        ctx: &Context<'_>,
        kind: BaseKind,
        span: (usize, usize),
        ext: &Extremes,
        counted: Counted<'_>,
    ) {
        let (s, t) = span;
        let direction = ctx.direction;
        let window = ctx.tape.window(direction, s, t, ext);
        let measures = measure::measure(&window, ctx.ladder, direction);
        if measures.zero_width() {
            if ctx.zero_width == ZeroWidthReading::Refuse {
                self.refusals.record(Refusal::ZeroWidthBase);
                return;
            }
            self.counts.zero_width_bases += 1;
        }
        if measures.squeeze == Ratio::OverZero {
            self.counts.ratio_over_zero += 1;
        }
        let stamp = ctx.ladder.stamp();
        self.counts.extreme_from_withheld_bucket += u64::from(stamp.extreme_from_withheld_bucket);
        self.counts.frame_from_suspect_print += u64::from(stamp.frame_from_suspect_print);
        let known = ctx.state.known;
        let from = self.touches.len();
        if known {
            match counted {
                Counted::Direct => {
                    self.touches.resize(from + ctx.state.tolerance.len(), 0);
                    count_touches(
                        self.touches.iter_mut().skip(from),
                        ctx.tape.slice(s, t),
                        direction,
                        window.high,
                        &ctx.state.tolerance,
                        true,
                    );
                }
                Counted::Running(touch) => self.touches.extend_from_slice(&touch.counts),
            }
        } else {
            self.refusals.record(Refusal::NoTouchTolerance);
        }
        self.spans.push((from, self.touches.len()));
        let close_at = |i: usize| ctx.tape.recs.get(i).map_or(0, |r| r.close_ts);
        let index = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
        self.snapshots.push(BaseSnapshot {
            kind,
            direction,
            high: ext.high,
            low: ext.low,
            candles: u16::try_from(t + 1 - s).unwrap_or(u16::MAX),
            first_close_ts_micros: close_at(s),
            last_close_ts_micros: close_at(t),
            first_index: index(s),
            last_index: index(t),
            measures,
            shape: shape::scores(&window, kind),
            ladder: stamp,
            touches_known: known,
        });
        self.counts.snapshots += 1;
    }

    /// The bytes its buffers hold.
    fn bytes(&self) -> usize {
        self.snapshots.capacity() * core::mem::size_of::<BaseSnapshot>()
            + self.touches.capacity() * core::mem::size_of::<u16>()
            + self.spans.capacity() * core::mem::size_of::<(usize, usize)>()
    }
}

/// The bases one candle emitted, borrowed from the detector's buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Emitted<'a> {
    /// The snapshots, long first, then short.
    snapshots: &'a [BaseSnapshot],
    /// Every snapshot's touch counts, end to end.
    touches: &'a [u16],
    /// Each snapshot's slice of `touches`.
    spans: &'a [(usize, usize)],
    /// The ladder the bases were measured against, `None` for `no_ladder`.
    ladder: Option<Ladder>,
}

impl<'a> Emitted<'a> {
    /// The snapshots, long first, then short; per direction `micro2`,
    /// `micro3`, the maximal rest and the generated rest lengths ascending.
    #[must_use]
    pub const fn snapshots(&self) -> &'a [BaseSnapshot] {
        self.snapshots
    }

    /// Snapshot `at`'s touch counts, one per generated level `q` of its
    /// direction; empty when its direction had no tolerance, or no such
    /// snapshot.
    #[must_use]
    pub fn touches(&self, at: usize) -> &'a [u16] {
        self.spans
            .get(at)
            .and_then(|&(from, to)| self.touches.get(from..to))
            .unwrap_or(&[])
    }

    /// The ladder after the candle folded, or `None` when it had none.
    #[must_use]
    pub const fn ladder(&self) -> Option<Ladder> {
        self.ladder
    }
}

/// The base detector of one width of one stock, allocated once per worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detector {
    /// What it runs with.
    params: DetectorParams,
    /// The operator's two micro sizes, in candles.
    micro: (usize, usize),
    /// The begun day.
    day: Option<Day>,
    /// The day's extremes.
    extremes: DayExtremes,
    /// The day's candles and sums.
    tape: Tape,
    /// The highest low of the maximal rest.
    overlap_low: Monotone,
    /// The lowest high of the maximal rest.
    overlap_high: Monotone,
    /// The first index after the last hole.
    segment_start: usize,
    /// The maximal rest's first index.
    rest_start: usize,
    /// The last accepted close instant.
    prev_close: Option<i64>,
    /// The close instant the next candle must have to be contiguous.
    next_close: Option<i64>,
    /// The maximal rest's lane.
    rest: Lane,
    /// One lane per distinct generated rest length today, ascending.
    lanes: Vec<Lane>,
    /// Lanes in use today.
    active: usize,
    /// The long direction's day.
    long: DirectionState,
    /// The short direction's day.
    short: DirectionState,
    /// What it emits and counts.
    out: Out,
    /// The structural work counter: deque slots and candles touched.
    work: u64,
}

impl Detector {
    /// A detector for one width, every buffer allocated here and never again.
    ///
    /// # Errors
    ///
    /// [`Refusal::WidthBeyondTop`] for a width outside `1..=N_max`
    /// (`vocab::fib_cube::max_width_minutes`).
    pub fn new(params: DetectorParams) -> Result<Self, Refusal> {
        let width = u64::from(params.tf_minutes);
        if width == 0 || width > max_width_minutes() {
            return Err(Refusal::WidthBeyondTop);
        }
        let size = |value: u64| usize::try_from(value).unwrap_or(usize::MAX);
        let capacity = usize::from(params.ring_capacity);
        let long_lengths = usize::from(params.long_rest_lengths);
        let short_lengths = usize::from(params.short_rest_lengths);
        let long_levels = usize::from(params.long_touch_levels);
        let short_levels = usize::from(params.short_touch_levels);
        let mut powers = Vec::with_capacity(capacity + 1);
        let mut running = NO_POWERS;
        powers.push(running);
        for x in (0..capacity).map(|x| i128::try_from(x).unwrap_or(i128::MAX)) {
            running = Powers {
                x: running.x + x,
                xx: running.xx + x * x,
                xxx: running.xxx + x * x * x,
                xxxx: running.xxxx + x * x * x * x,
            };
            powers.push(running);
        }
        let lanes = (0..long_lengths + short_lengths)
            .map(|_| Lane::new(capacity, long_levels, short_levels))
            .collect();
        let snapshots = KINDS + long_lengths + KINDS + short_lengths;
        let touches = (KINDS + long_lengths) * long_levels + (KINDS + short_lengths) * short_levels;
        let micro = (size(MICRO2_CANDLES), size(MICRO3_CANDLES));
        Ok(Self {
            params,
            micro,
            day: None,
            extremes: DayExtremes::empty(),
            tape: Tape {
                recs: Vec::with_capacity(capacity),
                prefix: Vec::with_capacity(capacity + 1),
                powers: powers.into_boxed_slice(),
                made_high: None,
                made_low: None,
                parts: (size(SHAPE_HALVES), size(SHAPE_THIRDS)),
                micro3: micro.1,
            },
            overlap_low: Monotone::new(capacity, true),
            overlap_high: Monotone::new(capacity, false),
            segment_start: 0,
            rest_start: 0,
            prev_close: None,
            next_close: None,
            rest: Lane::new(capacity, long_levels, short_levels),
            lanes,
            active: 0,
            long: DirectionState {
                tolerance: Vec::with_capacity(long_levels),
                known: false,
            },
            short: DirectionState {
                tolerance: Vec::with_capacity(short_levels),
                known: false,
            },
            out: Out {
                snapshots: Vec::with_capacity(snapshots),
                touches: Vec::with_capacity(touches),
                spans: Vec::with_capacity(snapshots),
                refusals: RefusalCounts::zero(),
                counts: DetectorCounts::zero(),
            },
            work: 0,
        })
    }

    /// What it runs with.
    #[must_use]
    pub const fn params(&self) -> &DetectorParams {
        &self.params
    }

    /// Refusals, by reason.
    #[must_use]
    pub const fn refusals(&self) -> &RefusalCounts {
        &self.out.refusals
    }

    /// The other counts.
    #[must_use]
    pub const fn counts(&self) -> &DetectorCounts {
        &self.out.counts
    }

    /// The structural work counter (FM-F001): deque slots and candles touched.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// The day's extremes.
    #[must_use]
    pub const fn extremes(&self) -> &DayExtremes {
        &self.extremes
    }

    /// The bytes this detector holds: its own size and every buffer's
    /// allocation, which no day changes.
    #[must_use]
    pub fn state_bytes(&self) -> usize {
        core::mem::size_of::<Self>()
            + self.tape.bytes()
            + self.overlap_low.bytes()
            + self.overlap_high.bytes()
            + self.rest.bytes()
            + self
                .lanes
                .iter()
                .map(|lane| lane.bytes() + core::mem::size_of::<Lane>())
                .sum::<usize>()
            + (self.long.tolerance.capacity() + self.short.tolerance.capacity())
                * core::mem::size_of::<u64>()
            + self.out.bytes()
    }

    /// Counts a refusal and returns it.
    fn refuse(&mut self, why: Refusal) -> Refusal {
        self.out.refusals.record(why);
        why
    }

    /// Begins one stock-day: checks the day's values against what the run
    /// pre-sized and clears every buffer without freeing it. A refused day
    /// leaves the detector un-begun, so its candles are refused
    /// `day_not_begun`.
    ///
    /// # Errors
    ///
    /// [`Refusal::SessionLongerThanRing`], [`Refusal::NoObservedTick`],
    /// [`Refusal::GenerationBeyondCapacity`] or
    /// [`Refusal::RestLengthsNotAscending`], counted.
    pub fn begin_day(&mut self, day: &DayInput<'_>) -> Result<(), Refusal> {
        self.day = None;
        let session = usize::from(day.session_candles);
        let ascending = |lengths: &[u16]| {
            lengths
                .iter()
                .zip(lengths.iter().skip(1))
                .all(|(a, b)| a < b)
        };
        let beyond = |input: &DirectionDay<'_>, lengths: u16, levels: u16| {
            input.rest_lengths.len() > usize::from(lengths)
                || input
                    .touch_bp
                    .is_some_and(|q| q.len() > usize::from(levels))
        };
        let p = self.params;
        let refusal = if session > usize::from(p.ring_capacity) {
            Some(Refusal::SessionLongerThanRing)
        } else if day.observed_tick <= 0 {
            Some(Refusal::NoObservedTick)
        } else if beyond(&day.long, p.long_rest_lengths, p.long_touch_levels)
            || beyond(&day.short, p.short_rest_lengths, p.short_touch_levels)
        {
            Some(Refusal::GenerationBeyondCapacity)
        } else if !ascending(day.long.rest_lengths) || !ascending(day.short.rest_lengths) {
            Some(Refusal::RestLengthsNotAscending)
        } else {
            None
        };
        if let Some(why) = refusal {
            return Err(self.refuse(why));
        }
        self.extremes = DayExtremes::empty();
        self.tape.recs.clear();
        self.tape.prefix.clear();
        self.tape.prefix.push(NO_SUMS);
        self.tape.made_high = None;
        self.tape.made_low = None;
        self.overlap_low.clear();
        self.overlap_high.clear();
        self.segment_start = 0;
        self.rest_start = 0;
        self.prev_close = None;
        self.next_close = None;
        for (state, input) in [(&mut self.long, &day.long), (&mut self.short, &day.short)] {
            state.tolerance.clear();
            state
                .tolerance
                .extend_from_slice(input.touch_bp.unwrap_or(&[]));
            state.known = input.touch_bp.is_some();
        }
        self.assign_lanes(day.long.rest_lengths, day.short.rest_lengths);
        self.day = Some(Day {
            number: day.ist_day,
            session,
            tick: day.observed_tick,
        });
        Ok(())
    }

    /// The maximal rest's lane serves both directions; one lane follows per
    /// distinct generated length of either direction, ascending, naming which
    /// direction emits it.
    fn assign_lanes(&mut self, long: &[u16], short: &[u16]) {
        let levels = (self.long.tolerance.len(), self.short.tolerance.len());
        self.rest.reset(0, (true, true), levels);
        let (mut a, mut b) = (
            long.iter().copied().peekable(),
            short.iter().copied().peekable(),
        );
        let merged = core::iter::from_fn(|| {
            let (x, y) = (a.peek().copied(), b.peek().copied());
            let length = match (x, y) {
                (Some(x), Some(y)) => x.min(y),
                (Some(only), None) | (None, Some(only)) => only,
                (None, None) => return None,
            };
            let uses = (x == Some(length), y == Some(length));
            if uses.0 {
                a.next();
            }
            if uses.1 {
                b.next();
            }
            Some((usize::from(length), uses))
        });
        let mut active = 0;
        for (lane, (length, uses)) in self.lanes.iter_mut().zip(merged) {
            lane.reset(length, uses, levels);
            active += 1;
        }
        self.active = active;
    }

    /// Folds an extremes-only update of a bucket `pull::fold` withheld
    /// (FM-F025): into the day extremes only, never a base candle.
    ///
    /// # Errors
    ///
    /// [`Refusal::DayNotBegun`] for another day, [`Refusal::CorruptCandle`]
    /// for a high below its low or a price not positive; counted.
    pub fn fold_withheld(&mut self, update: &WithheldExtremes) -> Result<(), Refusal> {
        if self
            .day
            .is_none_or(|day| ist_day(update.open_ts_micros) != day.number)
        {
            return Err(self.refuse(Refusal::DayNotBegun));
        }
        if self.extremes.step_withheld(update).is_err() {
            return Err(self.refuse(Refusal::CorruptCandle));
        }
        self.out.counts.withheld_updates += 1;
        Ok(())
    }

    /// Folds one closed candle and emits the bases that end at it.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] of a candle that is folded into nothing: a day not
    /// begun, another width, a partial bucket, missing minutes, another IST
    /// day, out of order, past the day's session, or corrupt. Counted, and
    /// nothing changes.
    pub fn fold(&mut self, input: &CandleInput) -> Result<Emitted<'_>, Refusal> {
        let Some(day) = self.day else {
            return Err(self.refuse(Refusal::DayNotBegun));
        };
        let given = &input.candle;
        if let Some(why) = self.check(day, given) {
            return Err(self.refuse(why));
        }
        let t = self.tape.recs.len();
        let index = u32::try_from(t).unwrap_or(u32::MAX);
        let Ok(prior) = self.extremes.step(
            &given.candle,
            index,
            given.close_ts_micros,
            input.suspect_print,
        ) else {
            return Err(self.refuse(Refusal::CorruptCandle));
        };
        let linked = self.next_close == Some(given.close_ts_micros);
        let rec = record(given, prior, linked);
        if prior.is_none_or(|p| rec.high > p.high) {
            self.tape.made_high = Some(t);
        }
        if prior.is_none_or(|p| rec.low < p.low) {
            self.tape.made_low = Some(t);
        }
        let previous = self.tape.recs.last().filter(|_| linked).copied();
        self.tape.push(rec, previous.as_ref(), index);
        self.prev_close = Some(given.close_ts_micros);
        self.next_close = Some(input.next_close_ts_micros);
        if !linked {
            self.segment_start = t;
            self.rest_start = t;
            self.overlap_low.clear();
            self.overlap_high.clear();
            self.rest.clear();
            for lane in self.lanes.iter_mut().take(self.active) {
                lane.clear();
            }
            if t > 0 {
                self.out.counts.rest_resets += 1;
            }
        }
        self.advance_rest(index, &rec);
        self.advance_lanes(index, &rec, t);
        self.out.snapshots.clear();
        self.out.touches.clear();
        self.out.spans.clear();
        let ladder = self.extremes.ladder(day.tick);
        match ladder {
            Some(ladder) => {
                for direction in [TradeDir::Long, TradeDir::Short] {
                    self.emit_direction(direction, &rec, t, &ladder);
                }
            }
            None => self.out.refusals.record(Refusal::NoLadder),
        }
        Ok(Emitted {
            snapshots: &self.out.snapshots,
            touches: &self.out.touches,
            spans: &self.out.spans,
            ladder,
        })
    }

    /// Why `given` cannot fold into the begun `day`, if it cannot.
    fn check(&self, day: Day, given: &FibCandle) -> Option<Refusal> {
        if given.tf_minutes != self.params.tf_minutes {
            Some(Refusal::WidthMismatch)
        } else if !given.complete {
            Some(Refusal::PartialCandle)
        } else if given.missing_minutes > 0 {
            Some(Refusal::MissingMinutes)
        } else if ist_day(given.candle.ts_micros) != day.number {
            Some(Refusal::DayNotBegun)
        } else if self
            .prev_close
            .is_some_and(|prev| given.close_ts_micros <= prev)
        {
            Some(Refusal::OutOfOrder)
        } else if self.tape.recs.len() >= day.session {
            Some(Refusal::SessionLongerThanRing)
        } else {
            None
        }
    }

    /// Advances the maximal rest's start while its candles share no price.
    fn advance_rest(&mut self, index: u32, rec: &Rec) {
        self.work +=
            self.overlap_low.push(index, rec.low) + self.overlap_high.push(index, rec.high);
        while self.overlap_low.front() > self.overlap_high.front() {
            self.rest_start += 1;
            let start = u32::try_from(self.rest_start).unwrap_or(u32::MAX);
            self.work += self.overlap_low.expire(start) + self.overlap_high.expire(start);
        }
    }

    /// Pushes the candle into every lane and slides or recounts its touches.
    ///
    /// A generated lane keeps live touch counts only while it can be emitted
    /// (its length between three candles and the maximal rest's); otherwise
    /// they are dropped, and its next emittable candle recounts it once. The
    /// rest grows by at most one candle a candle, so that recount is paid for
    /// by the candles it took to grow: amortised O(1).
    fn advance_lanes(&mut self, index: u32, rec: &Rec, t: usize) {
        let Self {
            micro,
            tape,
            rest,
            lanes,
            active,
            long,
            short,
            out,
            work,
            rest_start,
            segment_start,
            ..
        } = self;
        let segment = *segment_start;
        let (micro3, rest_length) = (micro.1, t + 1 - *rest_start);
        let windows = core::iter::once((&mut *rest, *rest_start, true)).chain(
            lanes.iter_mut().take(*active).map(|lane| {
                let start = segment.max((t + 1).saturating_sub(lane.length));
                let emittable = lane.length > micro3 && lane.length < rest_length;
                (lane, start, emittable)
            }),
        );
        for (lane, start, emittable) in windows {
            *work += lane.push(index, rec, u32::try_from(start).unwrap_or(u32::MAX));
            if !emittable {
                lane.long.valid = false;
                lane.short.valid = false;
                continue;
            }
            let edges = [
                (TradeDir::Long, &*long, lane.high.front_or(rec.high)),
                (TradeDir::Short, &*short, -lane.low.front_or(rec.low)),
            ];
            for (direction, state, edge) in edges {
                if lane.uses(direction) && state.known {
                    let touch = match direction {
                        TradeDir::Long => &mut lane.long,
                        TradeDir::Short => &mut lane.short,
                    };
                    let (recounted, visited) = update_touch(
                        touch,
                        &tape.recs,
                        direction,
                        &state.tolerance,
                        edge,
                        (start, t),
                    );
                    let visited = u64::try_from(visited).unwrap_or(u64::MAX);
                    let levels = u64::try_from(state.tolerance.len()).unwrap_or(u64::MAX);
                    *work += visited.saturating_mul(levels);
                    if recounted {
                        out.counts.touch_recounts += 1;
                        out.counts.touch_recount_candles += visited;
                    }
                }
            }
        }
    }

    /// Emits every base of one direction that ends at candle `t`, `last`.
    fn emit_direction(&mut self, direction: TradeDir, last: &Rec, t: usize, ladder: &Ladder) {
        let Self {
            params,
            micro,
            tape,
            rest,
            lanes,
            active,
            long,
            short,
            out,
            rest_start,
            ..
        } = self;
        let ctx = Context {
            tape,
            ladder,
            direction,
            state: match direction {
                TradeDir::Long => long,
                TradeDir::Short => short,
            },
            zero_width: params.zero_width,
        };
        let (micro2, micro3) = *micro;
        for (kind, size) in [(BaseKind::Micro2, micro2), (BaseKind::Micro3, micro3)] {
            let Some(s) = (t + 1).checked_sub(size) else {
                continue;
            };
            let span = tape.slice(s, t);
            if span.iter().skip(1).all(|r| r.linked) {
                out.emit(&ctx, kind, (s, t), &Extremes::of(span), Counted::Direct);
            } else {
                out.refusals.record(Refusal::GapInBase);
            }
        }
        let length = t + 1 - *rest_start;
        if length < micro2 {
            return;
        }
        let maximal = Counted::Running(rest.touch(direction));
        out.emit(
            &ctx,
            BaseKind::Rest,
            (*rest_start, t),
            &rest.extremes(last),
            maximal,
        );
        for lane in lanes.iter().take(*active) {
            if lane.uses(direction) && lane.length > micro3 && lane.length < length {
                let span = (t + 1 - lane.length, t);
                let running = Counted::Running(lane.touch(direction));
                out.emit(&ctx, BaseKind::Rest, span, &lane.extremes(last), running);
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]
mod tests {
    use super::*;
    use crate::{Candle, OI_NULL};

    const MINUTE: i64 = 60_000_000;

    fn params(capacity: u16, lengths: u16, levels: u16) -> DetectorParams {
        DetectorParams {
            tf_minutes: 1,
            generation_version: 7,
            ring_capacity: capacity,
            long_rest_lengths: lengths,
            short_rest_lengths: lengths,
            long_touch_levels: levels,
            short_touch_levels: levels,
            zero_width: ZeroWidthReading::Emit,
        }
    }

    /// The candle at grid slot `slot` of IST day `day` (09:15 IST onwards).
    fn candle(day: i64, slot: i64, ohlc: (i64, i64, i64, i64)) -> CandleInput {
        let open_ts = day * 86_400_000_000 - 19_800_000_000 + 33_300_000_000 + slot * MINUTE;
        let (open, high, low, close) = ohlc;
        CandleInput {
            candle: FibCandle {
                candle: Candle {
                    ts_micros: open_ts,
                    open,
                    high,
                    low,
                    close,
                    volume: 1,
                    open_interest: OI_NULL,
                },
                tf_minutes: 1,
                complete: true,
                missing_minutes: 0,
                close_ts_micros: open_ts + MINUTE,
            },
            next_close_ts_micros: open_ts + MINUTE + MINUTE,
            suspect_print: false,
        }
    }

    /// Every buffer's address and capacity.
    fn fingerprint(d: &Detector) -> Vec<(usize, usize)> {
        let mut out = vec![
            (d.tape.recs.as_ptr().addr(), d.tape.recs.capacity()),
            (d.tape.prefix.as_ptr().addr(), d.tape.prefix.capacity()),
            (d.tape.powers.as_ptr().addr(), d.tape.powers.len()),
            (
                d.overlap_low.slots.as_ptr().addr(),
                d.overlap_low.slots.capacity(),
            ),
            (
                d.overlap_high.slots.as_ptr().addr(),
                d.overlap_high.slots.capacity(),
            ),
            (d.lanes.as_ptr().addr(), d.lanes.capacity()),
            (
                d.long.tolerance.as_ptr().addr(),
                d.long.tolerance.capacity(),
            ),
            (
                d.short.tolerance.as_ptr().addr(),
                d.short.tolerance.capacity(),
            ),
            (d.out.snapshots.as_ptr().addr(), d.out.snapshots.capacity()),
            (d.out.touches.as_ptr().addr(), d.out.touches.capacity()),
            (d.out.spans.as_ptr().addr(), d.out.spans.capacity()),
        ];
        for lane in core::iter::once(&d.rest).chain(&d.lanes) {
            for m in [
                &lane.high,
                &lane.low,
                &lane.close_high,
                &lane.close_low,
                &lane.poke_long,
                &lane.poke_short,
            ] {
                out.push((m.slots.as_ptr().addr(), m.slots.capacity()));
            }
            for touch in [&lane.long, &lane.short] {
                out.push((touch.counts.as_ptr().addr(), touch.counts.capacity()));
            }
        }
        out
    }

    /// FM-F023: after day one, no day moves or grows a buffer: every one is
    /// allocated in `Detector::new` at the run's sizes and cleared each day.
    /// (A counting allocator is the stronger proof; the workspace's one test
    /// root allowed to implement `GlobalAlloc` cannot link this crate, see
    /// `docs/06-limits.md` L-FIB3-06.)
    #[test]
    fn no_allocation_per_day() {
        let mut d = Detector::new(params(48, 4, 3)).unwrap();
        let mut seed: u64 = 0x00F1_B3A1;
        let mut next = |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        let mut first = None;
        for day in 0..=100 {
            let lengths: [Vec<u16>; 2] = [
                (4..12).filter(|_| next(3) == 0).take(4).collect(),
                (4..12).filter(|_| next(3) == 0).take(4).collect(),
            ];
            let tolerance: Vec<u64> = (0..=next(3)).map(|q| q * 7).collect();
            let input = DayInput {
                ist_day: 20_500 + day,
                session_candles: 48,
                observed_tick: 1,
                long: DirectionDay {
                    rest_lengths: &lengths[0],
                    touch_bp: Some(&tolerance),
                },
                short: DirectionDay {
                    rest_lengths: &lengths[1],
                    touch_bp: (day % 4 != 0).then_some(tolerance.as_slice()),
                },
            };
            d.begin_day(&input).unwrap();
            let mut mid: i64 = 98_765;
            let mut slot = 0;
            for i in 0..52 {
                mid += i64::try_from(next(21)).unwrap() - 10;
                let flat = next(5) == 0;
                let (up, down) = if flat {
                    (0, 0)
                } else {
                    (
                        i64::try_from(next(9)).unwrap(),
                        i64::try_from(next(9)).unwrap(),
                    )
                };
                let ohlc = (mid, mid + up + 40 * i64::from(i == 0), mid - down, mid);
                let _ = d.fold(&candle(20_500 + day, slot, ohlc));
                if next(10) == 0 {
                    let _ = d.fold(&candle(20_500 + day, slot + 1, (5, 1, 9, 5)));
                }
                slot += 1 + i64::from(next(12) == 0);
            }
            if day == 0 {
                first = Some(fingerprint(&d));
            } else {
                assert_eq!(Some(fingerprint(&d)), first, "day {day} moved a buffer");
            }
        }
        assert!(d.counts().snapshots > 10_000);
        assert!(d.refusals().get(Refusal::SessionLongerThanRing) > 0);
        assert!(d.refusals().get(Refusal::CorruptCandle) > 0);
        assert!(d.counts().rest_resets > 0);
    }

    #[test]
    fn the_accessors_name_what_was_given_and_done() {
        let p = params(8, 1, 1);
        let mut d = Detector::new(p).unwrap();
        assert_eq!(*d.params(), p);
        assert_eq!(d.work(), 0);
        let day = DayInput {
            ist_day: 20_500,
            session_candles: 8,
            observed_tick: 1,
            long: DirectionDay {
                rest_lengths: &[],
                touch_bp: None,
            },
            short: DirectionDay {
                rest_lengths: &[],
                touch_bp: None,
            },
        };
        d.begin_day(&day).unwrap();
        let first = d
            .fold(&candle(20_500, 0, (7_000, 7_300, 6_900, 7_200)))
            .unwrap();
        assert!(first.ladder().is_some());
        assert!(first.snapshots().is_empty());
        assert!(first.touches(0).is_empty(), "no such snapshot");
        let second = d
            .fold(&candle(20_500, 1, (7_000, 7_100, 6_950, 7_050)))
            .unwrap();
        assert_eq!(
            second.ladder().map(|l| (l.high().price, l.low().price)),
            Some((7_300, 6_900))
        );
        assert!(second.touches(0).is_empty(), "no tolerance: no count");
        assert!(d.work() > 0);
        assert_eq!(d.refusals().get(Refusal::NoTouchTolerance), 4);
        assert!(d.state_bytes() > core::mem::size_of::<Detector>());
        assert_eq!(d.extremes().high().map(|e| e.price), Some(7_300));
    }

    #[test]
    fn a_monotone_deque_keeps_the_window_s_extreme() {
        let mut largest = Monotone::new(8, true);
        let mut smallest = Monotone::new(8, false);
        assert_eq!(largest.front(), None);
        assert_eq!(
            largest.front_or(42),
            42,
            "an empty window is the candle alone"
        );
        for (index, key) in [(0, 5), (1, 3), (2, 4), (3, 4)] {
            largest.push(index, key);
            smallest.push(index, key);
        }
        assert_eq!((largest.front(), smallest.front()), (Some(5), Some(3)));
        assert_eq!(largest.expire(1), 1);
        assert_eq!(largest.front(), Some(4), "the later equal key survives");
        assert_eq!(smallest.expire(2), 1);
        assert_eq!(smallest.front(), Some(4));
        assert_eq!(largest.expire(9), 1);
        assert_eq!(largest.front(), None);
        largest.clear();
        assert_eq!(largest.push(0, 1), 1);
        assert_eq!(largest.bytes(), 8 * core::mem::size_of::<(u32, i64)>());
    }
}
