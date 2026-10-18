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

use super::extremes::{DayExtremes, Ladder, WithheldExtremes};
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

/// A monotonic deque of `(day index, key)` over a buffer of the ring's
/// capacity: a candle is pushed once per clear, so the tail never passes it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Monotone {
    /// The slots.
    slots: Box<[(u32, i64)]>,
    /// The front.
    head: usize,
    /// One past the back.
    tail: usize,
    /// The front is the largest key (else the smallest).
    largest: bool,
}

impl Monotone {
    /// An empty deque of `capacity` slots.
    fn new(capacity: usize, largest: bool) -> Self {
        Self {
            slots: vec![(0, 0); capacity].into_boxed_slice(),
            head: 0,
            tail: 0,
            largest,
        }
    }

    /// Empties it without freeing.
    const fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    /// Pushes a key, dropping every key behind it that it dominates. Returns
    /// the slots touched.
    fn push(&mut self, index: u32, key: i64) -> u64 {
        let mut work = 1;
        while let Some(&(_, back)) = self
            .tail
            .checked_sub(1)
            .filter(|back| *back >= self.head)
            .and_then(|back| self.slots.get(back))
        {
            let dominated = if self.largest {
                back <= key
            } else {
                back >= key
            };
            if !dominated {
                break;
            }
            self.tail -= 1;
            work += 1;
        }
        if let Some(slot) = self.slots.get_mut(self.tail) {
            *slot = (index, key);
            self.tail += 1;
        }
        work
    }

    /// Drops every front entry whose index is before `start`. Returns the
    /// entries dropped.
    fn expire(&mut self, start: u32) -> u64 {
        let mut work = 0;
        while let Some(&(index, _)) = self.slots.get(self.head).filter(|_| self.head < self.tail) {
            if index >= start {
                break;
            }
            self.head += 1;
            work += 1;
        }
        work
    }

    /// The front key: the window's largest (or smallest).
    fn front(&self) -> Option<i64> {
        self.slots
            .get(self.head)
            .filter(|_| self.head < self.tail)
            .map(|&(_, key)| key)
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
}

/// One rest window of the day: the maximal rest, or one generated length.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lane {
    /// `None` for the maximal rest, else the length in candles.
    length: Option<usize>,
    /// The long direction emits this length.
    long_uses: bool,
    /// The short direction emits this length.
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
            length: None,
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

    /// Empties every deque and invalidates the touches.
    const fn clear(&mut self) {
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

    /// The raw extremes of its window.
    fn extremes(&self) -> Extremes {
        Extremes {
            high: self.high.front().unwrap_or(0),
            low: self.low.front().unwrap_or(0),
            close_high: self.close_high.front().unwrap_or(0),
            close_low: self.close_low.front().unwrap_or(0),
            poke_long: self.poke_long.front().unwrap_or(0),
            poke_short: self.poke_short.front().unwrap_or(0),
        }
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

/// Adds (or removes) the touches of `recs` against `edge` to `counts`.
fn count_touches(
    counts: &mut [u16],
    recs: &[Rec],
    direction: TradeDir,
    edge: i64,
    tolerance: &[u64],
    add: bool,
) {
    for rec in recs {
        let high = view_high(rec, direction);
        for (count, tolerance) in counts.iter_mut().zip(tolerance) {
            if measure::touches(high, edge, *tolerance) {
                *count = if add {
                    count.saturating_add(1)
                } else {
                    count.saturating_sub(1)
                };
            }
        }
    }
}

/// Slides or recounts one direction's touches over the window `[start, t]`
/// whose highest view high is `edge`. Returns whether it recounted and the
/// candles it visited.
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
        count_touches(&mut touch.counts, all, direction, edge, tolerance, true);
        all.len()
    } else {
        let leaving = recs.get(touch.start..start).unwrap_or(&[]);
        count_touches(
            &mut touch.counts,
            leaving,
            direction,
            edge,
            tolerance,
            false,
        );
        let entering = recs.get(t..=t).unwrap_or(&[]);
        count_touches(
            &mut touch.counts,
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

/// One direction's values for the begun day.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectionState {
    /// `(length, lane)` of each generated rest length, ascending.
    lengths: Vec<(usize, usize)>,
    /// The touch tolerance of each level `q`, in basis points.
    tolerance: Vec<u64>,
    /// The direction has a touch tolerance today.
    known: bool,
}

impl DirectionState {
    /// Empty, with room for the run's generation.
    fn new(lengths: usize, levels: usize) -> Self {
        Self {
            lengths: Vec::with_capacity(lengths),
            tolerance: Vec::with_capacity(levels),
            known: false,
        }
    }
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

/// Where a base's touch counts come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TouchSource {
    /// Counted over its few candles directly (a micro base).
    Direct,
    /// A rest lane's running counts.
    Lane(usize),
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
    /// The shape halves and thirds.
    parts: (usize, usize),
    /// The begun day.
    day: Option<Day>,
    /// The day's extremes.
    extremes: DayExtremes,
    /// The day's accepted candles.
    recs: Vec<Rec>,
    /// The exclusive prefix sums by day index.
    prefix: Vec<Sums>,
    /// The exclusive prefix sums of the powers of the day index.
    powers: Box<[Powers]>,
    /// The highest low of the maximal rest.
    overlap_low: Monotone,
    /// The lowest high of the maximal rest.
    overlap_high: Monotone,
    /// The first index after the last hole.
    segment_start: usize,
    /// The maximal rest's first index.
    rest_start: usize,
    /// The last index that made the day high at its close.
    made_high: Option<usize>,
    /// The last index that made the day low at its close.
    made_low: Option<usize>,
    /// The last accepted close instant.
    prev_close: Option<i64>,
    /// The close instant the next candle must have to be contiguous.
    next_close: Option<i64>,
    /// The rest lanes; the first is the maximal rest.
    lanes: Vec<Lane>,
    /// Lanes in use today.
    active: usize,
    /// The long direction's day.
    long: DirectionState,
    /// The short direction's day.
    short: DirectionState,
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
        let lanes = (0..=long_lengths + short_lengths)
            .map(|_| Lane::new(capacity, long_levels, short_levels))
            .collect();
        let snapshots = KINDS + long_lengths + KINDS + short_lengths;
        let touches = (KINDS + long_lengths) * long_levels + (KINDS + short_lengths) * short_levels;
        Ok(Self {
            params,
            micro: (size(MICRO2_CANDLES), size(MICRO3_CANDLES)),
            parts: (size(SHAPE_HALVES), size(SHAPE_THIRDS)),
            day: None,
            extremes: DayExtremes::empty(),
            recs: Vec::with_capacity(capacity),
            prefix: Vec::with_capacity(capacity + 1),
            powers: powers.into_boxed_slice(),
            overlap_low: Monotone::new(capacity, true),
            overlap_high: Monotone::new(capacity, false),
            segment_start: 0,
            rest_start: 0,
            made_high: None,
            made_low: None,
            prev_close: None,
            next_close: None,
            lanes,
            active: 0,
            long: DirectionState::new(long_lengths, long_levels),
            short: DirectionState::new(short_lengths, short_levels),
            snapshots: Vec::with_capacity(snapshots),
            touches: Vec::with_capacity(touches),
            spans: Vec::with_capacity(snapshots),
            refusals: RefusalCounts::zero(),
            counts: DetectorCounts::zero(),
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
        &self.refusals
    }

    /// The other counts.
    #[must_use]
    pub const fn counts(&self) -> &DetectorCounts {
        &self.counts
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
        let lane = |l: &Lane| {
            [
                &l.high,
                &l.low,
                &l.close_high,
                &l.close_low,
                &l.poke_long,
                &l.poke_short,
            ]
            .iter()
            .map(|m| core::mem::size_of_val(&*m.slots))
            .sum::<usize>()
                + (l.long.counts.capacity() + l.short.counts.capacity())
                    * core::mem::size_of::<u16>()
                + core::mem::size_of::<Lane>()
        };
        core::mem::size_of::<Self>()
            + self.recs.capacity() * core::mem::size_of::<Rec>()
            + self.prefix.capacity() * core::mem::size_of::<Sums>()
            + core::mem::size_of_val(&*self.powers)
            + core::mem::size_of_val(&*self.overlap_low.slots)
            + core::mem::size_of_val(&*self.overlap_high.slots)
            + self.lanes.iter().map(lane).sum::<usize>()
            + (self.long.lengths.capacity() + self.short.lengths.capacity())
                * core::mem::size_of::<(usize, usize)>()
            + (self.long.tolerance.capacity() + self.short.tolerance.capacity())
                * core::mem::size_of::<u64>()
            + self.snapshots.capacity() * core::mem::size_of::<BaseSnapshot>()
            + self.touches.capacity() * core::mem::size_of::<u16>()
            + self.spans.capacity() * core::mem::size_of::<(usize, usize)>()
    }

    /// Counts a refusal and returns it.
    fn refuse(&mut self, why: Refusal) -> Refusal {
        self.refusals.record(why);
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
        self.recs.clear();
        self.prefix.clear();
        self.prefix.push(NO_SUMS);
        self.overlap_low.clear();
        self.overlap_high.clear();
        self.segment_start = 0;
        self.rest_start = 0;
        self.made_high = None;
        self.made_low = None;
        self.prev_close = None;
        self.next_close = None;
        for (state, input) in [(&mut self.long, &day.long), (&mut self.short, &day.short)] {
            state.lengths.clear();
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

    /// Lane 0 is the maximal rest; one lane per distinct generated length of
    /// either direction follows, ascending.
    fn assign_lanes(&mut self, long: &[u16], short: &[u16]) {
        let (long_levels, short_levels) = (self.long.tolerance.len(), self.short.tolerance.len());
        let reset = |lane: &mut Lane, length, long_uses, short_uses| {
            lane.clear();
            lane.length = length;
            lane.long_uses = long_uses;
            lane.short_uses = short_uses;
            lane.long.counts.clear();
            lane.long.counts.resize(long_levels, 0);
            lane.short.counts.clear();
            lane.short.counts.resize(short_levels, 0);
        };
        if let Some(lane) = self.lanes.first_mut() {
            reset(lane, None, true, true);
        }
        let (mut a, mut b) = (
            long.iter().copied().peekable(),
            short.iter().copied().peekable(),
        );
        let mut next = 1;
        loop {
            let (x, y) = (a.peek().copied(), b.peek().copied());
            let length = match (x, y) {
                (Some(x), Some(y)) => x.min(y),
                (Some(only), None) | (None, Some(only)) => only,
                (None, None) => break,
            };
            let (long_uses, short_uses) = (x == Some(length), y == Some(length));
            if long_uses {
                a.next();
            }
            if short_uses {
                b.next();
            }
            let length = usize::from(length);
            if let Some(lane) = self.lanes.get_mut(next) {
                reset(lane, Some(length), long_uses, short_uses);
            }
            if long_uses {
                self.long.lengths.push((length, next));
            }
            if short_uses {
                self.short.lengths.push((length, next));
            }
            next += 1;
        }
        self.active = next;
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
        self.counts.withheld_updates += 1;
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
        let refusal = if given.tf_minutes != self.params.tf_minutes {
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
        } else if self.recs.len() >= day.session {
            Some(Refusal::SessionLongerThanRing)
        } else {
            None
        };
        if let Some(why) = refusal {
            return Err(self.refuse(why));
        }
        let t = self.recs.len();
        let index = u32::try_from(t).unwrap_or(u32::MAX);
        let Ok(prior) = self.extremes.step(
            &given.candle,
            index,
            given.close_ts_micros,
            input.suspect_print,
        ) else {
            return Err(self.refuse(Refusal::CorruptCandle));
        };
        let c = given.candle;
        let linked = self.next_close == Some(given.close_ts_micros);
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
        if prior.is_none_or(|p| c.high > p.high) {
            self.made_high = Some(t);
        }
        if prior.is_none_or(|p| c.low < p.low) {
            self.made_low = Some(t);
        }
        let previous = self.recs.last().filter(|_| linked).copied();
        let rec = Rec {
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            close_ts: given.close_ts_micros,
            linked,
            poke_long,
            poke_short,
        };
        self.push_sums(&rec, previous.as_ref(), index);
        self.recs.push(rec);
        self.prev_close = Some(given.close_ts_micros);
        self.next_close = Some(input.next_close_ts_micros);
        if !linked {
            self.segment_start = t;
            self.rest_start = t;
            self.overlap_low.clear();
            self.overlap_high.clear();
            for lane in self.lanes.iter_mut().take(self.active) {
                lane.clear();
            }
            if t > 0 {
                self.counts.rest_resets += 1;
            }
        }
        self.advance_rest(index, &rec);
        self.advance_lanes(index, &rec, t);
        self.snapshots.clear();
        self.touches.clear();
        self.spans.clear();
        let Some(ladder) = self.extremes.ladder(day.tick) else {
            self.refusals.record(Refusal::NoLadder);
            return Ok(self.emitted(None));
        };
        for direction in [TradeDir::Long, TradeDir::Short] {
            self.emit_direction(direction, t, &ladder);
        }
        Ok(self.emitted(Some(ladder)))
    }

    /// The emitted view of the buffers.
    fn emitted(&self, ladder: Option<Ladder>) -> Emitted<'_> {
        Emitted {
            snapshots: &self.snapshots,
            touches: &self.touches,
            spans: &self.spans,
            ladder,
        }
    }

    /// Appends candle `rec`'s prefix sums.
    fn push_sums(&mut self, rec: &Rec, previous: Option<&Rec>, index: u32) {
        let last = self.prefix.last().copied().unwrap_or(NO_SUMS);
        let x = i128::from(index);
        let (open, high, low, close) = (
            i128::from(rec.open),
            i128::from(rec.high),
            i128::from(rec.low),
            i128::from(rec.close),
        );
        let flag = |test: fn(&Rec, &Rec) -> bool| u32::from(previous.is_some_and(|p| test(rec, p)));
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
    fn advance_lanes(&mut self, index: u32, rec: &Rec, t: usize) {
        let (rest_start, segment_start) = (self.rest_start, self.segment_start);
        for lane in self.lanes.iter_mut().take(self.active) {
            let start = lane.length.map_or(rest_start, |length| {
                segment_start.max((t + 1).saturating_sub(length))
            });
            self.work += lane.push(index, rec, u32::try_from(start).unwrap_or(u32::MAX));
            // Both fronts exist: the candle was just pushed.
            if let (Some(top), Some(bottom)) = (lane.high.front(), lane.low.front()) {
                for (direction, uses, touch, state, edge) in [
                    (
                        TradeDir::Long,
                        lane.long_uses,
                        &mut lane.long,
                        &self.long,
                        top,
                    ),
                    (
                        TradeDir::Short,
                        lane.short_uses,
                        &mut lane.short,
                        &self.short,
                        -bottom,
                    ),
                ] {
                    if uses && state.known {
                        let (recounted, visited) = update_touch(
                            touch,
                            &self.recs,
                            direction,
                            &state.tolerance,
                            edge,
                            (start, t),
                        );
                        let visited = u64::try_from(visited).unwrap_or(u64::MAX);
                        let levels = u64::try_from(state.tolerance.len()).unwrap_or(u64::MAX);
                        self.work += visited.saturating_mul(levels);
                        if recounted {
                            self.counts.touch_recounts += 1;
                            self.counts.touch_recount_candles += visited;
                        }
                    }
                }
            }
        }
    }

    /// Emits every base of one direction that ends at `t`.
    fn emit_direction(&mut self, direction: TradeDir, t: usize, ladder: &Ladder) {
        let (micro2, micro3) = self.micro;
        for (kind, size) in [(BaseKind::Micro2, micro2), (BaseKind::Micro3, micro3)] {
            let Some(s) = (t + 1).checked_sub(size) else {
                continue;
            };
            let contiguous = self
                .recs
                .get(s + 1..=t)
                .is_some_and(|tail| tail.iter().all(|r| r.linked));
            if contiguous {
                let ext = Extremes::of(self.recs.get(s..=t).unwrap_or(&[]));
                self.emit(direction, kind, (s, t), ext, TouchSource::Direct, ladder);
            } else {
                self.refusals.record(Refusal::GapInBase);
            }
        }
        let rest = t + 1 - self.rest_start;
        if rest < micro2 {
            return;
        }
        if let Some(lane) = self.lanes.first() {
            let ext = lane.extremes();
            self.emit(
                direction,
                BaseKind::Rest,
                (self.rest_start, t),
                ext,
                TouchSource::Lane(0),
                ladder,
            );
        }
        // Taken out for the loop and put back after: `Vec::new` allocates
        // nothing and the capacity comes back with the list.
        let lengths = core::mem::take(match direction {
            TradeDir::Long => &mut self.long.lengths,
            TradeDir::Short => &mut self.short.lengths,
        });
        for &(length, lane) in &lengths {
            if length > micro3
                && length < rest
                && let Some(ext) = self.lanes.get(lane).map(Lane::extremes)
            {
                let span = (t + 1 - length, t);
                self.emit(
                    direction,
                    BaseKind::Rest,
                    span,
                    ext,
                    TouchSource::Lane(lane),
                    ladder,
                );
            }
        }
        match direction {
            TradeDir::Long => self.long.lengths = lengths,
            TradeDir::Short => self.short.lengths = lengths,
        }
    }

    /// Measures and emits one base over day indices `span`.
    fn emit(
        &mut self,
        direction: TradeDir,
        kind: BaseKind,
        span: (usize, usize),
        ext: Extremes,
        source: TouchSource,
        ladder: &Ladder,
    ) {
        let (s, t) = span;
        let window = self.window(direction, s, t, &ext);
        let measures = measure::measure(&window, ladder, direction);
        if measures.zero_width() {
            if self.params.zero_width == ZeroWidthReading::Refuse {
                self.refusals.record(Refusal::ZeroWidthBase);
                return;
            }
            self.counts.zero_width_bases += 1;
        }
        if measures.squeeze == Ratio::OverZero {
            self.counts.ratio_over_zero += 1;
        }
        let stamp = ladder.stamp();
        self.counts.extreme_from_withheld_bucket += u64::from(stamp.extreme_from_withheld_bucket);
        self.counts.frame_from_suspect_print += u64::from(stamp.frame_from_suspect_print);
        let state = match direction {
            TradeDir::Long => &self.long,
            TradeDir::Short => &self.short,
        };
        let known = state.known;
        let from = self.touches.len();
        if known {
            match source {
                TouchSource::Direct => {
                    let start = self.touches.len();
                    self.touches.resize(start + state.tolerance.len(), 0);
                    if let Some(counts) = self.touches.get_mut(start..) {
                        count_touches(
                            counts,
                            self.recs.get(s..=t).unwrap_or(&[]),
                            direction,
                            window.high,
                            &state.tolerance,
                            true,
                        );
                    }
                }
                TouchSource::Lane(lane) => {
                    if let Some(lane) = self.lanes.get(lane) {
                        let touch = match direction {
                            TradeDir::Long => &lane.long,
                            TradeDir::Short => &lane.short,
                        };
                        self.touches.extend_from_slice(&touch.counts);
                    }
                }
            }
        } else {
            self.refusals.record(Refusal::NoTouchTolerance);
        }
        self.spans.push((from, self.touches.len()));
        let close_at = |i: usize| self.recs.get(i).map_or(0, |r| r.close_ts);
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
        let rec = |i: usize| self.recs.get(i).map(|r| view_bar(r, direction));
        let zero = Bar { high: 0, low: 0 };
        let candles = u32::try_from(n).unwrap_or(u32::MAX);
        let first_index = u32::try_from(s).unwrap_or(u32::MAX);
        let (first, last) = (rec(s).unwrap_or(zero), rec(t).unwrap_or(zero));
        let middle = if n == self.micro.1 { rec(s + 1) } else { None };
        let long = Window {
            candles,
            first_index,
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
            first,
            middle,
            last,
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
}
