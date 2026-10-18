//! FIB-3's detector, proved against its specification (D-6340 to D-6347).
//!
//! Every price is synthetic: random walks around 1,23,457 paisa and hand-built
//! candles at prices no listed share trades at. Nothing here reads a store or
//! a vendor row.
//!
//! The ORACLE below recomputes every base and every measurement from the
//! day's candle list directly: a backward scan for the rest, plain sums over
//! the slice, the thirds and halves by slicing. It shares no code with the
//! detector except the public types it compares, and the short view is the
//! oracle's long-view code run on negated prices.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    reason = "the exception every test crate in this workspace takes: a test \
              that cannot panic cannot fail."
)]

use indicators::fibflag::base::{CandleInput, DayInput, Detector, DirectionDay};
use indicators::fibflag::extremes::WithheldExtremes;
use indicators::fibflag::measure::{Exact, LevelRelation, Ratio};
use indicators::fibflag::shape::Shape;
use indicators::fibflag::types::{
    BaseKind, BaseSnapshot, DetectorParams, FibCandle, Refusal, TradeDir, ZeroWidthReading,
};
use indicators::{Candle, CurDayFib, Leg, OI_NULL, ist_day};
use vocab::fib_cube::{ZONE_RUNGS, untyped_numbers};

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;
/// One day, in microseconds.
const DAY_MICROS: i64 = 86_400_000_000;
/// The IST offset, in microseconds.
const IST: i64 = 19_800_000_000;
/// 09:15 IST, in microseconds after IST midnight.
const OPEN_0915: i64 = (9 * 3_600 + 15 * 60) * 1_000_000;

/// A deterministic generator (splitmix64).
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, permille: u64) -> bool {
        self.below(1_000) < permille
    }
}

/// The UTC instant of 09:15 IST on IST day `day`.
fn open_of(day: i64) -> i64 {
    day * DAY_MICROS - IST + OPEN_0915
}

/// The candle at grid slot `slot` of a width of `tf` minutes on `day`.
fn input(day: i64, tf: u16, slot: i64, ohlc: (i64, i64, i64, i64)) -> CandleInput {
    let width = i64::from(tf) * MINUTE;
    let open_ts = open_of(day) + slot * width;
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
            tf_minutes: tf,
            complete: true,
            missing_minutes: 0,
            close_ts_micros: open_ts + width,
        },
        next_close_ts_micros: open_ts + width + width,
        suspect_print: false,
    }
}

/// Parameters for the random days.
fn params(capacity: u16, lengths: u16, levels: u16) -> DetectorParams {
    DetectorParams {
        tf_minutes: 1,
        generation_version: 1,
        ring_capacity: capacity,
        long_rest_lengths: lengths,
        short_rest_lengths: lengths,
        long_touch_levels: levels,
        short_touch_levels: levels,
        zero_width: ZeroWidthReading::Emit,
    }
}

/// A day's input with no generated lengths and one zero tolerance.
fn plain_day(day: i64, session: u16, tick: i64) -> DayInput<'static> {
    DayInput {
        ist_day: day,
        session_candles: session,
        observed_tick: tick,
        long: DirectionDay {
            rest_lengths: &[],
            touch_bp: Some(&[0]),
        },
        short: DirectionDay {
            rest_lengths: &[],
            touch_bp: Some(&[0]),
        },
    }
}

/// One emitted base, owned: the snapshot and its touch counts.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Emitted {
    snapshot: BaseSnapshot,
    touches: Vec<u16>,
}

/// Folds one candle and owns what it emitted.
fn fold(detector: &mut Detector, candle: &CandleInput) -> Result<Vec<Emitted>, Refusal> {
    let emitted = detector.fold(candle)?;
    Ok(emitted
        .snapshots()
        .iter()
        .enumerate()
        .map(|(i, snapshot)| Emitted {
            snapshot: *snapshot,
            touches: emitted.touches(i).to_vec(),
        })
        .collect())
}

/// The bases of one kind and direction.
fn of(emitted: &[Emitted], kind: BaseKind, direction: TradeDir) -> Vec<&Emitted> {
    emitted
        .iter()
        .filter(|e| e.snapshot.kind == kind && e.snapshot.direction == direction)
        .collect()
}

/// Two exact values are the same number.
fn same(a: Exact, b: (i128, i128)) -> bool {
    a.num() * b.1 == b.0 * a.den()
}

// ---------------------------------------------------------------------------
// The oracle.
// ---------------------------------------------------------------------------

/// A candle in a direction's view: `(open, high, low, close)`.
type View = (i64, i64, i64, i64);

/// A grid slot and its `(open, high, low, close)`.
type Slot = (i64, (i64, i64, i64, i64));

/// The view of a raw candle.
fn view(c: &Candle, direction: TradeDir) -> View {
    match direction {
        TradeDir::Long => (c.open, c.high, c.low, c.close),
        TradeDir::Short => (-c.open, -c.low, -c.high, -c.close),
    }
}

/// The day so far, as the oracle keeps it: every accepted candle and whether it
/// was contiguous with the one before.
#[derive(Clone, Debug, Default)]
struct Day {
    candles: Vec<Candle>,
    closes: Vec<i64>,
    linked: Vec<bool>,
}

/// What the oracle expects one base to measure.
#[derive(Clone, Debug)]
struct Expected {
    kind: BaseKind,
    direction: TradeDir,
    s: usize,
    t: usize,
}

/// The first index of the segment `t` belongs to.
fn segment_start(day: &Day, t: usize) -> usize {
    let mut s = t;
    while s > 0 && day.linked[s] {
        s -= 1;
    }
    s
}

/// The maximal rest's first index, by a backward scan.
fn naive_rest_start(day: &Day, t: usize) -> usize {
    let floor = segment_start(day, t);
    let mut s = t;
    while s > floor {
        let window = &day.candles[s - 1..=t];
        let low = window.iter().map(|c| c.low).max().unwrap();
        let high = window.iter().map(|c| c.high).min().unwrap();
        if low > high {
            break;
        }
        s -= 1;
    }
    s
}

/// The bases the specification says end at `t`, in emission order.
fn expected_bases(day: &Day, t: usize, lengths: [&[u16]; 2]) -> Vec<Expected> {
    let mut out = Vec::new();
    for (direction, lengths) in [(TradeDir::Long, lengths[0]), (TradeDir::Short, lengths[1])] {
        for (kind, size) in [(BaseKind::Micro2, 2), (BaseKind::Micro3, 3)] {
            if t + 1 >= size && (t + 2 - size..=t).all(|i| day.linked[i]) {
                out.push(Expected {
                    kind,
                    direction,
                    s: t + 1 - size,
                    t,
                });
            }
        }
        let start = naive_rest_start(day, t);
        let rest = t + 1 - start;
        if rest >= 2 {
            out.push(Expected {
                kind: BaseKind::Rest,
                direction,
                s: start,
                t,
            });
            for &length in lengths {
                let length = usize::from(length);
                if length > 3 && length < rest {
                    out.push(Expected {
                        kind: BaseKind::Rest,
                        direction,
                        s: t + 1 - length,
                        t,
                    });
                }
            }
        }
    }
    out
}

/// `floor(num x unit / den)` with the stated zero-denominator limits.
fn naive_ratio(num: i128, den: i128, unit: i128) -> Ratio {
    if den == 0 {
        if num == 0 {
            Ratio::Value(unit as u64)
        } else {
            Ratio::OverZero
        }
    } else {
        Ratio::Value((num * unit / den) as u64)
    }
}

/// `floor(num x 1000 / den)` in 0..=1000.
fn per(num: i128, den: i128) -> u16 {
    (num * 1_000 / den).clamp(0, 1_000) as u16
}

/// Checks one emitted base against the oracle's recomputation.
fn check_base(day: &Day, base: &Emitted, want: &Expected, tolerances: [Option<&[u64]>; 2]) {
    let snap = &base.snapshot;
    let (s, t) = (want.s, want.t);
    assert_eq!((snap.kind, snap.direction), (want.kind, want.direction));
    assert_eq!(
        (snap.first_index, snap.last_index),
        (s as u32, t as u32),
        "{want:?}"
    );
    let raw = &day.candles[s..=t];
    assert_eq!(snap.high, raw.iter().map(|c| c.high).max().unwrap());
    assert_eq!(snap.low, raw.iter().map(|c| c.low).min().unwrap());
    assert_eq!(usize::from(snap.candles), t + 1 - s);
    assert_eq!(snap.first_close_ts_micros, day.closes[s]);
    assert_eq!(snap.last_close_ts_micros, day.closes[t]);
    let direction = want.direction;
    let v: Vec<View> = raw.iter().map(|c| view(c, direction)).collect();
    let n = v.len() as i128;
    let bh = v.iter().map(|c| c.1).max().unwrap();
    let bl = v.iter().map(|c| c.2).min().unwrap();
    let w = i128::from(bh) - i128::from(bl);
    let m = &snap.measures;
    assert_eq!(m.width, w);
    assert_eq!(m.zero_width(), w == 0);
    assert_eq!(m.width_bp, naive_ratio(w, i128::from(bh).abs(), 10_000));
    assert_eq!(
        m.range_sum,
        v.iter().map(|c| i128::from(c.1 - c.2)).sum::<i128>()
    );
    assert_eq!(
        m.body_sum,
        v.iter()
            .map(|c| i128::from((c.3 - c.0).abs()))
            .sum::<i128>()
    );
    // Slopes over the absolute day index, by plain sums.
    let xs: Vec<i128> = (s..=t).map(|x| x as i128).collect();
    let sx: i128 = xs.iter().sum();
    let sxx: i128 = xs.iter().map(|x| x * x).sum();
    let slope = |y: &dyn Fn(&View) -> i64| {
        let sy: i128 = v.iter().map(|c| i128::from(y(c))).sum();
        let sxy: i128 = v.iter().zip(&xs).map(|(c, x)| x * i128::from(y(c))).sum();
        (n * sxy - sx * sy, n * sxx - sx * sx)
    };
    assert!(same(m.slope_highs.unwrap(), slope(&|c| c.1)), "{want:?}");
    assert!(same(m.slope_lows.unwrap(), slope(&|c| c.2)), "{want:?}");
    if v.len() >= 3 {
        let p = |k: u32| xs.iter().map(|x| x.pow(k)).sum::<i128>();
        let q = |k: u32| {
            v.iter()
                .zip(&xs)
                .map(|(c, x)| x.pow(k) * i128::from(c.3))
                .sum::<i128>()
        };
        let (s1, s2, s3, s4) = (p(1), p(2), p(3), p(4));
        let (t0, t1, t2) = (q(0), q(1), q(2));
        let det = |a: [[i128; 3]; 3]| {
            a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
        };
        let den = det([[n, s1, s2], [s1, s2, s3], [s2, s3, s4]]);
        let num = det([[n, s1, t0], [s1, s2, t1], [s2, s3, t2]]);
        assert!(same(m.curvature_closes.unwrap(), (num, den)), "{want:?}");
    } else {
        assert!(m.curvature_closes.is_none());
    }
    // Squeeze: the last half's mean range over the first half's.
    let first_half = v.len() / 2;
    let range = |c: &View| i128::from(c.1 - c.2);
    let rf: i128 = v[..first_half].iter().map(range).sum();
    let rl: i128 = v[first_half..].iter().map(range).sum();
    let nf = first_half as i128;
    let squeeze = naive_ratio(rl * nf, rf * (n - nf), 1_000);
    assert_eq!(m.squeeze, squeeze, "{want:?}");
    // Holds, from the day's own running extremes.
    let made = |i: usize, high: bool| {
        i == 0
            || if high {
                day.candles[i].high > day.candles[..i].iter().map(|c| c.high).max().unwrap()
            } else {
                day.candles[i].low < day.candles[..i].iter().map(|c| c.low).min().unwrap()
            }
    };
    let made_high = (s..=t).any(|i| made(i, true));
    let made_low = (s..=t).any(|i| made(i, false));
    let (level0, level1) = match direction {
        TradeDir::Long => (made_high, made_low),
        TradeDir::Short => (made_low, made_high),
    };
    assert_eq!(
        (m.holds.level0, m.holds.level1),
        (level0, level1),
        "{want:?}"
    );
    assert_eq!(m.holds.extreme(), level0 || level1);
    // The frame, after t folded.
    let h = day.candles[..=t].iter().map(|c| c.high).max().unwrap();
    let l = day.candles[..=t].iter().map(|c| c.low).min().unwrap();
    let r = i128::from(h - l);
    let l0 = i128::from(match direction {
        TradeDir::Long => h,
        TradeDir::Short => -l,
    });
    let part = l0 - i128::from(bl);
    assert!(same(m.retrace, (part * 1_000, r)));
    assert!((0..=r).contains(&part), "the day range contains the base");
    let mut zone = 0;
    let mut best = i128::MAX;
    for (at, rung) in ZONE_RUNGS.iter().enumerate() {
        let gap = (part * 1_000 - i128::from(*rung) * r).abs();
        if gap < best {
            best = gap;
            zone = at;
        }
    }
    assert_eq!(usize::from(m.fib_zone), zone);
    let rung = i128::from(ZONE_RUNGS[zone]);
    assert!(same(m.level_distance, ((rung * r - 1_000 * part).abs(), r)));
    let price = 1_000 * l0 - rung * r;
    let relation = if 1_000 * i128::from(bl) > price {
        LevelRelation::Above
    } else if 1_000 * i128::from(bh) < price {
        LevelRelation::Below
    } else {
        LevelRelation::Spans
    };
    assert_eq!(m.relation, relation, "{want:?}");
    assert_eq!((snap.ladder.high, snap.ladder.low), (h, l));
    // Shapes.
    let score = |shape: Shape| snap.shape.get(shape);
    match want.kind {
        BaseKind::Rest => {
            let third = v.len() / 3;
            let (u, inverted) = if third == 0 || w == 0 {
                (0, 0)
            } else {
                let close = |c: &View| i128::from(c.3);
                let ends: i128 = v[..third].iter().map(close).sum::<i128>()
                    + v[v.len() - third..].iter().map(close).sum::<i128>();
                let middle = v.len() - 2 * third;
                let mid: i128 = v[third..third + middle].iter().map(close).sum();
                let k2 = 2 * third as i128;
                let mm = middle as i128;
                let diff = ends * mm - mid * k2;
                let den = k2 * mm * w;
                (per(diff.max(0), den), per((-diff).max(0), den))
            };
            assert_eq!(score(Shape::U), u, "{want:?}");
            assert_eq!(score(Shape::InvertedU), inverted, "{want:?}");
            let flat = if w == 0 {
                1_000
            } else {
                let dev: i128 = v
                    .iter()
                    .map(|c| i128::from(bh - c.1) + i128::from(c.2 - bl))
                    .sum();
                1_000 - per(dev, 2 * n * w)
            };
            assert_eq!(score(Shape::Flat), flat, "{want:?}");
            let triangle = match squeeze {
                Ratio::Value(value) => 1_000_u16.saturating_sub(value.min(1_000) as u16),
                Ratio::OverZero => 0,
            };
            assert_eq!(score(Shape::Triangle), triangle);
            let pairs = (v.len() - 1) as i128;
            let inside = v
                .windows(2)
                .filter(|p| p[1].1 <= p[0].1 && p[1].2 >= p[0].2)
                .count() as i128;
            assert_eq!(score(Shape::InsideCandles), per(inside, pairs));
            let tight = if w == 0 {
                1_000
            } else {
                let closes: Vec<i64> = v.iter().map(|c| c.3).collect();
                let spread =
                    i128::from(closes.iter().max().unwrap() - closes.iter().min().unwrap());
                1_000 - per(spread, w)
            };
            assert_eq!(score(Shape::TightCloses), tight);
            let rising = v.windows(2).filter(|p| p[1].2 > p[0].2).count() as i128;
            assert_eq!(score(Shape::RisingLows), per(rising, pairs));
            // The deepest view poke below the day's view low before the candle.
            let poke = (s..=t)
                .filter(|&i| i > 0)
                .map(|i| {
                    let before: Vec<View> = day.candles[..i]
                        .iter()
                        .map(|c| view(c, direction))
                        .collect();
                    let low = before.iter().map(|c| c.2).min().unwrap();
                    let c = view(&day.candles[i], direction);
                    if c.2 < low && c.3 > low { low - c.2 } else { 0 }
                })
                .max()
                .unwrap_or(0);
            let shakeout = if w == 0 { 0 } else { per(i128::from(poke), w) };
            assert_eq!(score(Shape::ShakeoutReclaim), shakeout, "{want:?}");
            for shape in Shape::MICRO {
                assert_eq!(score(*shape), 0);
            }
        }
        BaseKind::Micro2 | BaseKind::Micro3 => {
            let (first, last) = (v[0], v[v.len() - 1]);
            let v_bottom = if v.len() == 3 && v[1].2 < first.2 && v[1].2 < last.2 {
                per(i128::from(first.2.min(last.2) - v[1].2), w)
            } else {
                0
            };
            assert_eq!(score(Shape::VBottom), v_bottom);
            let partial = if w == 0 {
                1_000
            } else {
                1_000 - per(i128::from((last.2 - first.2).abs()), w)
            };
            assert_eq!(score(Shape::PartialU), partial);
            let before = v[v.len() - 2];
            let pair = if last.1 <= before.1 && last.2 >= before.2 {
                1_000
            } else {
                0
            };
            assert_eq!(score(Shape::InsidePair), pair);
            let rises: i128 = v
                .windows(2)
                .map(|p| i128::from(p[1].1 > p[0].1) + i128::from(p[1].2 > p[0].2))
                .sum();
            assert_eq!(score(Shape::Stair), per(rises, 2 * (n - 1)));
            for shape in Shape::REST {
                assert_eq!(score(*shape), 0);
            }
        }
    }
    // Touches: candles whose view high is within the tolerance of the view
    // base high, per level.
    let tolerance = match direction {
        TradeDir::Long => tolerances[0],
        TradeDir::Short => tolerances[1],
    };
    assert_eq!(snap.touches_known, tolerance.is_some());
    let want_touches: Vec<u16> = tolerance
        .unwrap_or(&[])
        .iter()
        .map(|tol| {
            v.iter()
                .filter(|c| {
                    (i128::from(bh) - i128::from(c.1)).abs() * 10_000
                        <= i128::from(*tol) * i128::from(bh).abs()
                })
                .count() as u16
        })
        .collect();
    assert_eq!(base.touches, want_touches, "{want:?}");
}

/// One random day's parameters and candles.
struct RandomDay {
    tick: i64,
    lengths: [Vec<u16>; 2],
    tolerances: [Option<Vec<u64>>; 2],
    slots: Vec<Slot>,
}

/// A random day: a slow walk, some one-price candles, some holes, at most
/// `candles` candles, the first one wide enough for a ladder at tick 1.
fn random_day(rng: &mut Seeded, candles: u64, zero_runs: bool) -> RandomDay {
    let count = 1 + rng.below(candles);
    let mut mid = 123_457 + i64::try_from(rng.below(5_000)).unwrap();
    let mut slot = 0;
    let mut slots = Vec::new();
    for i in 0..count {
        let flat = zero_runs && rng.chance(300);
        let step = i64::try_from(rng.below(41)).unwrap() - 20;
        mid += step;
        let (open, close) = if flat {
            (mid, mid)
        } else {
            (
                mid + i64::try_from(rng.below(15)).unwrap() - 7,
                mid + i64::try_from(rng.below(15)).unwrap() - 7,
            )
        };
        let wide = if i == 0 { 30 } else { 0 };
        let (up, down) = if flat {
            (0, 0)
        } else {
            (
                i64::try_from(rng.below(9)).unwrap() + wide,
                i64::try_from(rng.below(9)).unwrap() + wide,
            )
        };
        slots.push((
            slot,
            (open, open.max(close) + up, open.min(close) - down, close),
        ));
        slot += if rng.chance(60) {
            2 + i64::try_from(rng.below(3)).unwrap()
        } else {
            1
        };
    }
    let mut lengths = || {
        let mut out: Vec<u16> = (1..=20).filter(|_| rng.chance(250)).collect();
        out.truncate(6);
        out
    };
    let (long, short) = (lengths(), lengths());
    let mut tolerance = || {
        if rng.chance(100) {
            None
        } else {
            Some(
                (0..=rng.below(3))
                    .map(|_| rng.below(40))
                    .collect::<Vec<u64>>(),
            )
        }
    };
    let (tl, ts) = (tolerance(), tolerance());
    RandomDay {
        tick: if rng.chance(50) {
            1 + i64::try_from(rng.below(200)).unwrap()
        } else {
            1
        },
        lengths: [long, short],
        tolerances: [tl, ts],
        slots,
    }
}

/// Runs one random day through the detector and the oracle.
fn run_random_day(detector: &mut Detector, day_number: i64, day: &RandomDay, full: bool) -> u64 {
    let input_day = DayInput {
        ist_day: day_number,
        session_candles: 64,
        observed_tick: day.tick,
        long: DirectionDay {
            rest_lengths: &day.lengths[0],
            touch_bp: day.tolerances[0].as_deref(),
        },
        short: DirectionDay {
            rest_lengths: &day.lengths[1],
            touch_bp: day.tolerances[1].as_deref(),
        },
    };
    detector.begin_day(&input_day).unwrap();
    let mut oracle = Day::default();
    let mut previous_slot = None;
    let mut checked = 0;
    for &(slot, ohlc) in &day.slots {
        let candle = input(day_number, 1, slot, ohlc);
        let emitted = fold(detector, &candle).unwrap();
        oracle.candles.push(candle.candle.candle);
        oracle.closes.push(candle.candle.close_ts_micros);
        oracle.linked.push(previous_slot == Some(slot - 1));
        previous_slot = Some(slot);
        let t = oracle.candles.len() - 1;
        let h = oracle.candles.iter().map(|c| c.high).max().unwrap();
        let l = oracle.candles.iter().map(|c| c.low).min().unwrap();
        let ladder = i128::from(h - l) * 118 >= 1_000 * i128::from(day.tick);
        let want = if ladder {
            expected_bases(&oracle, t, [&day.lengths[0], &day.lengths[1]])
        } else {
            Vec::new()
        };
        assert_eq!(
            emitted.len(),
            want.len(),
            "day {day_number} candle {t}: {want:?}"
        );
        for (base, want) in emitted.iter().zip(&want) {
            if full {
                check_base(
                    &oracle,
                    base,
                    want,
                    [day.tolerances[0].as_deref(), day.tolerances[1].as_deref()],
                );
            } else {
                assert_eq!(
                    (
                        base.snapshot.kind,
                        base.snapshot.direction,
                        base.snapshot.first_index
                    ),
                    (want.kind, want.direction, want.s as u32)
                );
            }
            checked += 1;
        }
    }
    checked
}

// ---------------------------------------------------------------------------
// The specification's tests.
// ---------------------------------------------------------------------------

/// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): FIB-3's strategy files type
/// no number but 0 and 1 in their shipping parts; every other value is a
/// named constant of `vocab::fib_cube::FIXED_NUMBERS` or generated.
#[test]
fn fib3_strategy_files_type_no_number() {
    for (name, source) in [
        ("extremes.rs", include_str!("../src/fibflag/extremes.rs")),
        ("base.rs", include_str!("../src/fibflag/base.rs")),
        ("measure.rs", include_str!("../src/fibflag/measure.rs")),
        ("shape.rs", include_str!("../src/fibflag/shape.rs")),
        ("types.rs", include_str!("../src/fibflag/types.rs")),
    ] {
        let typed = untyped_numbers(source);
        assert!(typed.is_empty(), "{name} types {typed:?}");
    }
}

#[test]
fn range_zero_at_0915_has_no_ladder_and_is_counted() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    let emitted = fold(&mut d, &input(20_100, 1, 0, (5_555, 5_555, 5_555, 5_555))).unwrap();
    assert!(emitted.is_empty());
    assert_eq!(d.refusals().get(Refusal::NoLadder), 1);
    let flat_again = fold(&mut d, &input(20_100, 1, 1, (5_555, 5_555, 5_555, 5_555))).unwrap();
    assert!(flat_again.is_empty(), "two flat candles: still no ladder");
    assert_eq!(d.refusals().get(Refusal::NoLadder), 2);
}

#[test]
fn a_range_too_small_for_distinct_rungs_at_the_tick_has_no_ladder() {
    // R = 84: 84 x 118 = 9,912 < 1,000 x 10, so no ladder at tick 10; at
    // tick 9 (9,000) there is one.
    for (tick, ladder) in [(10, false), (9, true)] {
        let mut d = Detector::new(params(8, 0, 1)).unwrap();
        d.begin_day(&plain_day(20_100, 8, tick)).unwrap();
        fold(&mut d, &input(20_100, 1, 0, (7_000, 7_084, 7_000, 7_050))).unwrap();
        let e = fold(&mut d, &input(20_100, 1, 1, (7_050, 7_060, 7_040, 7_045))).unwrap();
        assert_eq!(!e.is_empty(), ladder, "tick {tick}");
        assert_eq!(d.refusals().get(Refusal::NoLadder), u64::from(!ladder) * 2);
    }
}

#[test]
fn a_single_candle_is_never_a_base() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200))).unwrap();
    assert!(e.is_empty(), "one wide candle: a ladder, but no base");
    assert_eq!(d.refusals().get(Refusal::NoLadder), 0);
    // After a hole, the first candle of the new segment is alone again.
    fold(&mut d, &input(20_100, 1, 1, (7_200, 7_250, 7_150, 7_200))).unwrap();
    let alone = fold(&mut d, &input(20_100, 1, 3, (7_200, 7_250, 7_150, 7_200))).unwrap();
    assert!(of(&alone, BaseKind::Rest, TradeDir::Long).is_empty());
    assert!(of(&alone, BaseKind::Micro2, TradeDir::Long).is_empty());
}

#[test]
fn an_engulfing_candle_makes_both_extremes() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_100, 6_950, 7_050))).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 1, (7_000, 7_400, 6_600, 7_300))).unwrap();
    let high = d.extremes().high().unwrap();
    let low = d.extremes().low().unwrap();
    assert_eq!(
        (high.price, high.index, low.price, low.index),
        (7_400, Some(1), 6_600, Some(1))
    );
    let micro = of(&e, BaseKind::Micro2, TradeDir::Long)[0];
    let m = &micro.snapshot.measures;
    assert!(m.holds.level0 && m.holds.level1 && m.holds.extreme());
    // The far edge is the day low: retrace 1000, zone l1000, spanning it.
    assert!(same(m.retrace, (1_000, 1)));
    assert_eq!(m.fib_zone, 6);
    assert_eq!(m.relation, LevelRelation::Spans);
    let short = &of(&e, BaseKind::Micro2, TradeDir::Short)[0]
        .snapshot
        .measures;
    assert!(short.holds.level0 && short.holds.level1);
    assert!(same(short.retrace, (1_000, 1)));
}

#[test]
fn a_new_high_inside_a_rest_holds_the_extreme_and_the_rest_continues() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    // A wide first candle, then a rest of overlapping candles, one of which
    // makes a new day high while the overlap holds.
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_200, 6_800, 7_100))).unwrap();
    fold(&mut d, &input(20_100, 1, 1, (7_100, 7_150, 7_050, 7_120))).unwrap();
    fold(&mut d, &input(20_100, 1, 2, (7_120, 7_160, 7_080, 7_140))).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 3, (7_140, 7_250, 7_100, 7_110))).unwrap();
    let rest = of(&e, BaseKind::Rest, TradeDir::Long)[0];
    assert_eq!(
        (rest.snapshot.first_index, rest.snapshot.candles),
        (0, 4),
        "the rest continues"
    );
    assert!(
        rest.snapshot.measures.holds.level0,
        "candle 3 made the day high at its close"
    );
    let e = fold(&mut d, &input(20_100, 1, 4, (7_110, 7_130, 7_105, 7_120))).unwrap();
    let rest = of(&e, BaseKind::Rest, TradeDir::Long)[0];
    assert_eq!(rest.snapshot.candles, 5);
    assert!(rest.snapshot.measures.holds.level0);
    let micro = of(&e, BaseKind::Micro2, TradeDir::Long)[0];
    assert!(
        micro.snapshot.measures.holds.level0,
        "candle 3 is in the micro base"
    );
    let e = fold(&mut d, &input(20_100, 1, 5, (7_120, 7_130, 7_110, 7_125))).unwrap();
    let micro = of(&e, BaseKind::Micro2, TradeDir::Long)[0];
    assert!(
        !micro.snapshot.measures.holds.extreme(),
        "candles 4 and 5 made neither"
    );
}

#[test]
fn a_hole_inside_a_micro_base_is_refused_and_the_rest_resets() {
    let mut d = Detector::new(params(16, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 16, 1)).unwrap();
    for slot in 0..3 {
        fold(
            &mut d,
            &input(20_100, 1, slot, (7_000, 7_200, 6_900, 7_100)),
        )
        .unwrap();
    }
    // Slot 3 is missing: slot 4 opens a hole.
    let e = fold(&mut d, &input(20_100, 1, 4, (7_000, 7_200, 6_900, 7_100))).unwrap();
    assert!(e.is_empty(), "no micro base and no rest across the hole");
    assert_eq!(
        d.refusals().get(Refusal::GapInBase),
        4,
        "micro2 and micro3, both directions"
    );
    assert_eq!(d.counts().rest_resets, 1);
    let e = fold(&mut d, &input(20_100, 1, 5, (7_000, 7_200, 6_900, 7_100))).unwrap();
    assert_eq!(of(&e, BaseKind::Micro2, TradeDir::Long).len(), 1);
    assert!(
        of(&e, BaseKind::Micro3, TradeDir::Long).is_empty(),
        "micro3 would span the hole"
    );
    assert_eq!(d.refusals().get(Refusal::GapInBase), 6);
    assert_eq!(
        of(&e, BaseKind::Rest, TradeDir::Long)[0]
            .snapshot
            .first_index,
        3
    );
}

#[test]
fn a_rest_of_exactly_two_overlapping_candles() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_200, 6_800, 7_100))).unwrap();
    // Touching at one price: 7,200 is the low of candle 1 and the high of 0.
    let e = fold(&mut d, &input(20_100, 1, 1, (7_250, 7_300, 7_200, 7_260))).unwrap();
    let rest = of(&e, BaseKind::Rest, TradeDir::Long);
    assert_eq!(rest.len(), 1);
    assert_eq!(
        (rest[0].snapshot.first_index, rest[0].snapshot.candles),
        (0, 2)
    );
    // One paisa apart: no overlap, no rest.
    let e = fold(&mut d, &input(20_100, 1, 2, (7_350, 7_400, 7_301, 7_390))).unwrap();
    assert!(of(&e, BaseKind::Rest, TradeDir::Long).is_empty());
    assert_eq!(
        of(&e, BaseKind::Micro2, TradeDir::Long).len(),
        1,
        "a micro base needs no overlap"
    );
}

#[test]
fn a_rest_as_long_as_the_whole_session_never_overflows_the_ring() {
    let session: u16 = 375;
    let mut d = Detector::new(params(session, 1, 1)).unwrap();
    let day = DayInput {
        long: DirectionDay {
            rest_lengths: &[300],
            touch_bp: Some(&[0]),
        },
        ..plain_day(20_100, session, 1)
    };
    d.begin_day(&day).unwrap();
    let mut last = Vec::new();
    for slot in 0..i64::from(session) {
        let high = if slot == 0 { 7_400 } else { 7_200 };
        last = fold(&mut d, &input(20_100, 1, slot, (7_100, high, 7_000, 7_150))).unwrap();
    }
    let rest = of(&last, BaseKind::Rest, TradeDir::Long);
    assert_eq!(
        rest[0].snapshot.candles, session,
        "the maximal rest is the session"
    );
    assert_eq!(rest[1].snapshot.candles, 300, "the generated length");
    assert_eq!(rest[0].touches, [1], "only the first candle reached 7,400");
    assert_eq!(
        rest[1].touches,
        [300],
        "every candle of the last 300 is at 7,200"
    );
    let over = input(20_100, 1, i64::from(session), (7_100, 7_200, 7_000, 7_150));
    assert_eq!(d.fold(&over).unwrap_err(), Refusal::SessionLongerThanRing);
    assert_eq!(d.refusals().get(Refusal::SessionLongerThanRing), 1);
    let longer = plain_day(20_101, session + 1, 1);
    assert_eq!(d.begin_day(&longer), Err(Refusal::SessionLongerThanRing));
    assert_eq!(
        d.fold(&input(20_101, 1, 0, (7_100, 7_200, 7_000, 7_150)))
            .unwrap_err(),
        Refusal::DayNotBegun,
        "a refused day is never begun"
    );
}

#[test]
fn the_two_pointer_rest_equals_a_naive_recomputation_on_100000_seeded_days() {
    let mut rng = Seeded(0x00F1_B300_0001);
    let mut d = Detector::new(params(64, 6, 3)).unwrap();
    let mut checked = 0;
    for day in 0..100_000 {
        let mut random = random_day(&mut rng, 40, day % 7 == 0);
        random.lengths = [Vec::new(), Vec::new()];
        random.tick = 1;
        checked += run_random_day(&mut d, 30_000 + day, &random, false);
    }
    assert!(checked > 5_000_000, "{checked} bases checked");
}

#[test]
fn every_measurement_and_score_equals_its_naive_recomputation() {
    let mut rng = Seeded(0x00F1_B300_0002);
    let mut d = Detector::new(params(64, 6, 3)).unwrap();
    let mut checked = 0;
    for day in 0..3_000 {
        let random = random_day(&mut rng, 48, day % 5 == 0);
        checked += run_random_day(&mut d, 40_000 + day, &random, true);
    }
    assert!(checked > 200_000, "{checked} bases checked");
    assert!(d.counts().touch_recounts > 0 && d.counts().zero_width_bases > 0);
    assert!(d.refusals().get(Refusal::NoTouchTolerance) > 0);
    assert!(d.refusals().get(Refusal::NoLadder) > 0);
}

/// FM-F024: days with runs of one-price candles never panic and match the
/// oracle's stated limits.
#[test]
fn zero_range_days_never_panic_and_match_the_oracle() {
    let mut rng = Seeded(0x00F1_B300_0003);
    let mut d = Detector::new(params(64, 6, 3)).unwrap();
    for day in 0..2_000 {
        let mut random = random_day(&mut rng, 30, true);
        // Every candle but the first flat at one walk price.
        for slot in random.slots.iter_mut().skip(1) {
            if rng.chance(700) {
                let p = slot.1.3;
                slot.1 = (p, p, p, p);
            }
        }
        run_random_day(&mut d, 50_000 + day, &random, true);
    }
    assert!(d.counts().zero_width_bases > 1_000);
    assert!(
        d.counts().ratio_over_zero > 0,
        "a flat first half under a ranged last half"
    );
}

#[test]
fn three_one_price_candles_give_flat_1000_and_set_the_flag() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200))).unwrap();
    let mut e = Vec::new();
    for slot in 1..4 {
        e = fold(
            &mut d,
            &input(20_100, 1, slot, (7_444, 7_444, 7_444, 7_444)),
        )
        .unwrap();
    }
    for kind in [BaseKind::Micro2, BaseKind::Micro3] {
        let base = &of(&e, kind, TradeDir::Long)[0].snapshot;
        assert!(base.measures.zero_width());
        assert_eq!(base.shape.get(Shape::PartialU), 1_000);
        assert_eq!(base.shape.get(Shape::VBottom), 0);
        assert_eq!(base.shape.get(Shape::InsidePair), 1_000);
        assert_eq!(
            base.measures.squeeze,
            Ratio::Value(1_000),
            "0 / 0 is one whole"
        );
    }
    let rest = &of(&e, BaseKind::Rest, TradeDir::Long)[0].snapshot;
    assert_eq!(
        rest.candles, 3,
        "the flat candles share no price with the first"
    );
    for (shape, limit) in [
        (Shape::Flat, 1_000),
        (Shape::TightCloses, 1_000),
        (Shape::ShakeoutReclaim, 0),
        (Shape::U, 0),
        (Shape::Triangle, 0),
    ] {
        assert_eq!(rest.shape.get(shape), limit, "{shape:?}");
    }
    assert_eq!(rest.measures.width_bp, Ratio::Value(0));
    assert!(d.counts().zero_width_bases >= 6);
    // The labelled other reading refuses them by name.
    let refusing = DetectorParams {
        zero_width: ZeroWidthReading::Refuse,
        ..params(8, 0, 1)
    };
    let mut r = Detector::new(refusing).unwrap();
    r.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut r, &input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200))).unwrap();
    let mut e = Vec::new();
    for slot in 1..4 {
        e = fold(
            &mut r,
            &input(20_100, 1, slot, (7_444, 7_444, 7_444, 7_444)),
        )
        .unwrap();
    }
    assert!(e.iter().all(|b| !b.snapshot.measures.zero_width()));
    assert!(r.refusals().get(Refusal::ZeroWidthBase) >= 6);
    assert_eq!(r.counts().zero_width_bases, 0);
}

/// FM-F022: touches are a pure function of the window, including after the
/// highest-high candle leaves it and while highs creep.
#[test]
fn touches_after_max_high_leaves_rest_equal_naive_recomputation() {
    let mut d = Detector::new(params(32, 1, 2)).unwrap();
    let day = DayInput {
        long: DirectionDay {
            rest_lengths: &[5],
            touch_bp: Some(&[0, 20]),
        },
        short: DirectionDay {
            rest_lengths: &[],
            touch_bp: Some(&[0, 20]),
        },
        ..plain_day(20_100, 32, 1)
    };
    d.begin_day(&day).unwrap();
    // Creeping highs (one paisa a candle, inside 20 bp of each other) under a
    // fixed band, then a tall candle that later leaves the 5-candle window.
    let highs = [
        9_000, 9_001, 9_002, 9_003, 9_004, 9_050, 9_005, 9_006, 9_007, 9_008, 9_009, 9_010,
    ];
    let mut oracle = Day::default();
    for (slot, high) in highs.iter().enumerate() {
        let candle = input(20_100, 1, slot as i64, (8_950, *high, 8_900, 8_960));
        let e = fold(&mut d, &candle).unwrap();
        oracle.candles.push(candle.candle.candle);
        oracle.closes.push(candle.candle.close_ts_micros);
        oracle.linked.push(slot > 0);
        let t = slot;
        let want = expected_bases(&oracle, t, [&[5], &[]]);
        assert_eq!(e.len(), want.len());
        for (base, want) in e.iter().zip(&want) {
            check_base(&oracle, base, want, [Some(&[0, 20]), Some(&[0, 20])]);
        }
    }
    assert!(d.counts().touch_recounts > 0);
    // The window rule restates the restart: after 9,050 the earlier creeping
    // highs (more than 20 bp below it) are no touches of the rest.
    let e = fold(&mut d, &input(20_100, 1, 12, (8_950, 9_011, 8_900, 8_960))).unwrap();
    let rest = of(&e, BaseKind::Rest, TradeDir::Long)[0];
    assert_eq!(rest.snapshot.high, 9_050);
    // 9,050 itself; 20 bp of 9,050 is 18.1 paisa: 9,032 and above only.
    assert_eq!(rest.touches, [1, 1]);
    let five = of(&e, BaseKind::Rest, TradeDir::Long)[1];
    assert_eq!(
        five.snapshot.high, 9_011,
        "9,050 left the five-candle window"
    );
    // 20 bp of 9,011 is 18.022: every high from 8,993 up, all five.
    assert_eq!(five.touches, [1, 5]);
}

#[test]
fn the_relation_is_exact_with_a_range_of_1001() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    // H = 3,001, L = 2,000: R = 1,001, the 382 level 2,618.618 exactly.
    fold(&mut d, &input(20_100, 1, 0, (2_500, 3_001, 2_000, 2_700))).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 1, (2_700, 2_800, 2_619, 2_700))).unwrap();
    let m = &of(&e, BaseKind::Micro2, TradeDir::Long)[0]
        .snapshot
        .measures;
    // The micro base's far edge is the day low: rung 1000.
    assert_eq!(m.fib_zone, 6);
    let e = fold(&mut d, &input(20_100, 1, 2, (2_700, 2_750, 2_619, 2_700))).unwrap();
    let m = &of(&e, BaseKind::Micro2, TradeDir::Long)[0]
        .snapshot
        .measures;
    assert_eq!(m.fib_zone, 2, "rung 382");
    assert_eq!(
        m.relation,
        LevelRelation::Above,
        "truncating the level would say spans"
    );
    assert!(same(m.level_distance, (382, 1_001)));
}

/// The short view is the long view on negated prices: the oracle above checks
/// the short view with its long-view code on negated candles; here a day and
/// its reflection `K - price` agree on every measurement that does not depend
/// on the price level.
#[test]
fn the_short_view_equals_the_long_view_on_negated_prices() {
    let mut rng = Seeded(0x00F1_B300_0004);
    for day in 0..500 {
        let random = random_day(&mut rng, 40, day % 3 == 0);
        let reflect = 300_000;
        let mirrored: Vec<Slot> = random
            .slots
            .iter()
            .map(|&(slot, (o, h, l, c))| {
                (slot, (reflect - o, reflect - l, reflect - h, reflect - c))
            })
            .collect();
        let zero: &[u64] = &[0];
        let run = |slots: &[Slot], long: &[u16], short: &[u16]| {
            let mut d = Detector::new(params(64, 6, 3)).unwrap();
            d.begin_day(&DayInput {
                ist_day: 60_000 + day,
                session_candles: 64,
                observed_tick: 1,
                long: DirectionDay {
                    rest_lengths: long,
                    touch_bp: Some(zero),
                },
                short: DirectionDay {
                    rest_lengths: short,
                    touch_bp: Some(zero),
                },
            })
            .unwrap();
            slots
                .iter()
                .map(|&(slot, ohlc)| fold(&mut d, &input(60_000 + day, 1, slot, ohlc)).unwrap())
                .collect::<Vec<_>>()
        };
        // The day's short lengths are the reflection's long lengths.
        let original = run(&random.slots, &random.lengths[0], &random.lengths[1]);
        let reflected = run(&mirrored, &random.lengths[1], &random.lengths[0]);
        for (a, b) in original.iter().zip(&reflected) {
            let shorts: Vec<&Emitted> = a
                .iter()
                .filter(|e| e.snapshot.direction == TradeDir::Short)
                .collect();
            let longs: Vec<&Emitted> = b
                .iter()
                .filter(|e| e.snapshot.direction == TradeDir::Long)
                .collect();
            assert_eq!(shorts.len(), longs.len());
            for (s, l) in shorts.iter().zip(&longs) {
                let (s, l) = (&s.snapshot, &l.snapshot);
                assert_eq!(
                    (s.kind, s.first_index, s.last_index),
                    (l.kind, l.first_index, l.last_index)
                );
                assert_eq!((s.high, s.low), (reflect - l.low, reflect - l.high));
                let (ms, ml) = (&s.measures, &l.measures);
                assert_eq!(ms.width, ml.width);
                assert_eq!((ms.range_sum, ms.body_sum), (ml.range_sum, ml.body_sum));
                assert_eq!(
                    (ms.slope_highs, ms.slope_lows),
                    (ml.slope_highs, ml.slope_lows)
                );
                assert_eq!(ms.curvature_closes, ml.curvature_closes);
                assert_eq!(ms.squeeze, ml.squeeze);
                assert_eq!(ms.holds, ml.holds);
                assert_eq!((ms.retrace, ms.fib_zone), (ml.retrace, ml.fib_zone));
                assert_eq!(
                    (ms.level_distance, ms.relation),
                    (ml.level_distance, ml.relation)
                );
                assert_eq!(s.shape, l.shape);
            }
            let s_touches: Vec<&Vec<u16>> = a
                .iter()
                .filter(|e| e.snapshot.direction == TradeDir::Short)
                .map(|e| &e.touches)
                .collect();
            let l_touches: Vec<&Vec<u16>> = b
                .iter()
                .filter(|e| e.snapshot.direction == TradeDir::Long)
                .map(|e| &e.touches)
                .collect();
            assert_eq!(
                s_touches, l_touches,
                "a zero tolerance does not depend on the level"
            );
        }
    }
}

#[test]
fn no_look_ahead_snapshots_are_bit_identical_when_later_candles_are_garbage() {
    let mut rng = Seeded(0x00F1_B300_0005);
    for day in 0..300 {
        let random = random_day(&mut rng, 40, day % 4 == 0);
        let cut = usize::try_from(rng.below(random.slots.len() as u64)).unwrap();
        let mut garbage = Seeded(0x00F1_B300_0100 + day as u64);
        let mut honest = Detector::new(params(64, 6, 3)).unwrap();
        let mut spoiled = Detector::new(params(64, 6, 3)).unwrap();
        let begin = DayInput {
            ist_day: 70_000 + day,
            session_candles: 64,
            observed_tick: random.tick,
            long: DirectionDay {
                rest_lengths: &random.lengths[0],
                touch_bp: random.tolerances[0].as_deref(),
            },
            short: DirectionDay {
                rest_lengths: &random.lengths[1],
                touch_bp: random.tolerances[1].as_deref(),
            },
        };
        honest.begin_day(&begin).unwrap();
        spoiled.begin_day(&begin).unwrap();
        for (i, &(slot, ohlc)) in random.slots.iter().enumerate() {
            let truth = fold(&mut honest, &input(70_000 + day, 1, slot, ohlc));
            // After the cut every candle is garbage: random prices, a third
            // of them corrupt (high below low).
            let fed = if i > cut {
                let p = 1 + i64::try_from(garbage.below(1_000_000)).unwrap();
                let q = i64::try_from(garbage.below(9_999)).unwrap();
                if garbage.chance(333) {
                    (p, p, p + q + 1, p)
                } else {
                    (p, p + q, p, p)
                }
            } else {
                ohlc
            };
            let seen = fold(&mut spoiled, &input(70_000 + day, 1, slot, fed));
            if i <= cut {
                assert_eq!(seen, truth, "day {day} candle {i} (cut {cut})");
                assert!(seen.is_ok());
            }
        }
    }
}

#[test]
fn the_day_resets_at_ist_day_including_a_stock_with_no_0915_candle() {
    let mut d = Detector::new(params(16, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 16, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_500, 6_500, 7_100))).unwrap();
    fold(&mut d, &input(20_100, 1, 1, (7_000, 7_200, 6_900, 7_100))).unwrap();
    // A candle of the next IST day before that day is begun is refused.
    let next = input(20_101, 1, 5, (7_000, 7_100, 6_950, 7_050));
    assert_eq!(d.fold(&next).unwrap_err(), Refusal::DayNotBegun);
    // The next day begins at 09:20: its first candle is alone and the
    // extremes are its own.
    d.begin_day(&plain_day(20_101, 16, 1)).unwrap();
    assert!(fold(&mut d, &next).unwrap().is_empty());
    let high = d.extremes().high().unwrap();
    assert_eq!((high.price, high.index), (7_100, Some(0)));
    assert_eq!(d.extremes().day(), Some(20_101));
    let e = fold(&mut d, &input(20_101, 1, 6, (7_050, 7_080, 7_000, 7_060))).unwrap();
    let rest = of(&e, BaseKind::Rest, TradeDir::Long)[0];
    assert_eq!((rest.snapshot.first_index, rest.snapshot.candles), (0, 2));
    assert_eq!(ist_day(next.candle.candle.ts_micros), 20_101);
}

#[test]
fn high_below_low_and_a_price_not_positive_are_refused_and_counted() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200))).unwrap();
    let inverted = input(20_100, 1, 1, (7_000, 6_000, 7_100, 7_050));
    assert_eq!(d.fold(&inverted).unwrap_err(), Refusal::CorruptCandle);
    let zero = input(20_100, 1, 1, (0, 7_000, 0, 10));
    assert_eq!(d.fold(&zero).unwrap_err(), Refusal::CorruptCandle);
    assert_eq!(d.refusals().get(Refusal::CorruptCandle), 2);
    assert_eq!(d.extremes().refused(), 2);
    // Nothing moved: the next good candle is contiguous with the first.
    let e = fold(&mut d, &input(20_100, 1, 1, (7_000, 7_100, 6_950, 7_050))).unwrap();
    assert_eq!(of(&e, BaseKind::Micro2, TradeDir::Long).len(), 1);
    assert_eq!(d.extremes().high().unwrap().price, 7_300);
    // A withheld update with a high below its low, or for another day.
    let bad = WithheldExtremes {
        open_ts_micros: open_of(20_100),
        close_ts_micros: open_of(20_100) + MINUTE,
        high: 10,
        low: 20,
        suspect_print: false,
    };
    assert_eq!(d.fold_withheld(&bad), Err(Refusal::CorruptCandle));
    let other_day = WithheldExtremes {
        open_ts_micros: open_of(20_200),
        ..bad
    };
    assert_eq!(d.fold_withheld(&other_day), Err(Refusal::DayNotBegun));
}

#[test]
fn every_candle_refusal_is_counted_by_name_and_changes_nothing() {
    let mut d = Detector::new(params(4, 0, 1)).unwrap();
    let good = input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200));
    assert_eq!(d.fold(&good).unwrap_err(), Refusal::DayNotBegun);
    d.begin_day(&plain_day(20_100, 4, 1)).unwrap();
    let mut partial = input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200));
    partial.candle.complete = false;
    assert_eq!(d.fold(&partial).unwrap_err(), Refusal::PartialCandle);
    let mut missing = good;
    missing.candle.missing_minutes = 1;
    assert_eq!(d.fold(&missing).unwrap_err(), Refusal::MissingMinutes);
    let mut wide = good;
    wide.candle.tf_minutes = 3;
    assert_eq!(d.fold(&wide).unwrap_err(), Refusal::WidthMismatch);
    fold(&mut d, &good).unwrap();
    assert_eq!(d.fold(&good).unwrap_err(), Refusal::OutOfOrder);
    for (refusal, count) in [
        (Refusal::DayNotBegun, 1),
        (Refusal::PartialCandle, 1),
        (Refusal::MissingMinutes, 1),
        (Refusal::WidthMismatch, 1),
        (Refusal::OutOfOrder, 1),
    ] {
        assert_eq!(d.refusals().get(refusal), count, "{}", refusal.name());
    }
    let e = fold(&mut d, &input(20_100, 1, 1, (7_000, 7_100, 6_950, 7_050))).unwrap();
    assert_eq!(
        of(&e, BaseKind::Micro2, TradeDir::Long).len(),
        1,
        "the refusals moved nothing"
    );
    // The width is refused at construction outside 1..=N_max.
    for tf in [0, 119] {
        let p = DetectorParams {
            tf_minutes: tf,
            ..params(4, 0, 1)
        };
        assert_eq!(Detector::new(p).unwrap_err(), Refusal::WidthBeyondTop);
    }
    let top = DetectorParams {
        tf_minutes: 118,
        ..params(4, 0, 1)
    };
    assert!(Detector::new(top).is_ok());
}

#[test]
fn every_day_refusal_is_counted_by_name() {
    let mut d = Detector::new(params(8, 2, 1)).unwrap();
    let no_tick = plain_day(20_100, 8, 0);
    assert_eq!(d.begin_day(&no_tick), Err(Refusal::NoObservedTick));
    let three = [4, 5, 6];
    let beyond = DayInput {
        short: DirectionDay {
            rest_lengths: &three,
            touch_bp: Some(&[0]),
        },
        ..plain_day(20_100, 8, 1)
    };
    assert_eq!(d.begin_day(&beyond), Err(Refusal::GenerationBeyondCapacity));
    let levels = DayInput {
        long: DirectionDay {
            rest_lengths: &[],
            touch_bp: Some(&[0, 1]),
        },
        ..plain_day(20_100, 8, 1)
    };
    assert_eq!(d.begin_day(&levels), Err(Refusal::GenerationBeyondCapacity));
    let unsorted = DayInput {
        long: DirectionDay {
            rest_lengths: &[6, 5],
            touch_bp: Some(&[0]),
        },
        ..plain_day(20_100, 8, 1)
    };
    assert_eq!(
        d.begin_day(&unsorted),
        Err(Refusal::RestLengthsNotAscending)
    );
    let repeated = DayInput {
        short: DirectionDay {
            rest_lengths: &[5, 5],
            touch_bp: Some(&[0]),
        },
        ..plain_day(20_100, 8, 1)
    };
    assert_eq!(
        d.begin_day(&repeated),
        Err(Refusal::RestLengthsNotAscending)
    );
    for (refusal, count) in [
        (Refusal::NoObservedTick, 1),
        (Refusal::GenerationBeyondCapacity, 2),
        (Refusal::RestLengthsNotAscending, 2),
    ] {
        assert_eq!(d.refusals().get(refusal), count, "{}", refusal.name());
    }
}

/// Each direction's generated values are its own (17:15 IST item 4): the
/// long and the short emit different rest lengths and count touches at
/// different tolerances; a direction with no tolerance carries no count.
#[test]
fn each_direction_reads_its_own_generated_values() {
    let mut d = Detector::new(params(32, 2, 2)).unwrap();
    let day = DayInput {
        ist_day: 20_100,
        session_candles: 32,
        observed_tick: 1,
        long: DirectionDay {
            rest_lengths: &[4, 6],
            touch_bp: Some(&[0, 50]),
        },
        short: DirectionDay {
            rest_lengths: &[5],
            touch_bp: None,
        },
    };
    d.begin_day(&day).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_400, 6_600, 7_100))).unwrap();
    let mut e = Vec::new();
    for slot in 1..8 {
        let high = 7_200 - slot;
        e = fold(&mut d, &input(20_100, 1, slot, (7_100, high, 7_000, 7_150))).unwrap();
    }
    let long: Vec<u16> = of(&e, BaseKind::Rest, TradeDir::Long)
        .iter()
        .map(|b| b.snapshot.candles)
        .collect();
    let short: Vec<u16> = of(&e, BaseKind::Rest, TradeDir::Short)
        .iter()
        .map(|b| b.snapshot.candles)
        .collect();
    assert_eq!(long, [8, 4, 6], "the maximal rest, then the long lengths");
    assert_eq!(short, [8, 5], "the maximal rest, then the short length");
    let long_rest = of(&e, BaseKind::Rest, TradeDir::Long)[0];
    assert_eq!(long_rest.touches, [1, 1], "7,400 alone: 50 bp is 37 paisa");
    let four = of(&e, BaseKind::Rest, TradeDir::Long)[1];
    assert_eq!(
        four.touches,
        [1, 4],
        "7,196 once at 0 bp; all four within 50 bp"
    );
    for b in of(&e, BaseKind::Rest, TradeDir::Short) {
        assert!(!b.snapshot.touches_known && b.touches.is_empty());
    }
    assert!(d.refusals().get(Refusal::NoTouchTolerance) > 0);
}

/// FM-F025: a width that misses a minute keeps the one-minute stream's day
/// extremes at every one of its closes, through the extremes-only update.
#[test]
fn a_withheld_bucket_keeps_every_width_on_the_one_minute_extremes() {
    let mut rng = Seeded(0x00F1_B300_0006);
    let mut withheld_extremes = 0;
    for day in 0..400 {
        let width: i64 = 2 + i64::try_from(rng.below(14)).unwrap();
        let buckets = 180 / width;
        let total = buckets * width;
        let mut mid = 123_457;
        let bars: Vec<Option<(i64, i64, i64, i64)>> = (0..total)
            .map(|_| {
                mid += i64::try_from(rng.below(61)).unwrap() - 30;
                let o = mid + i64::try_from(rng.below(11)).unwrap() - 5;
                let c = mid + i64::try_from(rng.below(11)).unwrap() - 5;
                let bar = (
                    o,
                    o.max(c) + i64::try_from(rng.below(9)).unwrap(),
                    o.min(c) - i64::try_from(rng.below(9)).unwrap(),
                    c,
                );
                (!rng.chance(80)).then_some(bar)
            })
            .collect();
        let d_number = 80_000 + day;
        let begin = |session: u16| plain_day(d_number, session, 1);
        let mut one = Detector::new(params(u16::try_from(total).unwrap(), 0, 1)).unwrap();
        one.begin_day(&begin(u16::try_from(total).unwrap()))
            .unwrap();
        let wide_params = DetectorParams {
            tf_minutes: u16::try_from(width).unwrap(),
            ..params(u16::try_from(buckets).unwrap(), 0, 1)
        };
        let mut wide = Detector::new(wide_params).unwrap();
        wide.begin_day(&begin(u16::try_from(buckets).unwrap()))
            .unwrap();
        for bucket in 0..buckets {
            let minutes_of = &bars[usize::try_from(bucket * width).unwrap()
                ..usize::try_from((bucket + 1) * width).unwrap()];
            for (m, bar) in minutes_of.iter().enumerate() {
                if let Some(ohlc) = bar {
                    let slot = bucket * width + i64::try_from(m).unwrap();
                    fold(&mut one, &input(d_number, 1, slot, *ohlc)).unwrap();
                }
            }
            let present: Vec<&(i64, i64, i64, i64)> = minutes_of.iter().flatten().collect();
            let open_ts = open_of(d_number) + bucket * width * MINUTE;
            if present.len() == minutes_of.len() {
                let ohlc = (
                    present[0].0,
                    present.iter().map(|b| b.1).max().unwrap(),
                    present.iter().map(|b| b.2).min().unwrap(),
                    present[present.len() - 1].3,
                );
                let wide_tf = u16::try_from(width).unwrap();
                fold(&mut wide, &input(d_number, wide_tf, bucket, ohlc)).unwrap();
            } else if !present.is_empty() {
                wide.fold_withheld(&WithheldExtremes {
                    open_ts_micros: open_ts,
                    close_ts_micros: open_ts + width * MINUTE,
                    high: present.iter().map(|b| b.1).max().unwrap(),
                    low: present.iter().map(|b| b.2).min().unwrap(),
                    suspect_print: false,
                })
                .unwrap();
            }
            assert_eq!(
                wide.extremes().high().map(|e| e.price),
                one.extremes().high().map(|e| e.price),
                "day {day} bucket {bucket}: H_N == H_1"
            );
            assert_eq!(
                wide.extremes().low().map(|e| e.price),
                one.extremes().low().map(|e| e.price),
                "day {day} bucket {bucket}: L_N == L_1"
            );
        }
        withheld_extremes += u64::from(
            wide.extremes()
                .high()
                .is_some_and(|e| e.from_withheld_bucket()),
        ) + u64::from(
            wide.extremes()
                .low()
                .is_some_and(|e| e.from_withheld_bucket()),
        );
    }
    assert!(
        withheld_extremes > 20,
        "{withheld_extremes} days ended on an extreme only a withheld bucket held"
    );
}

#[test]
fn a_ladder_from_a_withheld_bucket_or_a_suspect_print_is_stamped_and_counted() {
    let mut d = Detector::new(params(8, 0, 1)).unwrap();
    d.begin_day(&plain_day(20_100, 8, 1)).unwrap();
    fold(&mut d, &input(20_100, 1, 0, (7_000, 7_300, 6_900, 7_200))).unwrap();
    d.fold_withheld(&WithheldExtremes {
        open_ts_micros: open_of(20_100) + MINUTE,
        close_ts_micros: open_of(20_100) + 2 * MINUTE,
        high: 7_600,
        low: 7_000,
        suspect_print: false,
    })
    .unwrap();
    assert_eq!(d.counts().withheld_updates, 1);
    fold(&mut d, &input(20_100, 1, 2, (7_000, 7_100, 6_950, 7_050))).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 3, (7_000, 7_100, 6_950, 7_050))).unwrap();
    assert!(!e.is_empty());
    for b in &e {
        assert!(b.snapshot.ladder.extreme_from_withheld_bucket);
        assert!(!b.snapshot.ladder.frame_from_suspect_print);
        assert_eq!(
            b.snapshot.ladder.high, 7_600,
            "the hidden high is level 0 of the long frame"
        );
    }
    assert_eq!(d.counts().extreme_from_withheld_bucket, e.len() as u64);
    // A suspect print that makes the new low stamps the frame.
    let mut suspect = input(20_100, 1, 4, (6_900, 7_000, 6_000, 6_950));
    suspect.suspect_print = true;
    fold(&mut d, &suspect).unwrap();
    let e = fold(&mut d, &input(20_100, 1, 5, (6_900, 6_960, 6_800, 6_950))).unwrap();
    assert!(e.iter().all(|b| b.snapshot.ladder.frame_from_suspect_print));
    assert!(d.counts().frame_from_suspect_print >= e.len() as u64);
}

/// The oracle: on the same stream `DayExtremes`' high and low are the
/// endpoints `CurDayFib` holds.
#[test]
fn the_day_extremes_are_cur_day_fib_s_endpoints() {
    let mut rng = Seeded(0x00F1_B300_0007);
    let tolerance = vocab::Tolerance::from_milli(0).unwrap();
    for day in 0..300 {
        let random = random_day(&mut rng, 60, day % 2 == 0);
        let mut d = Detector::new(params(64, 0, 1)).unwrap();
        d.begin_day(&plain_day(90_000 + day, 64, 1)).unwrap();
        let mut fib = CurDayFib::new();
        for &(slot, ohlc) in &random.slots {
            let candle = input(90_000 + day, 1, slot, ohlc);
            fib.step(&candle.candle.candle, tolerance).unwrap();
            fold(&mut d, &candle).unwrap();
            let (h, l) = (
                d.extremes().high().unwrap().price,
                d.extremes().low().unwrap().price,
            );
            assert_eq!(fib.range(), h - l);
            match fib.leg() {
                Leg::Up => assert_eq!((fib.level(0), fib.level(1_000)), (Some(h), Some(l))),
                Leg::Down => assert_eq!((fib.level(0), fib.level(1_000)), (Some(l), Some(h))),
                Leg::Undetermined => {}
            }
        }
    }
}

#[test]
fn the_detector_is_deterministic() {
    let run = || {
        let mut rng = Seeded(0x00F1_B300_0008);
        let mut d = Detector::new(params(64, 6, 3)).unwrap();
        let mut all = Vec::new();
        for day in 0..200 {
            let random = random_day(&mut rng, 40, true);
            d.begin_day(&DayInput {
                ist_day: 95_000 + day,
                session_candles: 64,
                observed_tick: random.tick,
                long: DirectionDay {
                    rest_lengths: &random.lengths[0],
                    touch_bp: random.tolerances[0].as_deref(),
                },
                short: DirectionDay {
                    rest_lengths: &random.lengths[1],
                    touch_bp: random.tolerances[1].as_deref(),
                },
            })
            .unwrap();
            for &(slot, ohlc) in &random.slots {
                all.push(fold(&mut d, &input(95_000 + day, 1, slot, ohlc)));
            }
        }
        (all, *d.refusals(), *d.counts(), d.work())
    };
    assert_eq!(run(), run());
}

/// The per-(stock-day, width) state's fixed part, asserted when this test
/// crate compiles; the buffers are sized once from the calendar and the
/// generation (`Detector::state_bytes`).
const _: () = assert!(core::mem::size_of::<Detector>() <= 1_152);
const _: () = assert!(core::mem::size_of::<BaseSnapshot>() <= 400);

#[test]
fn the_state_is_sized_once_from_the_calendar_and_the_generation() {
    let small = Detector::new(params(75, 2, 1)).unwrap().state_bytes();
    let large = Detector::new(params(375, 2, 1)).unwrap().state_bytes();
    let more_q = Detector::new(params(375, 2, 4)).unwrap().state_bytes();
    assert!(small < large && large < more_q);
    // Five times the ring is less than five times the bytes: the fixed part
    // and the snapshot buffers do not grow with it.
    assert!(large < 5 * small);
}
