//! The tests of `indicators::williams_r` (PARTS-WILLR-CCI.md 5.5; D-8640 to D-8644).
//!
//! Two oracles share nothing with [`WilliamsR::fold`]: `scan`, the highest high and lowest
//! low of the last `period` bars found by a direct scan, and `published`, the published
//! definition `100 (C - HH) / (HH - LL)` transcribed as an exact rational. Fixtures WR-1 and
//! WR-2 are pinned candle by candle, and the staircase of PARTS-WILLR-CCI.md 5.3 is counted
//! white-box. Every random input comes from one deterministic xorshift generator with many
//! seeds and no dependency.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use super::*;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// A bar's high, low and close, in paisa.
type Hlc = (i64, i64, i64);

/// Fixture WR-1 (PARTS-WILLR-CCI.md 5.5): period 3.
const WR_1: [Hlc; 7] = [
    (110, 100, 105),
    (115, 104, 114),
    (113, 101, 102),
    (120, 108, 119),
    (118, 112, 112),
    (110, 100, 103),
    (121, 105, 120),
];

/// Fixture WR-2 (PARTS-WILLR-CCI.md 5.5): period 3, a window of one price at its third bar.
const WR_2: [Hlc; 5] = [
    (500, 500, 500),
    (500, 500, 500),
    (500, 500, 500),
    (501, 500, 501),
    (501, 499, 499),
];

/// One row of fixture WR-1's table, candle 2 on.
#[derive(Debug, PartialEq, Eq)]
struct Row {
    /// `HH` and `LL`.
    window: (i64, i64),
    /// `U = HH - C` and `R = HH - LL`.
    parts: (i128, i128),
    /// The floored ppm.
    ppm: Option<i32>,
    /// The exact %R as a reduced fraction, numerator and denominator.
    exact: (i128, i128),
    /// Above -85, and below -15.
    sides: (bool, bool),
    /// Crossed above -85, and below -15, from the previous candle's reading.
    crossed: (bool, bool),
}

/// Fixture WR-1's table from candle 2, where the first reading is, to candle 6.
const WR_1_TABLE: [Row; 5] = [
    Row {
        window: (115, 100),
        parts: (13, 15),
        ppm: Some(-866_667),
        exact: (-260, 3),
        sides: (false, true),
        crossed: (false, false),
    },
    Row {
        window: (120, 101),
        parts: (1, 19),
        ppm: Some(-52_632),
        exact: (-100, 19),
        sides: (true, false),
        crossed: (true, false),
    },
    Row {
        window: (120, 101),
        parts: (8, 19),
        ppm: Some(-421_053),
        exact: (-800, 19),
        sides: (true, true),
        crossed: (false, true),
    },
    Row {
        window: (120, 100),
        parts: (17, 20),
        ppm: Some(-850_000),
        exact: (-85, 1),
        sides: (false, true),
        crossed: (false, false),
    },
    Row {
        window: (121, 100),
        parts: (1, 21),
        ppm: Some(-47_620),
        exact: (-100, 21),
        sides: (true, false),
        crossed: (true, false),
    },
];

/// The long level, %R -85.
const LONG: i32 = -850_000;

/// The short level, %R -15.
const SHORT: i32 = -150_000;

/// A deterministic xorshift64 generator.
struct Xorshift(u64);

impl Xorshift {
    /// A generator from `seed`, made odd so it is never the all-zero state.
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// The next 64 bits.
    const fn draw(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A draw in `0..bound`, as an `i64`.
    fn below(&mut self, bound: u64) -> i64 {
        i64::try_from(self.draw() % bound).unwrap()
    }
}

/// A valid candle at `ts` with this high, low and close, its open at the close.
const fn candle(ts: i64, (high, low, close): Hlc) -> Candle {
    Candle {
        ts_micros: ts,
        open: close,
        high,
        low,
        close,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A fresh state at `period`.
fn willr(period: u16) -> WilliamsR {
    WilliamsR::new(period).unwrap()
}

/// A level of `ppm`.
fn level(ppm: i32) -> Level {
    Level::from_ppm(ppm).unwrap()
}

/// Folds `bars`, one minute apart, into `state`, and returns the reading after each.
fn fold_all(state: &mut WilliamsR, bars: &[Hlc]) -> Vec<Option<Reading>> {
    (0_i64..)
        .zip(bars)
        .map(|(minute, &bar)| {
            state.fold(&candle(minute * MINUTE, bar)).unwrap();
            state.reading()
        })
        .collect()
}

/// The readings of a fresh state at `period` after each of `bars`.
fn readings(period: u16, bars: &[Hlc]) -> Vec<Option<Reading>> {
    fold_all(&mut willr(period), bars)
}

/// The reading after the last of `bars` at `period`.
fn last(period: u16, bars: &[Hlc]) -> Reading {
    readings(period, bars).pop().flatten().unwrap()
}

/// The highest high and lowest low of the `period` bars ending at bar `t`, by a direct
/// scan, or `None` before `period` bars. It shares nothing with the deques.
fn scan(period: u16, bars: &[Hlc], t: usize) -> Option<(i64, i64)> {
    let first = (t + 1).checked_sub(usize::from(period))?;
    let window = &bars[first..=t];
    Some((
        window.iter().map(|bar| bar.0).max().unwrap(),
        window.iter().map(|bar| bar.1).min().unwrap(),
    ))
}

/// The published definition, `100 (C - HH) / (HH - LL)`, as the exact rational
/// `(numerator, denominator)` with a positive denominator, or `None` when `HH == LL`.
fn published(highest: i64, lowest: i64, close: i64) -> Option<(i128, i128)> {
    let den = i128::from(highest) - i128::from(lowest);
    (den != 0).then(|| (100 * (i128::from(close) - i128::from(highest)), den))
}

/// The published %R strictly above and strictly below `ppm` parts per million, read on
/// the exact rational: `num / den > ppm / 10^4` is `10^4 num > ppm den`.
fn published_sides(exact: Option<(i128, i128)>, ppm: i32) -> (bool, bool) {
    let side = |sign: i128| {
        exact.is_some_and(|(num, den)| sign * (10_000 * num - i128::from(ppm) * den) > 0)
    };
    (side(1), side(-1))
}

/// `len` random bars around a random price. Highs and lows are drawn from a band four
/// paisa wide, so equal highs and equal lows repeat; about one bar in eight has its high,
/// low and close at one price, and about one in eight starts a run of up to six such bars
/// at one price, so windows of one price occur at the small periods.
fn random_bars(rng: &mut Xorshift, len: usize) -> Vec<Hlc> {
    let mut out = Vec::with_capacity(len + 6);
    let mut base = 100 + rng.below(10_000);
    while out.len() < len {
        let kind = rng.below(8);
        let run = [1 + rng.below(6), 1][usize::from(kind > 0)];
        let one_price = [(base, base, base); 1];
        let high = base + rng.below(4);
        let low = (base - rng.below(4)).max(1);
        let close = low + rng.below(u64::try_from(high - low + 1).unwrap());
        let drawn = [(high, low, close); 1];
        let bar = [drawn, one_price][usize::from(kind < 2)];
        out.extend(std::iter::repeat_n(bar[0], usize::try_from(run).unwrap()));
        base = (base + rng.below(5) - 2).max(5);
    }
    out.truncate(len);
    out
}

#[test]
fn a_zero_period_is_refused_and_every_other_period_is_accepted() {
    assert_eq!(WilliamsR::new(0), Err(Refusal::PeriodZero));
    for period in [1, 14, u16::MAX] {
        let state = WilliamsR::new(period).unwrap();
        assert_eq!(state.period(), period);
        assert!(!state.warm());
        assert_eq!(state.reading(), None);
        assert!(state.highs.is_empty());
        assert!(state.lows.is_empty());
        assert!(state.highs.capacity() >= usize::from(period));
        assert!(state.lows.capacity() >= usize::from(period));
    }
}

#[test]
fn a_level_outside_minus_one_hundred_to_zero_is_refused_at_both_edges() {
    for ppm in [MIN_LEVEL_PPM, MAX_LEVEL_PPM, LONG, SHORT] {
        assert_eq!(Level::from_ppm(ppm).map(Level::ppm), Ok(ppm));
    }
    for ppm in [1, i32::MAX, MIN_LEVEL_PPM - 1, i32::MIN] {
        assert_eq!(Level::from_ppm(ppm), Err(Refusal::LevelOutOfRange { ppm }));
    }
}

#[test]
fn the_first_reading_appears_after_exactly_period_candles() {
    let bar = |k: i64| (1_000 + k % 7 * 3, 1_000 - k % 5 * 2, 1_000);
    for period in [1, 2, 3, 14, u16::MAX] {
        let bars: Vec<Hlc> = (0..i64::from(period)).map(bar).collect();
        let seen = readings(period, &bars);
        let (last, before) = seen.split_last().unwrap();
        assert!(before.iter().all(Option::is_none), "period {period}");
        let reading = last.unwrap();
        assert_eq!(reading.period(), period);
        // The window is the `period` bars, the current one included.
        let window = scan(period, &bars, bars.len() - 1);
        assert_eq!(Some((reading.highest(), reading.lowest())), window);
        assert_eq!(reading.close(), 1_000);
    }
}

#[test]
fn williams_r_matches_fixture_wr_1() {
    let seen = readings(3, &WR_1);
    assert_eq!(seen[..2], [None, None]);
    // Candle 2 has no previous reading to cross from.
    let mut previous = None;
    for (reading, row) in seen[2..].iter().zip(&WR_1_TABLE) {
        let reading = reading.unwrap();
        let (num, den) = published(reading.highest(), reading.lowest(), reading.close()).unwrap();
        let crossed = previous.map_or((false, false), |before: Reading| {
            (
                reading.crossed_above(&before, level(LONG)),
                reading.crossed_below(&before, level(SHORT)),
            )
        });
        let got = Row {
            window: (reading.highest(), reading.lowest()),
            parts: (reading.under(), reading.range()),
            ppm: reading.ppm(),
            // The exact value equals the table's reduced fraction.
            exact: [row.exact, (num, den)][usize::from(num * row.exact.1 != row.exact.0 * den)],
            sides: (reading.above(level(LONG)), reading.below(level(SHORT))),
            crossed,
        };
        assert_eq!(&got, row);
        assert_eq!(reading.period(), 3);
        assert!(!reading.flat());
        previous = Some(reading);
    }
}

#[test]
fn a_tie_at_the_level_is_neither_above_nor_below() {
    // WR-1 at candle 5 is exactly -85: U 17, R 20.
    let reading = last(3, &WR_1[..6]);
    assert_eq!((reading.under(), reading.range()), (17, 20));
    assert!(!reading.above(level(LONG)));
    assert!(!reading.below(level(LONG)));
    assert!(reading.above(level(LONG - 1)));
    assert!(!reading.below(level(LONG - 1)));
    assert!(reading.below(level(LONG + 1)));
    assert!(!reading.above(level(LONG + 1)));
}

#[test]
fn a_floored_ppm_equal_to_the_level_is_not_a_tie() {
    // Period 1: HH 2,000,001, LL 1,000,000, C 1,150,001, so U 850,000 and R 1,000,001. The
    // floor is -850,000, the level itself, yet the exact %R is -85,000,000 / 1,000,001 =
    // -84.999915...: above -85 and not below it, where a decision on `ppm() > level` would
    // read false.
    let reading = last(1, &[(2_000_001, 1_000_000, 1_150_001)]);
    assert_eq!((reading.under(), reading.range()), (850_000, 1_000_001));
    assert_eq!(reading.ppm(), Some(LONG));
    assert_eq!(PPM * 850_000, 850_000_000_000);
    assert_eq!(850_000 * 1_000_001, 850_000_850_000_i128);
    assert!(reading.above(level(LONG)));
    assert!(!reading.below(level(LONG)));
}

#[test]
fn a_flat_window_has_no_value_and_compares_false() {
    let seen = readings(3, &WR_2);
    let one_price = seen[2].unwrap();
    assert!(one_price.flat());
    assert_eq!(one_price.ppm(), None);
    assert_eq!((one_price.highest(), one_price.lowest()), (500, 500));
    for ppm in [MIN_LEVEL_PPM, LONG, SHORT, MAX_LEVEL_PPM] {
        assert!(!one_price.above(level(ppm)), "{ppm}");
        assert!(!one_price.below(level(ppm)), "{ppm}");
    }
    // The next candle reads 0: HH 501, LL 500, U 0, R 1.
    let next = seen[3].unwrap();
    assert!(!next.flat());
    assert_eq!((next.under(), next.range()), (0, 1));
    assert_eq!(next.ppm(), Some(0));
    assert!(next.above(level(LONG)));
    // Its fifth candle: HH 501, LL 499, U 2, R 2, the close on the window's low.
    let fifth = seen[4].unwrap();
    assert_eq!((fifth.under(), fifth.range()), (2, 2));
    assert_eq!(fifth.ppm(), Some(MIN_LEVEL_PPM));
    assert!(fifth.below(level(SHORT)));
}

#[test]
fn a_period_of_one_reads_the_candle_alone() {
    let bars = [
        (110, 90, 100),
        (100, 100, 100),
        (120, 100, 120),
        (120, 100, 100),
    ];
    let seen: Vec<Option<i32>> = readings(1, &bars)
        .into_iter()
        .map(|reading| reading.unwrap().ppm())
        .collect();
    assert_eq!(seen, [Some(-500_000), None, Some(0), Some(MIN_LEVEL_PPM)]);
    assert!(readings(1, &bars)[1].unwrap().flat());
}

#[test]
fn the_crossings_follow_tradingviews_crossover() {
    // WR-1: crossed above -85 at candles 3 and 6 only (6 after a previous reading exactly
    // on the level); crossed below -15 at candle 4 only.
    let seen = readings(3, &WR_1);
    let pairs: Vec<(Reading, Reading)> = seen[2..]
        .windows(2)
        .map(|pair| (pair[0].unwrap(), pair[1].unwrap()))
        .collect();
    let up: Vec<bool> = pairs
        .iter()
        .map(|(before, now)| now.crossed_above(before, level(LONG)))
        .collect();
    let down: Vec<bool> = pairs
        .iter()
        .map(|(before, now)| now.crossed_below(before, level(SHORT)))
        .collect();
    assert_eq!(up, [true, false, false, true]);
    assert_eq!(down, [false, true, false, false]);
    // WR-2: never from the reading of one price, either way, at any level; then crossed
    // below -15 at its fifth candle.
    let two = readings(3, &WR_2);
    let (one_price, fourth, fifth) = (two[2].unwrap(), two[3].unwrap(), two[4].unwrap());
    assert!(fourth.above(level(LONG)));
    for ppm in [MIN_LEVEL_PPM, LONG, SHORT, MAX_LEVEL_PPM] {
        assert!(!fourth.crossed_above(&one_price, level(ppm)));
        assert!(!fourth.crossed_below(&one_price, level(ppm)));
    }
    assert!(fifth.crossed_below(&fourth, level(SHORT)));
    // A previous reading of another period: false both ways, where one of its own period
    // with the same prices crosses.
    let other = Reading {
        period: 4,
        ..pairs[0].0
    };
    assert!(pairs[0].1.crossed_above(&pairs[0].0, level(LONG)));
    assert!(!pairs[0].1.crossed_above(&other, level(LONG)));
    let other = Reading {
        period: 2,
        ..fourth
    };
    assert!(!fifth.crossed_below(&other, level(SHORT)));
    // Random series: each crossing is the published `ta.crossover` / `ta.crossunder` on the
    // exact rationals, at the default levels and others.
    let mut rng = Xorshift::new(0x5EED_0009);
    let mut crossed = [0_usize; 2];
    for period in 1..=12 {
        let bars = random_bars(&mut rng, 120);
        for (t, pair) in (1..).zip(readings(period, &bars).windows(2)) {
            let (Some(before), Some(now)) = (pair[0], pair[1]) else {
                continue;
            };
            let then = scan(period, &bars, t - 1).unwrap();
            let at = scan(period, &bars, t).unwrap();
            for ppm in [MIN_LEVEL_PPM, -950_000, LONG, -500_000, SHORT, -100_000, 0] {
                let was = published_sides(published(then.0, then.1, bars[t - 1].2), ppm);
                let is = published_sides(published(at.0, at.1, bars[t].2), ppm);
                let known = published(then.0, then.1, bars[t - 1].2).is_some();
                let up = [known, !was.0, is.0]
                    .into_iter()
                    .all(std::convert::identity);
                let down = [known, !was.1, is.1]
                    .into_iter()
                    .all(std::convert::identity);
                assert_eq!(now.crossed_above(&before, level(ppm)), up);
                assert_eq!(now.crossed_below(&before, level(ppm)), down);
                crossed[0] += usize::from(up);
                crossed[1] += usize::from(down);
            }
        }
    }
    assert!(crossed[0] > 100, "{crossed:?}");
    assert!(crossed[1] > 100, "{crossed:?}");
}

#[test]
fn the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen() {
    // WR-1 candle 4, -800/19 = -42.105...: above -85 and below -15 at once. The module does
    // not make the two sides exclusive.
    let reading = last(3, &WR_1[..5]);
    assert_eq!(reading.ppm(), Some(-421_053));
    assert!(reading.above(level(LONG)));
    assert!(reading.below(level(SHORT)));
}

#[test]
fn every_reading_lies_between_minus_one_hundred_and_zero() {
    let mut rng = Xorshift::new(0x5EED_0011);
    let mut all = Vec::new();
    for seed in 0..200 {
        let period = 1 + u16::try_from(seed % 30).unwrap();
        let bars = random_bars(&mut rng, 90);
        all.extend(readings(period, &bars).into_iter().flatten());
    }
    let valued: Vec<Reading> = all.into_iter().filter(|r| !r.flat()).collect();
    assert!(valued.len() > 10_000, "{}", valued.len());
    for reading in &valued {
        let ppm = reading.ppm().unwrap();
        assert!(
            (MIN_LEVEL_PPM..=MAX_LEVEL_PPM).contains(&ppm),
            "{reading:?}"
        );
    }
    // A close on the window's high reads 0 and one on its low -1,000,000.
    let on_high: Vec<&Reading> = valued.iter().filter(|r| r.close() == r.highest()).collect();
    let on_low: Vec<&Reading> = valued.iter().filter(|r| r.close() == r.lowest()).collect();
    assert!(on_high.len() > 100, "{}", on_high.len());
    assert!(on_low.len() > 100, "{}", on_low.len());
    assert!(on_high.iter().all(|r| r.ppm() == Some(MAX_LEVEL_PPM)));
    assert!(on_low.iter().all(|r| r.ppm() == Some(MIN_LEVEL_PPM)));
}

#[test]
fn ppm_is_the_floor_of_the_exact_ratio() {
    let mut rng = Xorshift::new(0x5EED_0012);
    let mut checked = 0_usize;
    for seed in 0..200 {
        let period = 1 + u16::try_from(seed % 40).unwrap();
        let bars = random_bars(&mut rng, 90);
        for reading in readings(period, &bars).into_iter().flatten() {
            let (under, range) = (
                i128::from(reading.highest()) - i128::from(reading.close()),
                i128::from(reading.highest()) - i128::from(reading.lowest()),
            );
            let ppm = reading.ppm().map_or(0, i128::from);
            // `ppm R <= -10^6 U < (ppm + 1) R`, which a window of one price (R 0) cannot
            // meet: there `ppm` is None.
            assert_eq!(reading.ppm().is_some(), range > 0);
            assert!(ppm * range <= -PPM * under);
            assert!(-PPM * under < (ppm + 1) * range.max(1));
            checked += usize::from(range > 0);
        }
    }
    assert!(checked > 10_000, "{checked}");
}

#[test]
fn the_deques_equal_a_brute_force_window() {
    // 1,000 random series, periods 1 to 40, bars of one price and repeated highs and lows
    // mixed in: at every candle the reading is the scanned window's, and every comparison
    // at -85, -15 and -50 is the scanned window's.
    let mut rng = Xorshift::new(0x5EED_0013);
    let mut compared = 0_usize;
    for series in 0..1_000_u16 {
        let period = 1 + series % 40;
        let bars = random_bars(&mut rng, 60 + usize::from(series % 50));
        for (t, reading) in readings(period, &bars).into_iter().enumerate() {
            let window = scan(period, &bars, t);
            let close = bars[t].2;
            let expected = window.map(|(highest, lowest)| Reading {
                period,
                highest,
                lowest,
                close,
            });
            assert_eq!(reading, expected, "period {period}, candle {t}");
            for ppm in [LONG, SHORT, -500_000] {
                let exact = window.and_then(|(hh, ll)| published(hh, ll, close));
                let sides = reading.map(|r| (r.above(level(ppm)), r.below(level(ppm))));
                assert_eq!(sides, window.map(|_| published_sides(exact, ppm)));
            }
            compared += usize::from(reading.is_some());
        }
    }
    assert!(compared > 50_000, "{compared}");
}

#[test]
fn parity_with_the_published_definition() {
    // The published definition in exact rationals, written apart from the module: every
    // comparison against the rational, and `ppm` its floor, on random series and both
    // fixtures. Levels: the edges, the defaults, a ppm either side of them, and random.
    let mut rng = Xorshift::new(0x5EED_0014);
    let mut series: Vec<(u16, Vec<Hlc>)> = vec![(3, WR_1.to_vec()), (3, WR_2.to_vec())];
    for k in 0..300_u16 {
        series.push((1 + k % 25, random_bars(&mut rng, 40)));
    }
    let mut warm = 0_usize;
    for (period, bars) in &series {
        for (t, reading) in readings(*period, bars).into_iter().enumerate() {
            let Some((hh, ll)) = scan(*period, bars, t) else {
                assert_eq!(reading, None);
                continue;
            };
            let reading = reading.unwrap();
            let exact = published(hh, ll, bars[t].2);
            assert_eq!(reading.flat(), exact.is_none());
            assert_eq!(
                reading.ppm(),
                exact.map(|(num, den)| i32::try_from((10_000 * num).div_euclid(den)).unwrap())
            );
            let random = -i32::try_from(rng.below(1_000_001)).unwrap();
            for ppm in [
                MIN_LEVEL_PPM,
                LONG - 1,
                LONG,
                LONG + 1,
                SHORT,
                MAX_LEVEL_PPM,
                random,
            ] {
                let sides = (reading.above(level(ppm)), reading.below(level(ppm)));
                assert_eq!(sides, published_sides(exact, ppm), "{reading:?} at {ppm}");
            }
            warm += 1;
        }
    }
    assert!(warm > 8_000, "{warm}");
}

/// The staircase of PARTS-WILLR-CCI.md 5.3 at period `n`, fold `k` counted from the first
/// candle: the high is 1,000,000 when `k mod n = n - 1` and `900,000 - (k mod n)`
/// otherwise; the low is 10 below it; the open and the close are at the low; volume 0.
fn staircase(n: u16, k: u64) -> Candle {
    let step = k % u64::from(n);
    let falling = 900_000 - i64::try_from(step).unwrap();
    let high = [falling, 1_000_000][usize::from(step == u64::from(n) - 1)];
    let ts = i64::try_from(k).unwrap() * MINUTE;
    candle(ts, (high, high - 10, high - 10))
}

/// 1 when `deque`'s front is at least `period` candles old at candle `number`, else 0.
fn expiring(deque: &VecDeque<(u64, i64)>, number: u64, period: u64) -> usize {
    usize::from(
        deque
            .front()
            .is_some_and(|&(t, _)| number.wrapping_sub(t) >= period),
    )
}

/// What the staircase did to one deque, fold by fold: the back-pops, counted white-box as
/// `len_before + 1 - len_after - expired`, and the removals, `len_before + 1 - len_after`.
#[derive(Debug, Default)]
struct Pops {
    /// Back-pops per fold.
    back: Vec<usize>,
    /// Back-pops and the expiry per fold.
    removed: Vec<usize>,
}

impl Pops {
    /// Total back-pops, the most on one fold, candles above 100, and the value at sorted
    /// index `floor(0.99 n)`, counted from 0.
    fn figures(&self) -> (usize, usize, usize, usize) {
        let mut sorted = self.back.clone();
        sorted.sort_unstable();
        let p99 = sorted[sorted.len() * 99 / 100];
        let above = self.back.iter().filter(|&&pops| pops > 100).count();
        (self.back.iter().sum(), *sorted.last().unwrap(), above, p99)
    }
}

/// Folds the 20,000 staircase candles at `period` into a state whose candle number starts
/// at `start`, and returns what each deque did, highs then lows.
fn staircase_pops(period: u16, start: u64) -> [Pops; 2] {
    let mut state = willr(period);
    state.next = start;
    let n = u64::from(period);
    let mut pops = [Pops::default(), Pops::default()];
    for k in 0..20_000 {
        let before = [state.highs.len(), state.lows.len()];
        let expired = [
            expiring(&state.highs, state.next, n),
            expiring(&state.lows, state.next, n),
        ];
        state.fold(&staircase(period, k)).unwrap();
        let after = [state.highs.len(), state.lows.len()];
        for side in 0..2 {
            let removed = before[side] + 1 - after[side];
            pops[side].removed.push(removed);
            pops[side].back.push(removed - expired[side]);
        }
    }
    pops
}

#[test]
fn the_pops_stay_bounded_at_the_ninety_ninth_percentile() {
    // The model's back-pop figures (PARTS-WILLR-CCI.md 5.3 and 5.5), the same at both
    // starts: (period, highs (total, max, above 100, p99), lows (total, max, above 100,
    // p99), removals (highs total, highs max, lows total, lows max)).
    let pinned = [
        (
            2_000,
            (19_990, 1_999, 10, 0),
            (19_989, 1, 0, 1),
            (19_999, 2_000, 19_998, 2),
        ),
        (
            14,
            (18_564, 13, 0, 13),
            (18_571, 1, 0, 1),
            (19_991, 14, 19_998, 2),
        ),
    ];
    for (period, highs, lows, removals) in pinned {
        for start in [0, u64::MAX - 5] {
            let [high_pops, low_pops] = staircase_pops(period, start);
            assert_eq!(high_pops.figures(), highs, "period {period}, start {start}");
            assert_eq!(low_pops.figures(), lows, "period {period}, start {start}");
            let total = |pops: &Pops| pops.removed.iter().sum::<usize>();
            let most = |pops: &Pops| *pops.removed.iter().max().unwrap();
            let got = (
                total(&high_pops),
                most(&high_pops),
                total(&low_pops),
                most(&low_pops),
            );
            assert_eq!(got, removals, "period {period}, start {start}");
            let n = usize::from(period);
            for pops in [&high_pops, &low_pops] {
                let (back, _, above, _) = pops.figures();
                // Back-pops at most the candles folded, and fewer than 1% of candles above
                // 100.
                assert!(back <= 20_000);
                assert!(above * 100 < 20_000);
                // No fold removes more than `period` from a deque, the expiry included.
                assert!(pops.removed.iter().all(|&removed| removed <= n));
            }
            // The highs deque removes exactly `period` at every jump after the first.
            let jumps: Vec<usize> = high_pops.removed[n - 1..]
                .iter()
                .step_by(n)
                .copied()
                .collect();
            assert_eq!(jumps[0], n - 1);
            assert!(jumps[1..].iter().all(|&removed| removed == n));
            assert_eq!(jumps.len(), (20_000 - (n - 1)).div_ceil(n));
        }
    }
}

/// Checks the deques of `state` after a fold: every entry younger than `period` candles at
/// the last candle, the numbers rising and the highs falling, the lows rising, strictly,
/// front to back.
fn assert_window(state: &WilliamsR) {
    let last = state.next.wrapping_sub(1);
    let n = u64::from(state.period);
    for (deque, falls) in [(&state.highs, true), (&state.lows, false)] {
        let ages: Vec<u64> = deque.iter().map(|&(t, _)| last.wrapping_sub(t)).collect();
        assert!(ages.iter().all(|&age| age < n), "{ages:?}");
        assert!(ages.windows(2).all(|pair| pair[0] > pair[1]), "{ages:?}");
        let prices: Vec<i64> = deque.iter().map(|&(_, price)| price).collect();
        let order = [|a: i64, b: i64| a < b, |a: i64, b: i64| a > b][usize::from(falls)];
        assert!(
            prices.windows(2).all(|pair| order(pair[0], pair[1])),
            "{prices:?}"
        );
    }
}

#[test]
fn at_most_one_entry_expires_per_candle() {
    let mut rng = Xorshift::new(0x5EED_0016);
    for (k, period) in (0_u64..).zip([1_u16, 2, 3, 7, 14, 40]) {
        let mut state = willr(period);
        state.next = [0, u64::MAX - 20][usize::try_from(k % 2).unwrap()];
        let n = u64::from(period);
        let bars = random_bars(&mut rng, 400);
        let stairs = (0..300).map(|j| staircase(period, j));
        let candles = bars.iter().map(|&bar| candle(0, bar)).chain(stairs);
        for c in candles {
            // Before the fold, at most one entry of each deque is `period` or more candles
            // old at this candle's number: only the one numbered exactly `s - n` can be.
            for deque in [&state.highs, &state.lows] {
                let old = deque
                    .iter()
                    .filter(|&&(t, _)| state.next.wrapping_sub(t) >= n)
                    .count();
                assert!(old <= 1, "period {period}");
                assert!(old <= expiring(deque, state.next, n));
            }
            state.fold(&c).unwrap();
            assert_window(&state);
        }
    }
}

#[test]
fn the_deques_never_grow_past_their_period_or_reallocate() {
    // 100,000 folds: the first half with highs falling a paisa a candle (the highs deque
    // fills to `period`), the second with lows rising (the lows deque fills).
    let bar = |k: i64| {
        let falling = (10_000_000 - k, 10_000_000 - k - 10, 10_000_000 - k - 5);
        let rising = (k + 1_000, k, k + 5);
        [falling, rising][usize::from(k >= 50_000)]
    };
    for period in [1, 14, 1_000] {
        let mut state = willr(period);
        let capacity = (state.highs.capacity(), state.lows.capacity());
        let mut longest = (0, 0);
        for k in 0..100_000 {
            state.fold(&candle(k * MINUTE, bar(k))).unwrap();
            assert!(state.highs.len() <= usize::from(period));
            assert!(state.lows.len() <= usize::from(period));
            assert_eq!((state.highs.capacity(), state.lows.capacity()), capacity);
            longest = (
                longest.0.max(state.highs.len()),
                longest.1.max(state.lows.len()),
            );
        }
        assert_eq!(longest, (usize::from(period), usize::from(period)));
    }
}

#[test]
fn the_candle_counter_wraps_without_changing_a_reading() {
    let mut rng = Xorshift::new(0x5EED_0018);
    for period in [1, 3, 7, 14] {
        let bars = random_bars(&mut rng, 40);
        let mut from_zero = willr(period);
        let mut wrapping = willr(period);
        wrapping.next = u64::MAX - 5;
        assert_eq!(
            fold_all(&mut from_zero, &bars),
            fold_all(&mut wrapping, &bars)
        );
        // The window crossed the wrap: the numbers went past `u64::MAX` back to 0.
        assert_eq!(wrapping.next, 34);
        assert_window(&wrapping);
    }
}

#[test]
fn the_warm_counter_saturates_and_stays_warm() {
    let mut rng = Xorshift::new(0x5EED_0019);
    let bars = random_bars(&mut rng, 22);
    let mut state = willr(5);
    fold_all(&mut state, &bars[..20]);
    let mut twin = state.clone();
    state.seen = u32::MAX - 1;
    for (k, &bar) in (20..).zip(&bars[20..]) {
        state.fold(&candle(k * MINUTE, bar)).unwrap();
        twin.fold(&candle(k * MINUTE, bar)).unwrap();
        assert_eq!(state.seen, u32::MAX);
        assert!(state.warm());
        assert_eq!(state.reading(), twin.reading());
    }
    assert_eq!(twin.seen, 22);
}

#[test]
fn the_largest_prices_and_period_never_overflow() {
    // H `i64::MAX`, L 1, and C 1, `i64::MAX` and `i64::MAX - 1`, at periods 1 and 65,535.
    let top = i64::MAX;
    for period in [1, u16::MAX] {
        let mut state = willr(period);
        for k in 1..i64::from(period) {
            state.fold(&candle(k, (top, 1, top / 2))).unwrap();
        }
        let mut seen = Vec::new();
        for close in [1, top, top - 1] {
            state.fold(&candle(0, (top, 1, close))).unwrap();
            let reading = state.reading().unwrap();
            assert_eq!((reading.highest(), reading.lowest()), (top, 1));
            assert!(reading.under() <= reading.range());
            assert!(reading.range() < SPAN_LIMIT);
            assert_eq!(reading.below(level(MAX_LEVEL_PPM)), close < top);
            assert_eq!(reading.above(level(MIN_LEVEL_PPM)), close != 1);
            seen.push(reading.ppm());
        }
        assert_eq!(seen, [Some(MIN_LEVEL_PPM), Some(0), Some(-1)]);
    }
    // The const bounds, restated at the extremes.
    let span = i128::from(top) - 1;
    assert!(span < SPAN_LIMIT);
    assert!(PPM.checked_mul(span).unwrap() < PRODUCT_LIMIT);
    let widest = (-i128::from(MIN_LEVEL_PPM)).checked_mul(span).unwrap();
    assert!(widest < PRODUCT_LIMIT);
    assert!(-PPM * span > -PRODUCT_LIMIT);
    assert_eq!(SPAN_LIMIT.checked_mul(UNIT_LIMIT), Some(PRODUCT_LIMIT));
}

#[test]
fn a_price_gap_enters_the_window_and_leaves_it_after_period_candles() {
    // A 10% jump in the high at bar 20, on 61 bars, is HH for exactly `period` candles.
    let mut bars = vec![(100_100, 99_900, 100_000); 61];
    bars[20] = (110_110, 100_000, 110_000);
    for period in [1_u16, 3, 14] {
        let n = usize::from(period);
        let seen = readings(period, &bars);
        for (t, reading) in seen.iter().enumerate() {
            let window = scan(period, &bars, t);
            assert_eq!(reading.map(|r| (r.highest(), r.lowest())), window);
        }
        let gapped: Vec<usize> = (0..bars.len())
            .filter(|&t| seen[t].is_some_and(|r| r.highest() == 110_110))
            .collect();
        assert_eq!(gapped, (20..20 + n).collect::<Vec<_>>(), "period {period}");
    }
}

#[test]
fn a_time_gap_reads_as_contiguous_candles() {
    // Stamps a minute apart, a day apart, at random gaps, or receding: equal states.
    let mut rng = Xorshift::new(0x5EED_0022);
    for period in [1, 3, 14] {
        let bars = random_bars(&mut rng, 80);
        let gaps = [MINUTE, 1_440 * MINUTE, 7 * MINUTE + 13, -MINUTE];
        let mut states: Vec<WilliamsR> = gaps.iter().map(|_| willr(period)).collect();
        for (k, &bar) in (0_i64..).zip(&bars) {
            for (state, gap) in states.iter_mut().zip(gaps) {
                state.fold(&candle(k * gap, bar)).unwrap();
            }
            assert!(
                states.windows(2).all(|pair| pair[0] == pair[1]),
                "candle {k}"
            );
        }
    }
}

#[test]
fn no_reading_depends_on_a_later_candle() {
    let mut rng = Xorshift::new(0x5EED_0023);
    for period in [1, 3, 14] {
        let bars = random_bars(&mut rng, 60);
        let seen = readings(period, &bars);
        for t in [0, 5, 13, 14, 30, 59] {
            // Redraw every bar after t.
            let mut redrawn = bars[..=t].to_vec();
            redrawn.extend(random_bars(&mut rng, 59 - t));
            assert_eq!(
                readings(period, &redrawn)[t],
                seen[t],
                "period {period}, t {t}"
            );
        }
    }
}

#[test]
fn a_corrupt_candle_is_refused_and_changes_nothing() {
    let good = candle(0, (2_500_000, 2_500_000, 2_500_000));
    let corrupt = [
        (
            Candle {
                high: 2_400_000,
                ..good
            },
            Corrupt::HighBelowLow,
        ),
        (
            Candle {
                high: i64::MAX,
                low: -1,
                ..good
            },
            Corrupt::RangeOverflows,
        ),
        (
            Candle {
                open: 2_600_000,
                ..good
            },
            Corrupt::PriceOutsideRange,
        ),
        (Candle { volume: -1, ..good }, Corrupt::NegativeVolume),
        (candle(0, (0, 0, 0)), Corrupt::PriceNotPositive),
    ];
    let mut rng = Xorshift::new(0x5EED_0024);
    for period in [1, 3, 14] {
        let bars = random_bars(&mut rng, 30);
        let mut wrapped = willr(period);
        wrapped.next = u64::MAX;
        fold_all(&mut wrapped, &bars[..5]);
        // Fresh, one candle in, short of warm, warm, and warm across the wrap.
        let mut states = vec![willr(period), wrapped];
        for len in [1, usize::from(period) - 1, 30] {
            let mut state = willr(period);
            fold_all(&mut state, &bars[..len]);
            states.push(state);
        }
        for state in &states {
            for (bad, why) in corrupt {
                let mut refused = state.clone();
                assert_eq!(refused.fold(&bad), Err(Refusal::Corrupt(why)));
                assert_eq!(&refused, state);
            }
        }
    }
}

#[test]
fn the_state_is_a_fixed_size_and_allocates_only_in_new() {
    const fn copy<T: Copy>() {}
    copy::<Reading>();
    copy::<Level>();
    assert!(
        core::mem::size_of::<WilliamsR>() <= 128,
        "WilliamsR is {} bytes",
        core::mem::size_of::<WilliamsR>()
    );
    assert!(core::mem::size_of::<Reading>() <= 32);
    // Both capacities are the ones `new` reserved, through 10,000 folds.
    let mut rng = Xorshift::new(0x5EED_0025);
    for period in [1, 14, 200] {
        let mut state = willr(period);
        let capacity = (state.highs.capacity(), state.lows.capacity());
        for bar in random_bars(&mut rng, 10_000) {
            state.fold(&candle(0, bar)).unwrap();
            assert_eq!((state.highs.capacity(), state.lows.capacity()), capacity);
        }
    }
}

#[test]
fn the_previous_reading_judges_with_the_current_candle_excluded() {
    // WR-1: the reading kept at candle 4 is the window 2..=4, and folding candle 5 leaves
    // it so; candle 5's own reading has its low in it.
    let mut state = willr(3);
    let seen = fold_all(&mut state, &WR_1[..5]);
    let kept = seen[4].unwrap();
    state.fold(&candle(5 * MINUTE, WR_1[5])).unwrap();
    assert_eq!(kept, seen[4].unwrap());
    assert_eq!(Some((kept.highest(), kept.lowest())), scan(3, &WR_1, 4));
    assert_eq!((kept.highest(), kept.lowest()), (120, 101));
    let now = state.reading().unwrap();
    assert_eq!((now.highest(), now.lowest()), (120, 100));
    // Candle 5's close, 103, judged against the window before it: U 17, R 19, -89.47...,
    // below -85; its own reading is exactly -85, a tie, neither.
    let judged = Reading {
        close: WR_1[5].2,
        ..kept
    };
    assert_eq!((judged.under(), judged.range()), (17, 19));
    assert!(judged.below(level(LONG)));
    assert!(!now.below(level(LONG)));
    assert!(!now.above(level(LONG)));
}

#[test]
fn the_bench_measures_both_rows_at_tiny_sizes() {
    let shape = |rounds: &[Vec<u64>]| rounds.iter().map(Vec::len).collect::<Vec<_>>();
    assert_eq!(shape(&measure_step(20, 5, 3).unwrap()), [5, 5, 5]);
    // A history shorter than the period times steps that read nothing yet.
    assert_eq!(shape(&measure_step(0, 4, 2).unwrap()), [4, 4]);
    assert_eq!(shape(&measure_period(140, 3, 2).unwrap()), [3, 3]);
    assert_eq!(measure_period(0, 3, 2), Err(Refusal::PeriodZero));
    assert_eq!(measure_step(0, 0, 0), Ok(Vec::new()));
    // The candles it times are valid, and their highs and lows keep changing.
    let candles: Vec<Candle> = (0..10_000).map(bench_candle).collect();
    assert!(candles.iter().all(|c| c.check_evaluable().is_ok()));
    // Both deques hold several entries at times and pop from the back on about half the
    // folds (on this series at period 14: longest 9 and 9, popping on 5,278 and 5,420 of
    // the 10,000).
    let mut state = willr(STEP_PERIOD);
    let (mut longest, mut popped) = ([0_usize; 2], [0_usize; 2]);
    for c in &candles {
        let before = [state.highs.len(), state.lows.len()];
        state.fold(c).unwrap();
        let after = [state.highs.len(), state.lows.len()];
        for side in 0..2 {
            longest[side] = longest[side].max(after[side]);
            popped[side] += usize::from(after[side] <= before[side]);
        }
    }
    assert!(longest[0] > 3, "{longest:?}");
    assert!(longest[1] > 3, "{longest:?}");
    assert!(popped[0] > 4_000, "{popped:?}");
    assert!(popped[1] > 4_000, "{popped:?}");
    assert_eq!(BENCH_LEVEL.ppm(), LONG);
    assert_eq!(PERIOD_HISTORY, 20_000);
}

#[test]
fn a_cost_is_read_by_rank_and_judged_flat_or_not() {
    let sorted: Vec<u64> = (1..=200).collect();
    let read = StepCost {
        p50_ns: 100,
        p99_ns: 198,
        max_ns: 200,
        samples: 200,
    };
    assert_eq!(summarize(&sorted), read);
    assert_eq!(
        summarize(&[7]),
        StepCost {
            p50_ns: 7,
            p99_ns: 7,
            max_ns: 7,
            samples: 1
        }
    );
    assert_eq!(summarize(&[]), StepCost::default());
    let cost = |p99_ns| StepCost {
        p99_ns,
        ..StepCost::default()
    };
    // Within 3.0x of the first size and inside the budget.
    assert!(flat_within(
        &[cost(100), cost(300), cost(250)],
        3_000,
        2_000
    ));
    // A breach of the ceiling.
    assert!(!flat_within(&[cost(100), cost(301)], 3_000, 2_000));
    // Within the ceiling, over the budget.
    assert!(!flat_within(&[cost(1_000), cost(2_001)], 3_000, 2_000));
    // A zero first p99 fails, and so does a zero later one.
    assert!(!flat_within(&[cost(0), cost(0)], 3_000, 2_000));
    assert!(!flat_within(&[cost(100), cost(0)], 3_000, 2_000));
    // Nothing measured fails.
    assert!(!flat_within(&[], 3_000, 2_000));
}
