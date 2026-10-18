//! The bench rows of the base detector (FIB-3; D-6342, D-6344).
//!
//! One call folds ONE closed candle of a 375-candle one-minute session and
//! emits every base that ends at it, in both directions, with seven generated
//! rest lengths (4 .. 256, doubling) and two touch levels per direction. The
//! scale is how many whole days the detector folded before it is measured
//! (1, 100 and 1,000): its state is allocated once and cleared each day, so
//! the axis is FIXED-STATE and the claim is that a day does not grow it.
//!
//! Every price is synthetic. The streams:
//!
//! | Row | Stream | What moves every candle |
//! |---|---|---|
//! | C-FIB3-01 | each candle overlaps only the one before it | the rest's start (rest length 2) and its highest high |
//! | C-FIB3-02 | each candle overlaps the fifteen before it | the start (rest length 16) and the highest high |
//! | C-FIB3-03 | the whole session overlaps, its high made by the first candle | nothing: the rest grows to 375 candles |
//! | C-FIB3-04 | as C-FIB3-03, then the last candle overlaps none | once a day the start jumps 374 candles |
//! | C-FIB3-05 | the whole session overlaps and every high is one paisa higher | the highest high: every touch count recounts its window |

use std::fmt::Write as _;

use indicators::fibflag::base::{CandleInput, DayInput, Detector, DirectionDay};
use indicators::fibflag::types::{DetectorParams, FibCandle, ZeroWidthReading};
use indicators::{Candle, OI_NULL};

use crate::harness::{Axis, Class, Host, Row, run};

/// One one-minute session.
const SESSION: u16 = 375;
/// The session, as a grid slot.
const LAST_SLOT: i64 = 374;
/// The generated rest lengths of the bench, both directions.
const LENGTHS: [u16; 7] = [4, 8, 16, 32, 64, 128, 256];
/// The touch tolerances of the bench, in basis points.
const TOLERANCES: [u64; 2] = [0, 20];
/// A synthetic price level, in paisa.
const BASE: i64 = 4_321_000;
/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// A stream: the `(open, high, low, close)` of grid slot `t`.
type Stream = fn(i64) -> (i64, i64, i64, i64);

/// Each candle overlaps only the one before it: rest length 2.
fn stair(t: i64) -> (i64, i64, i64, i64) {
    let low = BASE + 5 * t;
    (low, low + 5, low, low + 5)
}

/// Each candle overlaps the fifteen before it: rest length 16.
fn slide(t: i64) -> (i64, i64, i64, i64) {
    let low = BASE + 5 * t;
    (low + 10, low + 75, low, low + 40)
}

/// The whole session overlaps; the first candle makes the high.
fn flat(t: i64) -> (i64, i64, i64, i64) {
    if t == 0 {
        (BASE, BASE + 200, BASE - 100, BASE + 50)
    } else {
        (BASE, BASE + 100, BASE - 50, BASE + (t % 7) * 3)
    }
}

/// The whole-session stream, then a last candle that overlaps none of it.
fn jump(t: i64) -> (i64, i64, i64, i64) {
    if t == LAST_SLOT {
        (BASE + 1_000, BASE + 1_100, BASE + 1_000, BASE + 1_050)
    } else {
        flat(t)
    }
}

/// The whole session overlaps and every high is one paisa higher.
fn creep(t: i64) -> (i64, i64, i64, i64) {
    (BASE, BASE + 100 + t, BASE - 50, BASE + 10)
}

/// One detector folding one stream, day after day.
pub(crate) struct Detect {
    /// The detector.
    detector: Detector,
    /// The stream.
    stream: Stream,
    /// The IST day being folded.
    day: i64,
    /// The next grid slot.
    slot: i64,
}

impl Detect {
    /// A detector at the bench's generation, before any day.
    fn new(stream: Stream) -> Self {
        let params = DetectorParams {
            tf_minutes: 1,
            generation_version: 1,
            ring_capacity: SESSION,
            long_rest_lengths: LENGTHS.len() as u16,
            short_rest_lengths: LENGTHS.len() as u16,
            long_touch_levels: TOLERANCES.len() as u16,
            short_touch_levels: TOLERANCES.len() as u16,
            zero_width: ZeroWidthReading::Emit,
        };
        let mut state = Self {
            detector: Detector::new(params).expect("a one-minute width"),
            stream,
            day: 30_000,
            slot: 0,
        };
        state.begin();
        state
    }

    /// Begins the current day.
    fn begin(&mut self) {
        let direction = DirectionDay {
            rest_lengths: &LENGTHS,
            touch_bp: Some(&TOLERANCES),
        };
        self.detector
            .begin_day(&DayInput {
                ist_day: self.day,
                session_candles: SESSION,
                observed_tick: 5,
                long: direction,
                short: direction,
            })
            .expect("the bench's day fits its generation");
    }

    /// Folds the next candle; returns how many bases it emitted.
    fn step(&mut self) -> u64 {
        if self.slot > LAST_SLOT {
            self.day += 1;
            self.slot = 0;
            self.begin();
        }
        let (open, high, low, close) = (self.stream)(self.slot);
        let open_ts =
            self.day * 86_400_000_000 - 19_800_000_000 + 33_300_000_000 + self.slot * MINUTE;
        let input = CandleInput {
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
            next_close_ts_micros: open_ts + 2 * MINUTE,
            suspect_print: false,
        };
        self.slot += 1;
        self.detector
            .fold(&input)
            .map_or(0, |emitted| emitted.snapshots().len() as u64)
    }

    /// The detector after `days` whole days of `stream`.
    fn after(stream: Stream, days: u64) -> Self {
        let mut state = Self::new(stream);
        for _ in 0..days * u64::from(SESSION) {
            state.step();
        }
        state
    }
}

/// A row over one stream.
fn row(
    id: &'static str,
    name: &'static str,
    class: Class,
    setup: fn(u64) -> Detect,
) -> Row<Detect> {
    Row {
        id,
        name,
        class: Some(class),
        axis: Some(Axis::FixedState {
            why: "the per-(stock-day, width) state is allocated once at the span's longest \
                  session and cleared each day; the scale is the days folded before",
        }),
        scales: [1, 100, 1_000],
        setup,
        op: |state, _| state.step(),
        work: |state| state.detector.work(),
        bytes: |state| state.detector.state_bytes() as u64,
    }
}

/// The rows, run and reported. False when any row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let day = Class::Amortised {
        period: u64::from(SESSION),
    };
    let rows = [
        row(
            "C-FIB3-01",
            "fold one candle, rest length 2, start moving every candle",
            day,
            |days| Detect::after(stair, days),
        ),
        row(
            "C-FIB3-02",
            "fold one candle, rest length 16, start moving every candle",
            day,
            |days| Detect::after(slide, days),
        ),
        row(
            "C-FIB3-03",
            "fold one candle, rest length up to 375 (a whole one-minute session)",
            day,
            |days| Detect::after(flat, days),
        ),
        row(
            "C-FIB3-04",
            "fold one candle, the rest's start jumping 374 candles once a day",
            Class::Bounded {
                bound: "O(session candles) at the candle the start jumps; amortised O(1) per candle",
            },
            |days| Detect::after(jump, days),
        ),
        row(
            "C-FIB3-05",
            "touch_recount_worst: every high creeps, every touch count recounts its window",
            Class::Bounded {
                bound: "O(window candles x touch levels) per recount, the window bounded by the session",
            },
            |days| Detect::after(creep, days),
        ),
    ];
    let state = Detect::after(flat, 1);
    let _ = writeln!(
        out,
        "detect: one (stock-day, width) state at a 375-candle ring, 7 rest lengths and 2 touch \
         levels a direction: {} bytes ({} fixed)",
        state.detector.state_bytes(),
        core::mem::size_of::<Detector>()
    );
    let mut ok = true;
    for row in &rows {
        let verdict = run(row, host, out);
        ok &= !verdict.fails();
        let counts = {
            let probe = (row.setup)(1);
            *probe.detector.counts()
        };
        let _ = writeln!(
            out,
            "  per day of this stream: {} snapshots, {} touch recounts visiting {} candles, {} rest resets",
            counts.snapshots,
            counts.touch_recounts,
            counts.touch_recount_candles,
            counts.rest_resets
        );
    }
    let _ = writeln!(out);
    ok
}
