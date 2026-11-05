//! The tests of `indicators::rsi` (PARTS.md 3.5; D-8580 to D-8584).
//!
//! Two oracles share nothing with [`Rsi::fold`]: `recompute`, the integer rule of
//! PARTS.md 3.3 run from the whole close history with the change count taken from the
//! history's own index, and `exact_averages`, the published formulas transcribed in
//! exact rationals. Fixture RSI-1's table is pinned row by row. Every random input
//! comes from one deterministic xorshift generator with many seeds and no dependency.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use super::*;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// Fixture RSI-1 (PARTS.md 3.5): period 3, closes in paisa.
const RSI_1: [i64; 8] = [100, 103, 101, 104, 104, 98, 101, 107];

/// Fixture RSI-1's table from its fourth close on: G and L scaled, and `ppm`.
const RSI_1_TABLE: [(i128, i128, u32); 5] = [
    (2_000_000, 666_666, 750_000),
    (1_333_333, 444_444, 750_000),
    (888_888, 2_296_296, 279_069),
    (1_592_592, 1_530_864, 509_881),
    (3_061_728, 1_020_576, 750_000),
];

/// The closes of the long-run test (PARTS.md 3.5): a drift that ends at 110.
const DRIFT: [i64; 21] = [
    100, 103, 101, 104, 102, 105, 103, 106, 104, 107, 105, 108, 106, 109, 107, 110, 108, 111, 109,
    112, 110,
];

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

    /// A draw in `0..bound`.
    const fn below(&mut self, bound: u64) -> u64 {
        self.draw() % bound
    }
}

/// A valid candle at `ts` whose four prices are all `close`.
fn candle(ts: i64, close: i64) -> Candle {
    Candle {
        ts_micros: ts,
        open: close,
        high: close,
        low: close,
        close,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A fresh state at `period`.
fn rsi(period: u16) -> Rsi {
    Rsi::new(period).unwrap()
}

/// A level of `ppm`.
fn level(ppm: u32) -> Level {
    Level::from_ppm(ppm).unwrap()
}

/// Folds `closes`, one minute apart, and returns the state after each.
fn stream(period: u16, closes: &[i64]) -> Vec<Rsi> {
    let mut state = rsi(period);
    (0_i64..)
        .zip(closes)
        .map(|(minute, &close)| {
            state.fold(&candle(minute * MINUTE, close)).unwrap();
            state
        })
        .collect()
}

/// The state after folding every close of `closes`.
fn last(period: u16, closes: &[i64]) -> Rsi {
    stream(period, closes).pop().unwrap()
}

/// A reading's two averages.
const fn averages(reading: &Reading) -> (i128, i128) {
    (reading.avg_gain_scaled(), reading.avg_loss_scaled())
}

/// `len` closes of a random walk from a random start near Rs 1 to Rs 10,000: about a
/// third of them unchanged, the rest moving 1 to `step` paisa up or down, never below 1.
fn walk(rng: &mut Xorshift, len: usize, step: u64) -> Vec<i64> {
    let mut close = i64::try_from(100 + rng.below(1_000_000)).unwrap();
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(close);
        if rng.below(3) > 0 {
            let size = i64::try_from(1 + rng.below(step)).unwrap();
            close = if rng.below(2) == 0 {
                close + size
            } else {
                (close - size).max(1)
            };
        }
    }
    out
}

/// PARTS.md 3.3's integer rule recomputed from a whole close history, sharing nothing
/// with [`Rsi::fold`]: the moves are the history's own differences, the change count
/// is their index (never a saturating counter), and nothing is carried between calls.
/// `None` before `period` changes.
fn recompute(period: u16, closes: &[i64]) -> Option<(i128, i128)> {
    let n = i128::from(period);
    let moves: Vec<(i128, i128)> = closes
        .windows(2)
        .map(|pair| {
            let change = i128::from(pair[1]) - i128::from(pair[0]);
            (change.max(0) * SCALE, (-change).max(0) * SCALE)
        })
        .collect();
    let seeded = usize::from(period);
    if moves.len() < seeded {
        return None;
    }
    let (seed, wilder) = moves.split_at(seeded);
    let mut gain = seed.iter().map(|m| m.0).sum::<i128>().div_euclid(n);
    let mut loss = seed.iter().map(|m| m.1).sum::<i128>().div_euclid(n);
    for &(up, down) in wilder {
        gain += (up - gain).div_euclid(n);
        loss += (down - loss).div_euclid(n);
    }
    Some((gain, loss))
}

/// The published definition (research report A's D3) in exact rationals, in paisa:
/// after each close, `(gain numerator, loss numerator, shared denominator)`, `None`
/// before the seed. The seed is the simple mean of the first `period` moves; each later
/// move gives `avg + (x - avg) / n = ((n - 1) avg + x) / n`, so the denominator is
/// `n^(k + 1)` after `k` steps. Callers keep it inside `i128`.
fn exact_averages(period: u16, closes: &[i64]) -> Vec<Option<(i128, i128, i128)>> {
    let n = i128::from(period);
    let mut out = vec![None];
    let (mut gain_sum, mut loss_sum) = (0_i128, 0_i128);
    let mut exact = None;
    for (changes, pair) in (1_usize..).zip(closes.windows(2)) {
        let change = i128::from(pair[1]) - i128::from(pair[0]);
        let (up, down) = (change.max(0), (-change).max(0));
        exact = if let Some((gain, loss, den)) = exact {
            Some((
                (n - 1) * gain + up * den,
                (n - 1) * loss + down * den,
                den * n,
            ))
        } else {
            gain_sum += up;
            loss_sum += down;
            (changes == usize::from(period)).then_some((gain_sum, loss_sum, n))
        };
        out.push(exact);
    }
    out
}

#[test]
fn a_zero_period_is_refused_and_every_other_period_is_accepted() {
    assert_eq!(Rsi::new(0), Err(Refusal::PeriodZero));
    for period in [1, 14, u16::MAX] {
        let state = Rsi::new(period).unwrap();
        assert_eq!(state.period(), period);
        assert!(!state.warm());
        assert_eq!(state.reading(), None);
    }
}

#[test]
fn a_level_above_one_hundred_is_refused_at_the_edge() {
    for ppm in [0, MAX_LEVEL_PPM] {
        assert_eq!(Level::from_ppm(ppm).map(Level::ppm), Ok(ppm));
    }
    for ppm in [MAX_LEVEL_PPM + 1, u32::MAX] {
        assert_eq!(Level::from_ppm(ppm), Err(Refusal::LevelAboveMax { ppm }));
    }
}

#[test]
fn the_first_close_folds_no_change_and_reads_nothing() {
    for period in [1, 3, u16::MAX] {
        let mut state = rsi(period);
        state.fold(&candle(0, 2_500_000)).unwrap();
        let recorded = Rsi {
            period,
            previous: Some(2_500_000),
            changes: 0,
            gain: 0,
            loss: 0,
        };
        assert_eq!(state, recorded);
        assert!(!state.warm());
        assert_eq!(state.reading(), None);
    }
}

#[test]
fn the_reading_appears_after_exactly_period_changes() {
    for period in [1, 2, 3, 14, u16::MAX] {
        let mut rng = Xorshift::new(u64::from(period));
        let closes = walk(&mut rng, usize::from(period) + 1, 50);
        let (newest, earlier) = closes.split_last().unwrap();
        let mut state = rsi(period);
        for (minute, &close) in (0_i64..).zip(earlier) {
            state.fold(&candle(minute * MINUTE, close)).unwrap();
            assert_eq!(
                state.reading(),
                None,
                "period {period}: a reading before the seed"
            );
        }
        assert!(!state.warm());
        state.fold(&candle(0, *newest)).unwrap();
        assert!(state.warm());
        let reading = state.reading().unwrap();
        assert_eq!(reading.period(), period);
        assert_eq!(Some(averages(&reading)), recompute(period, &closes));
    }
}

#[test]
fn the_largest_period_smooths_after_its_seed() {
    // Period u16::MAX, u16::MAX + 4 closes: the seed at close u16::MAX (0-based) and three
    // Wilder steps after it. A u16 counter saturates at 65,535 = n and re-seeds at the
    // first of those three instead of stepping.
    let period = u16::MAX;
    let seed_at = usize::from(period);
    let mut rng = Xorshift::new(0x5EED_0005);
    let closes = walk(&mut rng, seed_at + 4, 500);
    assert!(closes[..=seed_at].windows(2).any(|pair| pair[1] > pair[0]));
    assert!(closes[..=seed_at].windows(2).any(|pair| pair[1] < pair[0]));
    let states = stream(period, &closes);
    assert_eq!(states[seed_at - 1].reading(), None);
    assert_eq!(
        states[seed_at].reading().map(|r| averages(&r)),
        recompute(period, &closes[..=seed_at])
    );
    for (index, state) in states.iter().enumerate().skip(seed_at + 1) {
        let reading = state.reading().unwrap();
        assert_eq!(
            Some(averages(&reading)),
            recompute(period, &closes[..=index])
        );
        assert_eq!(state.changes, u32::try_from(index).unwrap());
    }
    // White-box: a warm state at period 14 whose counter is one short of saturating
    // folds two more closes exactly as a copy with its true count does.
    let closes = walk(&mut rng, 32, 500);
    let warm = last(14, &closes[..30]);
    let mut near = warm;
    near.changes = u32::MAX - 1;
    let mut counted = warm;
    for (minute, &close) in (30_i64..).zip(&closes[30..]) {
        near.fold(&candle(minute * MINUTE, close)).unwrap();
        counted.fold(&candle(minute * MINUTE, close)).unwrap();
        assert_eq!((near.gain, near.loss), (counted.gain, counted.loss));
        assert_eq!(near.changes, u32::MAX);
    }
}

#[test]
fn the_seed_is_the_floored_mean_of_the_first_period_moves() {
    // Period 3: gains of 1 and 1 paisa, a loss of 1, and moves that leave each sum a
    // remainder of 2 or 1 thirds of a unit, so the floor shows.
    let up = last(3, &[100, 101, 102, 102]).reading().unwrap();
    assert_eq!(averages(&up), (666_666, 0));
    let down = last(3, &[100, 99, 99, 98]).reading().unwrap();
    assert_eq!(averages(&down), (0, 666_666));
    let mixed = last(3, &[100, 101, 100, 102]).reading().unwrap();
    assert_eq!(averages(&mixed), (1_000_000, 333_333));
    // Many random seeds at periods 1..=30: the sums taken in paisa and scaled after.
    let mut rng = Xorshift::new(0x5EED_0006);
    for period in 1..=30_u16 {
        for _ in 0..20 {
            let closes = walk(&mut rng, usize::from(period) + 1, 300);
            let reading = last(period, &closes).reading().unwrap();
            let (mut rises, mut falls) = (0_i128, 0_i128);
            for pair in closes.windows(2) {
                let change = i128::from(pair[1]) - i128::from(pair[0]);
                if change > 0 {
                    rises += change;
                } else {
                    falls -= change;
                }
            }
            let n = i128::from(period);
            let seed = ((rises * SCALE).div_euclid(n), (falls * SCALE).div_euclid(n));
            assert_eq!(
                averages(&reading),
                seed,
                "period {period}, closes {closes:?}"
            );
        }
    }
}

#[test]
fn wilder_smoothing_matches_fixture_rsi_1() {
    let states = stream(3, &RSI_1);
    for state in &states[..3] {
        assert!(!state.warm());
        assert_eq!(state.reading(), None);
    }
    for (state, &(gain, loss, ppm)) in states[3..].iter().zip(&RSI_1_TABLE) {
        let reading = state.reading().unwrap();
        assert_eq!(
            (
                reading.avg_gain_scaled(),
                reading.avg_loss_scaled(),
                reading.ppm()
            ),
            (gain, loss, ppm)
        );
    }
    assert_eq!(states.len(), 3 + RSI_1_TABLE.len());
}

#[test]
fn a_flat_series_reads_one_hundred_and_says_it_is_flat() {
    for period in [1, 2, 14] {
        let reading = last(period, &[2_500_000; 100]).reading().unwrap();
        assert!(reading.flat());
        assert!(!reading.faithful());
        assert_eq!(reading.ppm(), MAX_LEVEL_PPM);
        assert!(reading.above(level(999_999)));
        assert!(!reading.above(level(MAX_LEVEL_PPM)));
        for ppm in [0, 1, 500_000, 999_999, MAX_LEVEL_PPM] {
            assert!(!reading.below(level(ppm)));
        }
    }
}

#[test]
fn an_all_up_series_reads_one_hundred() {
    let closes: Vec<i64> = (0..40).map(|k| 100_000 + 7 * k).collect();
    for period in [1, 3, 14] {
        let reading = last(period, &closes).reading().unwrap();
        assert!(!reading.flat());
        assert_eq!(reading.avg_loss_scaled(), 0);
        assert_eq!(reading.ppm(), MAX_LEVEL_PPM);
        assert!(reading.above(level(999_999)));
        assert!(!reading.above(level(MAX_LEVEL_PPM)));
        assert!(!reading.below(level(0)));
        assert!(!reading.below(level(1)));
    }
}

#[test]
fn an_all_down_series_reads_zero() {
    let closes: Vec<i64> = (0..40).map(|k| 100_000 - 7 * k).collect();
    for period in [1, 3, 14] {
        let reading = last(period, &closes).reading().unwrap();
        assert!(!reading.flat());
        assert_eq!(reading.avg_gain_scaled(), 0);
        assert_eq!(reading.ppm(), 0);
        assert!(!reading.above(level(999_999)));
        assert!(!reading.above(level(MAX_LEVEL_PPM)));
        assert!(!reading.below(level(0)));
        assert!(reading.below(level(1)));
    }
}

#[test]
fn equal_moves_at_the_seed_read_fifty() {
    let reading = last(2, &[100, 101, 100]).reading().unwrap();
    assert_eq!(averages(&reading), (500_000, 500_000));
    assert_eq!(reading.ppm(), 500_000);
    assert!(!reading.above(level(500_000)));
    assert!(!reading.below(level(500_000)));
}

#[test]
fn a_period_of_one_reads_the_last_change_alone() {
    let states = stream(1, &[100, 105, 101, 101, 109]);
    assert_eq!(states[0].reading(), None);
    let readings: Vec<(i128, i128, u32, bool)> = states[1..]
        .iter()
        .map(|state| {
            let reading = state.reading().unwrap();
            (
                reading.avg_gain_scaled(),
                reading.avg_loss_scaled(),
                reading.ppm(),
                reading.flat(),
            )
        })
        .collect();
    assert_eq!(
        readings,
        [
            (5 * SCALE, 0, MAX_LEVEL_PPM, false),
            (0, 4 * SCALE, 0, false),
            (0, 0, MAX_LEVEL_PPM, true),
            (8 * SCALE, 0, MAX_LEVEL_PPM, false),
        ]
    );
}

#[test]
fn a_tie_at_the_level_is_neither_above_nor_below() {
    let reading = last(3, &RSI_1).reading().unwrap();
    assert_eq!(averages(&reading), (3_061_728, 1_020_576));
    assert_eq!(reading.avg_gain_scaled(), 3 * reading.avg_loss_scaled());
    assert!(!reading.above(level(750_000)));
    assert!(!reading.below(level(750_000)));
    assert!(reading.above(level(749_999)));
    assert!(reading.below(level(750_001)));
}

#[test]
fn a_floor_ppm_equal_to_the_level_is_not_a_tie() {
    // RSI-1's first two readings floor to 750,000 ppm, yet the floored L leaves the state
    // at 75.000018750004687... and 75.000014062506152... RSI points.
    let states = stream(3, &RSI_1);
    for (state, digits) in states[3..5]
        .iter()
        .zip([75_000_018_750_004_687_i128, 75_000_014_062_506_152])
    {
        let reading = state.reading().unwrap();
        let (gain, loss) = averages(&reading);
        assert_eq!(
            (100 * 10_i128.pow(15) * gain).div_euclid(gain + loss),
            digits
        );
        assert_eq!(reading.ppm(), 750_000);
        assert!(reading.above(level(750_000)));
        assert!(!reading.below(level(750_000)));
        assert!(!reading.above(level(750_001)));
    }
}

#[test]
fn ppm_is_the_floor_of_the_exact_ratio() {
    // `ppm (G + L) <= 10^6 G < (ppm + 1)(G + L)` on random states with `G + L > 0`: built
    // directly over the whole range, and streamed from random series.
    let mut rng = Xorshift::new(0x5EED_0014);
    let mut states: Vec<Reading> = (0..20_000)
        .map(|_| {
            let gain = i128::from(rng.draw() >> rng.below(64));
            let loss = i128::from(rng.draw() >> rng.below(64)) + i128::from(gain == 0);
            Reading {
                period: 14,
                avg_gain: gain,
                avg_loss: loss,
            }
        })
        .collect();
    for period in [1, 2, 3, 14, 30] {
        let closes = walk(&mut rng, 200, 2_000);
        states.extend(
            stream(period, &closes)
                .iter()
                .filter_map(Rsi::reading)
                .filter(|r| !r.flat()),
        );
    }
    assert!(states.len() > 20_000);
    for reading in states {
        let (gain, loss) = averages(&reading);
        let ppm = i128::from(reading.ppm());
        assert!(ppm * (gain + loss) <= PPM * gain, "{reading:?}");
        assert!(PPM * gain < (ppm + 1) * (gain + loss), "{reading:?}");
    }
}

#[test]
fn a_price_gap_is_one_move_and_nothing_more() {
    // A 10% overnight jump, then a 10% fall: each is one change, one Wilder step with that
    // move as its gain or loss, and the stream equals the brute force throughout.
    let mut rng = Xorshift::new(0x5EED_0015);
    let mut closes: Vec<i64> = walk(&mut rng, 30, 40)
        .iter()
        .map(|close| close + 100_000)
        .collect();
    let before = *closes.last().unwrap();
    let jump = before / 10;
    closes.push(before + jump);
    closes.push(before + jump - (before + jump) / 10);
    closes.extend([before; 5]);
    for period in [2, 14] {
        let states = stream(period, &closes);
        let n = i128::from(period);
        let (ahead, gapped) = (states[29], states[30]);
        assert_eq!(gapped.changes, ahead.changes + 1);
        let up = i128::from(jump) * SCALE;
        assert_eq!(gapped.gain, ahead.gain + (up - ahead.gain).div_euclid(n));
        assert_eq!(gapped.loss, ahead.loss + (-ahead.loss).div_euclid(n));
        for (index, state) in states.iter().enumerate() {
            assert_eq!(
                state.reading().map(|r| averages(&r)),
                recompute(period, &closes[..=index])
            );
        }
    }
}

#[test]
fn a_time_gap_reads_as_contiguous_candles() {
    // Stamps a minute apart, a day apart, at random gaps, and receding: the timestamp is
    // not read, so every state is equal.
    let mut rng = Xorshift::new(0x5EED_0016);
    let closes = walk(&mut rng, 60, 200);
    for period in [1, 5, 14] {
        let mut minute_apart = rsi(period);
        let mut day_apart = rsi(period);
        let mut gapped = rsi(period);
        let mut receding = rsi(period);
        let mut stamp = 0_i64;
        for (k, &close) in (0_i64..).zip(&closes) {
            stamp += i64::try_from(1 + rng.below(10_000)).unwrap() * MINUTE;
            minute_apart.fold(&candle(k * MINUTE, close)).unwrap();
            day_apart.fold(&candle(k * 1_440 * MINUTE, close)).unwrap();
            gapped.fold(&candle(stamp, close)).unwrap();
            receding.fold(&candle(-k * MINUTE, close)).unwrap();
            assert_eq!(day_apart, minute_apart);
            assert_eq!(gapped, minute_apart);
            assert_eq!(receding, minute_apart);
        }
    }
}

#[test]
fn no_reading_depends_on_a_later_candle() {
    // Redraw every candle after t: the state, and so the reading, at t is unchanged.
    let mut rng = Xorshift::new(0x5EED_0017);
    for period in [1, 3, 14] {
        let closes = walk(&mut rng, 50, 300);
        let states = stream(period, &closes);
        for (t, state) in states.iter().enumerate() {
            let mut redrawn = closes[..=t].to_vec();
            redrawn.extend(walk(&mut rng, 20, 5_000));
            let other = stream(period, &redrawn);
            assert_eq!(other[t], *state);
            assert_eq!(other[t].reading(), state.reading());
        }
    }
}

#[test]
fn the_stream_equals_a_from_scratch_recomputation() {
    // BRUTE FORCE: 1,000 random series at periods 1..=30; at every candle the streamed
    // averages equal the recomputation from the whole close history.
    let mut rng = Xorshift::new(0x5EED_0018);
    let mut readings = 0_u32;
    for series in 0..1_000_u16 {
        let period = 1 + series % 30;
        let len = usize::try_from(1 + rng.below(80)).unwrap();
        let step = 1 + rng.below(10_000);
        let closes = walk(&mut rng, len, step);
        for (index, state) in stream(period, &closes).iter().enumerate() {
            let streamed = state.reading().map(|r| averages(&r));
            assert_eq!(
                streamed,
                recompute(period, &closes[..=index]),
                "period {period}, closes {closes:?}"
            );
            readings += u32::from(streamed.is_some());
        }
    }
    assert!(
        readings > 10_000,
        "only {readings} warm readings were compared"
    );
}

#[test]
fn the_fixed_point_stays_within_period_units_of_exact_wilder() {
    // PARITY with the published definition: at every candle `0 <= SCALE g - G < n` and
    // the same for L, cross-multiplied by the exact denominator. The lengths keep the
    // denominator `n^(k + 1)` times SCALE times 1,000 paisa inside `i128`.
    let mut rng = Xorshift::new(0x5EED_0019);
    let mut compared = 0_u32;
    for (period, len) in [
        (1_u16, 80_usize),
        (2, 60),
        (3, 50),
        (5, 36),
        (14, 34),
        (30, 45),
    ] {
        for _ in 0..40 {
            let closes = walk(&mut rng, len, 1_000);
            let n = i128::from(period);
            for (state, exact) in stream(period, &closes)
                .iter()
                .zip(exact_averages(period, &closes))
            {
                assert_eq!(state.reading().is_some(), exact.is_some());
                if let (Some(reading), Some((gain, loss, den))) = (state.reading(), exact) {
                    let (big_gain, big_loss) = averages(&reading);
                    for excess in [SCALE * gain - big_gain * den, SCALE * loss - big_loss * den] {
                        assert!(
                            (0..n * den).contains(&excess),
                            "period {period}: {excess} of {den}"
                        );
                    }
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 5_000);
    // Fixture RSI-1's ppm column is the floor of its exact RSI.
    let exact = exact_averages(3, &RSI_1);
    for (row, &(_, _, ppm)) in exact[3..].iter().zip(&RSI_1_TABLE) {
        let (gain, loss, _) = row.unwrap();
        assert_eq!((PPM * gain).div_euclid(gain + loss), i128::from(ppm));
    }
}

/// What one long run of unchanged closes did to the integer state.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    /// The exact RSI's floor in ppm, which the run does not move.
    exact_floor: u32,
    /// The first unchanged close, counted from 1, whose reading is not faithful.
    first_unfaithful: Option<usize>,
    /// The first reading exactly 0 or 100: close, G, L, `ppm`, `flat()`, `faithful()`.
    departure: Option<(usize, i128, i128, u32, bool, bool)>,
    /// The first unchanged close whose reading has both averages 0.
    first_both_zero: Option<usize>,
}

/// Folds `head` and then `unchanged` closes equal to its last at `period`, and checks at
/// every unchanged close the ratio bound and that a faithful reading is within 1,000 ppm
/// of the exact floor; returns what it saw.
fn long_run(period: u16, head: &[i64], unchanged: usize) -> Run {
    let mut closes = head.to_vec();
    closes.extend(std::iter::repeat_n(*head.last().unwrap(), unchanged));
    let (gain, loss, _) = exact_averages(period, head).pop().flatten().unwrap();
    // The exact RSI as the ratio `num / den` of 1, computed once at the run's first close.
    let (num, den) = (gain, gain + loss);
    let exact_floor = u32::try_from((PPM * num).div_euclid(den)).unwrap();
    let n = i128::from(period);
    let mut run = Run {
        exact_floor,
        first_unfaithful: None,
        departure: None,
        first_both_zero: None,
    };
    for (k, state) in (1_usize..).zip(&stream(period, &closes)[head.len()..]) {
        let reading = state.reading().unwrap();
        let (big_gain, big_loss) = averages(&reading);
        let total = big_gain + big_loss;
        if total > 0 {
            assert!(
                (big_gain * den - num * total).abs() < n * den,
                "the ratio bound at close {k}"
            );
        }
        if reading.faithful() {
            assert!(
                reading.ppm().abs_diff(exact_floor) <= 1_000,
                "a faithful reading off at close {k}"
            );
        } else {
            run.first_unfaithful.get_or_insert(k);
        }
        if big_gain == 0 || big_loss == 0 {
            run.departure.get_or_insert((
                k,
                big_gain,
                big_loss,
                reading.ppm(),
                reading.flat(),
                reading.faithful(),
            ));
        }
        if reading.flat() {
            run.first_both_zero.get_or_insert(k);
        }
    }
    run
}

#[test]
fn a_long_unchanged_run_departs_only_where_the_ratio_bound_allows() {
    // PARTS.md 3.5 (repair 1, check1 B1): the drift, then `20 n + 60` closes of 110.
    let expected = [
        (2, 428_572, 11, (20, 0, 1, 0), 21),
        (5, 547_023, 28, (58, 1, 0, MAX_LEVEL_PPM), 59),
        (14, 593_601, 70, (159, 5, 0, MAX_LEVEL_PPM), 164),
    ];
    for (period, exact_floor, unfaithful, (k, gain, loss, ppm), both_zero) in expected {
        let unchanged = 20 * usize::from(period) + 60;
        let run = long_run(period, &DRIFT, unchanged);
        let pinned = Run {
            exact_floor,
            first_unfaithful: Some(unfaithful),
            departure: Some((k, gain, loss, ppm, false, false)),
            first_both_zero: Some(both_zero),
        };
        assert_eq!(run, pinned, "period {period}");
    }
    // Repair 2 (check2 N1): period 2, closes 100, 10,100, 10,099, then 40 of 10,099. The
    // exact RSI is 10,000/10,001, floor 999,900 ppm; the first 0/100 reading reads
    // faithful, 100 ppm from the exact floor.
    let run = long_run(2, &[100, 10_100, 10_099], 40);
    let pinned = Run {
        exact_floor: 999_900,
        first_unfaithful: Some(22),
        departure: Some((19, 9_536, 0, MAX_LEVEL_PPM, false, true)),
        first_both_zero: Some(33),
    };
    assert_eq!(run, pinned);
}

#[test]
fn the_faithful_edge_is_one_thousand_units_per_period() {
    // White-box readings: `G + L = 1,000 n - 1` is unfaithful and `1,000 n` faithful, as
    // all gain, all loss and split in two; both averages 0 is unfaithful at every period.
    for period in [1, 14, u16::MAX] {
        let edge = FAITHFUL_FACTOR * i128::from(period);
        for total in [edge - 1, edge] {
            for (gain, loss) in [(total, 0), (0, total), (total / 2, total - total / 2)] {
                let reading = Reading {
                    period,
                    avg_gain: gain,
                    avg_loss: loss,
                };
                assert_eq!(
                    reading.faithful(),
                    total == edge,
                    "period {period}, {gain} + {loss}"
                );
            }
        }
        assert!(
            !Reading {
                period,
                avg_gain: 0,
                avg_loss: 0
            }
            .faithful()
        );
    }
    // Through `fold` at period 14: 15 closes of 100, then 101, then 25 closes of 101.
    let mut closes = vec![100; 15];
    closes.extend([101; 26]);
    let states = stream(14, &closes);
    let seeded = states[14].reading().unwrap();
    assert!(seeded.flat() && !seeded.faithful());
    let moved = states[15].reading().unwrap();
    assert_eq!(averages(&moved), (71_428, 0));
    assert!(moved.faithful());
    for (k, state) in (1_usize..).zip(&states[16..]) {
        assert_eq!(
            state.reading().unwrap().faithful(),
            k <= 21,
            "after {k} unchanged closes of 101"
        );
    }
}

#[test]
fn the_largest_moves_and_period_never_overflow() {
    // Closes alternating 1 and `i64::MAX - 1`, every move the largest two evaluable
    // closes allow, at periods 1 and u16::MAX.
    let top = i64::MAX - 1;
    for period in [1, u16::MAX] {
        let mut state = rsi(period);
        for k in 0..usize::from(period) + 9 {
            let close = if k % 2 == 0 { 1 } else { top };
            let extreme = Candle {
                high: top,
                low: 1,
                ..candle(0, close)
            };
            state.fold(&extreme).unwrap();
            if let Some(reading) = state.reading() {
                let (gain, loss) = averages(&reading);
                assert!((0..GAIN_LIMIT).contains(&gain) && (0..GAIN_LIMIT).contains(&loss));
                assert!(reading.ppm() <= MAX_LEVEL_PPM);
            }
        }
        assert!(state.warm());
    }
    // The const bounds, restated at the extremes.
    let largest_gain = (i128::from(top) - 1).checked_mul(SCALE).unwrap();
    assert!(largest_gain < GAIN_LIMIT);
    assert!(largest_gain.checked_mul(i128::from(u16::MAX)).unwrap() < 1 << 99);
    assert!(PPM.checked_mul(largest_gain).unwrap() < 1 << 103);
    assert!(
        i128::from(MAX_LEVEL_PPM)
            .checked_mul(2 * largest_gain)
            .unwrap()
            < 1 << 105
    );
    assert!(FAITHFUL_FACTOR.checked_mul(i128::from(u16::MAX)).unwrap() < 1 << 26);
}

#[test]
fn a_corrupt_candle_is_refused_and_changes_nothing() {
    let good = candle(0, 2_500_000);
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
        (candle(0, 0), Corrupt::PriceNotPositive),
    ];
    let mut rng = Xorshift::new(0x5EED_0023);
    for period in [1, 3, 14] {
        let closes = walk(&mut rng, 30, 400);
        // Fresh, one close in, one change in, and warm.
        for state in [
            rsi(period),
            last(period, &closes[..1]),
            last(period, &closes[..2]),
            last(period, &closes),
        ] {
            for (bad, why) in corrupt {
                let mut refused = state;
                assert_eq!(refused.fold(&bad), Err(Refusal::Corrupt(why)));
                assert_eq!(refused, state);
            }
        }
    }
}

#[test]
fn the_state_is_a_fixed_size_and_never_allocates() {
    // `Copy` types own no heap memory, and neither needs dropping.
    const fn copy<T: Copy>() {}
    copy::<Rsi>();
    copy::<Reading>();
    assert!(
        core::mem::size_of::<Rsi>() <= 96,
        "Rsi is {} bytes",
        core::mem::size_of::<Rsi>()
    );
    assert!(core::mem::size_of::<Reading>() <= 48);
    assert!(!core::mem::needs_drop::<Rsi>());
    assert!(!core::mem::needs_drop::<Reading>());
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
    // The candles it times are valid, and consecutive closes always differ.
    let candles: Vec<Candle> = (0..10_000).map(bench_candle).collect();
    assert!(candles.iter().all(|c| c.check_evaluable().is_ok()));
    assert!(
        candles
            .windows(2)
            .all(|pair| pair[0].close != pair[1].close)
    );
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
