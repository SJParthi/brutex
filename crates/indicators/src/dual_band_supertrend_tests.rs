//! `dual_band_supertrend.rs`'s tests: the nine fixtures of PARTS 5.4 with the
//! latch folded beside the two-band form on every candle, the published rules
//! transcribed from scratch, the refusals, the extremes and the two bench rows at
//! tiny sizes. Every random series comes from a seeded xorshift, so a failure
//! replays exactly.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use super::*;
use crate::trend::Trend::{Down, Up};
use crate::trend::{SCALE, SuperTrend, TrendThresholds};

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// The candle size the published rules are judged on: high, low and close.
type Hlc = (i64, i64, i64);

/// A two-band reading as a fixture writes it: trend, line, upper, lower.
type Two = (Trend, i128, i128, i128);

/// One fixture row: the candle, the engine ATR's `value()`, the latch's trend
/// and stop, and the two-band reading (`None` before the ATR is warm).
struct Row {
    hlc: Hlc,
    atr: i64,
    latch: (Trend, i64),
    two: Option<Two>,
}

/// A fixture row, in the column order of PARTS 5.4.
const fn row(hlc: Hlc, atr: i64, latch: (Trend, i64), two: Option<Two>) -> Row {
    Row {
        hlc,
        atr,
        latch,
        two,
    }
}

/// The `i`th candle, a minute after the one before, opening at its close.
fn bar(i: usize, (high, low, close): Hlc) -> Candle {
    Candle {
        ts_micros: MINUTE * i64::try_from(i).unwrap(),
        open: close,
        high,
        low,
        close,
        volume: 1,
        open_interest: OI_NULL,
    }
}

/// The latch's thresholds at the same period and factor as the two-band form.
fn latch_thresholds(period: u16, factor: u32) -> TrendThresholds {
    TrendThresholds {
        atr_period: i128::from(period),
        supertrend_mult: i128::from(factor),
        ..TrendThresholds::CLASSICAL
    }
}

/// Folds `rows` through the two-band form, the latch and the engine ATR side by
/// side and asserts all three on every candle. `settled` is false on the first
/// reading only, and `atr()` reads exactly where the two-band form reads.
fn check_fixture(name: &str, period: u16, factor: u32, rows: &[Row]) {
    let mut two = DualBandSuperTrend::new(period, factor).expect("a fixture's period and factor");
    let mut latch = SuperTrend::new(latch_thresholds(period, factor));
    let mut engine = Atr::new(i128::from(period));
    let mut settled = false;
    for (i, row) in rows.iter().enumerate() {
        let candle = bar(i, row.hlc);
        two.fold(&candle).expect("a fixture candle is evaluable");
        latch.fold(&candle);
        engine.fold(&candle);
        assert_eq!(
            engine.value(),
            Some(row.atr),
            "{name} candle {i}: the engine ATR"
        );
        assert_eq!(
            (latch.trend(), latch.stop()),
            (row.latch.0, Some(row.latch.1)),
            "{name} candle {i}: the latch"
        );
        let want = row.two.map(|(trend, line, upper, lower)| Reading {
            trend,
            line,
            upper,
            lower,
            settled,
        });
        assert_eq!(two.reading(), want, "{name} candle {i}: the two-band form");
        assert_eq!(
            two.atr(),
            row.two.map(|_| row.atr),
            "{name} candle {i}: the ATR the bands are built from"
        );
        settled |= row.two.is_some();
    }
}

/// xorshift64*: a deterministic stream, so every random test replays exactly.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn draw(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A draw in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        self.draw() % n
    }

    /// A draw in `low..=high`.
    fn between(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low + 1).unwrap();
        low + i64::try_from(self.below(span)).unwrap()
    }

    /// A period in `1..=most`.
    fn period(&mut self, most: u16) -> u16 {
        u16::try_from(1 + self.below(u64::from(most))).unwrap()
    }

    /// A factor in `low..=high` thousandths.
    fn factor(&mut self, low: u32, high: u32) -> u32 {
        low + u32::try_from(self.below(u64::from(high - low + 1))).unwrap()
    }
}

/// A random walk of `len` evaluable candles from `base` paisa. A candle opens at
/// the last close or, one time in ten, gaps up to four swings away; it moves up
/// to `swing` and grows wicks up to `swing`. Small prices and swings make equal
/// bands and closes on a band common.
fn walk(rng: &mut Rng, len: usize, base: i64, swing: i64) -> Vec<Candle> {
    let mut out = Vec::with_capacity(len);
    let mut close = base;
    for i in 0..len {
        let gap = if rng.below(10) == 0 {
            rng.between(-4 * swing, 4 * swing)
        } else {
            0
        };
        let open = (close + gap).max(1);
        close = (open + rng.between(-swing, swing)).max(1);
        let high = open.max(close) + rng.between(0, swing);
        let low = (open.min(close) - rng.between(0, swing)).max(1);
        out.push(Candle {
            ts_micros: MINUTE * i64::try_from(i).unwrap(),
            open,
            high,
            low,
            close,
            volume: 1,
            open_interest: OI_NULL,
        });
    }
    out
}

/// A random series of the three shapes the random tests mix: a few paisa near
/// Rs 1 to 3, tens of paisa near Rs 100, and rupees near Rs 25,000.
fn random_series(rng: &mut Rng, len: usize) -> Vec<Candle> {
    match rng.below(3) {
        0 => {
            let base = rng.between(100, 300);
            let swing = rng.between(1, 5);
            walk(rng, len, base, swing)
        }
        1 => walk(rng, len, 10_000, 50),
        _ => walk(rng, len, 2_500_000, 5_000),
    }
}

/// PARTS 5.3, transcribed from scratch in the script's own shape: a series over
/// the whole history, in which every value the script reads as missing is
/// `None`, its `nz()` is `unwrap_or(0)`, and a comparison with a missing close
/// is false. The ATR is `ta.atr`'s `rma` of `ta.tr(true)`, seeded by the simple
/// mean of the first `period` true ranges, held scaled by `SCALE` and floored at
/// every step. Returns the ATR the bands use and the reading at the last candle.
fn transcribe(candles: &[Candle], period: u16, factor: u32) -> (Option<i64>, Option<Reading>) {
    let length = usize::from(period);
    let count = i128::from(period);
    let high: Vec<i128> = candles
        .iter()
        .map(|candle| i128::from(candle.high))
        .collect();
    let low: Vec<i128> = candles
        .iter()
        .map(|candle| i128::from(candle.low))
        .collect();
    let close: Vec<i128> = candles
        .iter()
        .map(|candle| i128::from(candle.close))
        .collect();
    let size = candles.len();
    let mut range = vec![0_i128; size];
    let mut scaled: Vec<Option<i128>> = vec![None; size];
    for i in 0..size {
        range[i] = if i == 0 {
            high[0] - low[0]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        scaled[i] = match (i + 1).cmp(&length) {
            std::cmp::Ordering::Less => None,
            std::cmp::Ordering::Equal => {
                Some((range[..length].iter().sum::<i128>() * SCALE).div_euclid(count))
            }
            std::cmp::Ordering::Greater => {
                scaled[i - 1].map(|before| before + (range[i] * SCALE - before).div_euclid(count))
            }
        };
    }
    let atr: Vec<Option<i128>> = scaled
        .iter()
        .map(|held| held.map(|value| value.div_euclid(SCALE)))
        .collect();
    let mut upper: Vec<Option<i128>> = vec![None; size];
    let mut lower: Vec<Option<i128>> = vec![None; size];
    let mut down: Vec<Option<bool>> = vec![None; size];
    for i in 0..size {
        let Some(average) = atr[i] else { continue };
        // `hl2`, floored as PARTS 5.3 floors it: written apart from the
        // module's `midpoint`, and equal to it on positive prices.
        let mid = (high[i] + low[i]).div_euclid(2);
        let band = (average * i128::from(factor)).div_euclid(1000);
        let earlier = |series: &[Option<i128>]| if i == 0 { None } else { series[i - 1] };
        let upper_before = earlier(&upper).unwrap_or(0);
        let lower_before = earlier(&lower).unwrap_or(0);
        let close_before = if i == 0 { None } else { Some(close[i - 1]) };
        let new_upper = if mid + band < upper_before
            || close_before.is_some_and(|before| before > upper_before)
        {
            mid + band
        } else {
            upper_before
        };
        let new_lower = if mid - band > lower_before
            || close_before.is_some_and(|before| before < lower_before)
        {
            mid - band
        } else {
            lower_before
        };
        // `na(atr[1])` reads down; after it the previous direction picks the
        // band judged (D-8602).
        let is_down = if i == 0 || atr[i - 1].is_none() {
            true
        } else if down[i - 1] == Some(true) {
            close[i] <= new_upper
        } else {
            close[i] < new_lower
        };
        upper[i] = Some(new_upper);
        lower[i] = Some(new_lower);
        down[i] = Some(is_down);
    }
    let last = size - 1;
    let reading = down[last].map(|is_down| {
        let (band_up, band_down) = (upper[last].unwrap(), lower[last].unwrap());
        Reading {
            trend: if is_down { Down } else { Up },
            line: if is_down { band_up } else { band_down },
            upper: band_up,
            lower: band_down,
            settled: last > 0 && atr[last - 1].is_some(),
        }
    });
    (
        atr[last].map(|paisa| i64::try_from(paisa).unwrap()),
        reading,
    )
}

/// Every reading of `candles`, one per candle, from a fresh state.
fn readings(candles: &[Candle], period: u16, factor: u32) -> Vec<Option<Reading>> {
    let mut two = DualBandSuperTrend::new(period, factor).unwrap();
    candles
        .iter()
        .map(|candle| {
            two.fold(candle).unwrap();
            two.reading()
        })
        .collect()
}

#[test]
fn a_zero_period_and_a_factor_outside_its_range_are_refused() {
    assert_eq!(DualBandSuperTrend::new(0, 3_000), Err(Refusal::PeriodZero));
    assert_eq!(
        DualBandSuperTrend::new(0, 0),
        Err(Refusal::PeriodZero),
        "the period is judged first"
    );
    for milli in [0, MAX_FACTOR_MILLI + 1, u32::MAX] {
        assert_eq!(
            DualBandSuperTrend::new(10, milli),
            Err(Refusal::FactorOutOfRange { milli }),
            "a factor of {milli} thousandths is refused by name"
        );
    }
    let candles = walk(&mut Rng::new(7), 12, 10_000, 50);
    for milli in [1, MAX_FACTOR_MILLI] {
        let mut two = DualBandSuperTrend::new(10, milli).expect("the edge factors are accepted");
        for candle in &candles {
            assert_eq!(two.fold(candle), Ok(()), "factor {milli} folds");
        }
        assert!(two.reading().is_some(), "factor {milli} reads once warm");
    }
}

#[test]
fn nothing_is_read_before_the_atr_has_its_period() {
    for period in [1, 2, 10, u16::MAX] {
        let length = usize::from(period);
        let candles = walk(&mut Rng::new(u64::from(period)), length, 10_000, 50);
        let mut two = DualBandSuperTrend::new(period, 3_000).unwrap();
        for (i, candle) in candles[..length - 1].iter().enumerate() {
            two.fold(candle).unwrap();
            assert_eq!(
                two.reading(),
                None,
                "period {period}: candle {i} reads nothing"
            );
            assert_eq!(
                two.atr(),
                None,
                "period {period}: candle {i} has no warm ATR"
            );
        }
        two.fold(&candles[length - 1]).unwrap();
        let first = two
            .reading()
            .expect("the ATR's own period gives the first reading");
        assert!(
            !first.settled,
            "period {period}: the first reading is unsettled"
        );
        assert!(two.atr().is_some(), "period {period}: the ATR is warm");
    }
}

#[test]
fn the_first_atr_candle_is_down_on_the_upper_band_and_unsettled() {
    for (seed, period) in (1..=40_u64).zip([1, 2, 3, 10, 20].into_iter().cycle()) {
        let mut rng = Rng::new(seed);
        let factor = rng.factor(500, 5_000);
        let candles = random_series(&mut rng, usize::from(period) + 1);
        let mut two = DualBandSuperTrend::new(period, factor).unwrap();
        let first_at = usize::from(period) - 1;
        for candle in &candles[..=first_at] {
            two.fold(candle).unwrap();
        }
        let first = two.reading().unwrap();
        let candle = candles[first_at];
        let mid = (i128::from(candle.high) + i128::from(candle.low)).div_euclid(2);
        let band = (i128::from(two.atr().unwrap()) * i128::from(factor)).div_euclid(1000);
        // Prior bands of 0, as `nz()` gives them; with no previous close at
        // period 1 nothing crosses that 0 upper band, so it stays.
        let upper = if period == 1 { 0 } else { mid + band };
        let want = Reading {
            trend: Down,
            line: upper,
            upper,
            lower: (mid - band).max(0),
            settled: false,
        };
        assert_eq!(first, want, "seed {seed}, period {period}, factor {factor}");
        two.fold(&candles[first_at + 1]).unwrap();
        assert!(
            two.reading().unwrap().settled,
            "seed {seed}: the next reading is settled"
        );
    }
    // A band wider than the price: the basic lower band is below 0, so the
    // lower band stays at the zero prior band.
    let wide = [(200, 100, 150), (210, 100, 200), (205, 195, 200)];
    let rows = [
        row(wide[0], 100, (Up, -9_850), None),
        row(wide[1], 105, (Up, -9_850), Some((Down, 10_655, 10_655, 0))),
        row(wide[2], 57, (Up, -5_500), Some((Down, 5_900, 5_900, 0))),
    ];
    check_fixture("a band wider than the price", 2, MAX_FACTOR_MILLI, &rows);
}

#[test]
fn period_one_reads_the_published_zero_band_first() {
    // ST-6: the previous band is 0 and there is no previous close, so the first
    // upper band is 0 (PARTS 5.3); the latch seeds at its basic band.
    let rows = [
        row((110, 90, 100), 20, (Up, 80), Some((Down, 0, 0, 80))),
        row((106, 100, 105), 6, (Up, 97), Some((Down, 109, 109, 97))),
        row((109, 104, 108), 5, (Up, 101), Some((Down, 109, 109, 101))),
    ];
    check_fixture("ST-6", 1, 1_000, &rows);
}

#[test]
fn both_bands_ratchet_on_every_candle_and_reset_as_published() {
    // [upper, lower] counts of a band held, ratcheted and reset after a cross.
    let (mut held, mut ratcheted, mut reset) = ([0_u32; 2], [0_u32; 2], [0_u32; 2]);
    for seed in 1..=200_u64 {
        let mut rng = Rng::new(seed ^ 0xBA4D);
        let period = rng.period(20);
        let factor = rng.factor(500, 5_000);
        let candles = random_series(&mut rng, usize::from(period) + 60);
        let mut two = DualBandSuperTrend::new(period, factor).unwrap();
        let mut prior: Option<(Reading, i128)> = None;
        for (t, candle) in candles.iter().enumerate() {
            two.fold(candle).unwrap();
            let Some(now) = two.reading() else { continue };
            let mid = (i128::from(candle.high) + i128::from(candle.low)).div_euclid(2);
            let band = (i128::from(two.atr().unwrap()) * i128::from(factor)).div_euclid(1000);
            if let Some((before, close)) = prior {
                let at = format!("seed {seed}, candle {t}");
                // A membership test rather than `a || b`: a short-circuit inside
                // an assertion leaves failing sides a green run never takes.
                let uppers = [mid + band, before.upper];
                let lowers = [mid - band, before.lower];
                assert!(
                    uppers.contains(&now.upper),
                    "{at}: the upper band {uppers:?}"
                );
                assert!(
                    lowers.contains(&now.lower),
                    "{at}: the lower band {lowers:?}"
                );
                if close <= before.upper {
                    assert!(
                        now.upper <= before.upper,
                        "{at}: the upper band rose uncrossed"
                    );
                }
                if close >= before.lower {
                    assert!(
                        now.lower >= before.lower,
                        "{at}: the lower band fell uncrossed"
                    );
                }
                let sides = [
                    (
                        now.upper.cmp(&before.upper),
                        close > before.upper,
                        now.upper == mid + band,
                    ),
                    (
                        before.lower.cmp(&now.lower),
                        close < before.lower,
                        now.lower == mid - band,
                    ),
                ];
                for (side, (moved, crossed, basic)) in sides.into_iter().enumerate() {
                    match moved {
                        std::cmp::Ordering::Greater => {
                            assert!(crossed, "{at}: band {side} reset with no cross");
                            assert!(basic, "{at}: band {side} reset off its basic band");
                            reset[side] += 1;
                        }
                        std::cmp::Ordering::Less => ratcheted[side] += 1,
                        std::cmp::Ordering::Equal => held[side] += 1,
                    }
                }
            }
            prior = Some((now, i128::from(candle.close)));
        }
    }
    assert!(
        held.iter()
            .chain(&ratcheted)
            .chain(&reset)
            .all(|&count| count > 0),
        "[upper, lower] held {held:?}, ratcheted {ratcheted:?}, reset {reset:?}: every \
         case must occur on both bands"
    );
}

#[test]
fn a_turn_needs_a_close_beyond_the_current_band() {
    let mut turned = [0_u32; 2];
    for seed in 1..=200_u64 {
        let mut rng = Rng::new(seed ^ 0x7E57);
        let period = rng.period(20);
        let factor = rng.factor(500, 5_000);
        let candles = random_series(&mut rng, usize::from(period) + 60);
        let mut two = DualBandSuperTrend::new(period, factor).unwrap();
        let mut prior: Option<Trend> = None;
        for (t, candle) in candles.iter().enumerate() {
            two.fold(candle).unwrap();
            let Some(now) = two.reading() else { continue };
            let close = i128::from(candle.close);
            let at = format!("seed {seed}, candle {t}");
            match (prior, now.trend) {
                (Some(Down), Up) => {
                    assert!(
                        close > now.upper,
                        "{at}: up without a close above the upper band"
                    );
                    turned[0] += 1;
                }
                (Some(Down), Down) => assert!(close <= now.upper, "{at}: stayed down above it"),
                (Some(Up), Down) => {
                    assert!(
                        close < now.lower,
                        "{at}: down without a close below the lower band"
                    );
                    turned[1] += 1;
                }
                (Some(Up), Up) => assert!(close >= now.lower, "{at}: stayed up below it"),
                (None, _) => {}
            }
            prior = Some(now.trend);
        }
    }
    assert!(
        turned.iter().all(|&count| count > 0),
        "both turns occur: {turned:?}"
    );
    // A close EQUAL to the current band does not turn; one paisa beyond does.
    // ST-1's first five candles end down on an upper band of 120, which the next
    // candle holds; ST-2's first ten end up on a lower band of 110, likewise.
    let st1 = [
        (110, 90, 100),
        (112, 100, 110),
        (115, 105, 114),
        (116, 108, 115),
        (116, 109, 111),
    ];
    let st2 = [
        (126, 104, 106),
        (118, 104, 115),
        (120, 109, 110),
        (115, 108, 113),
        (118, 101, 101),
        (123, 100, 119),
        (135, 118, 130),
        (139, 126, 136),
        (136, 119, 123),
        (124, 116, 121),
    ];
    let cases: [(&[Hlc], u32, Hlc, Two); 4] = [
        (&st1, 1_000, (122, 118, 120), (Down, 120, 120, 111)),
        (&st1, 1_000, (122, 118, 121), (Up, 111, 120, 111)),
        (&st2, 1_500, (112, 108, 110), (Up, 110, 128, 110)),
        (&st2, 1_500, (112, 108, 109), (Down, 128, 128, 110)),
    ];
    for (prefix, factor, next, (trend, line, upper, lower)) in cases {
        let mut candles: Vec<Candle> = prefix
            .iter()
            .enumerate()
            .map(|(i, &hlc)| bar(i, hlc))
            .collect();
        candles.push(bar(prefix.len(), next));
        let all = readings(&candles, 2, factor);
        let want = Reading {
            trend,
            line,
            upper,
            lower,
            settled: true,
        };
        assert_eq!(
            all.last().copied().flatten(),
            Some(want),
            "next candle {next:?}"
        );
    }
}

#[test]
fn fixture_st_1_the_seed_differs() {
    // The latch seeds on the first candle's side (up); the two-band form reads
    // nothing until its ATR is warm, then starts down.
    let rows = [
        row((110, 90, 100), 20, (Up, 80), None),
        row((112, 100, 110), 16, (Up, 90), Some((Down, 122, 122, 90))),
        row((115, 105, 114), 13, (Up, 97), Some((Down, 122, 122, 97))),
        row((116, 108, 115), 10, (Up, 102), Some((Down, 122, 122, 102))),
        row((116, 109, 111), 8, (Up, 104), Some((Down, 120, 120, 104))),
        row((112, 96, 98), 12, (Down, 116), Some((Down, 116, 116, 104))),
        row((100, 90, 95), 11, (Down, 106), Some((Down, 106, 106, 84))),
    ];
    check_fixture("ST-1", 2, 1_000, &rows);
}

#[test]
fn fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band() {
    // At candle 10 both turn down: the latch re-seeds at the fresh basic band
    // 140, the two-band form keeps its ratcheted upper band 138.
    let rows = [
        row((126, 104, 106), 22, (Down, 148), None),
        row((118, 104, 115), 18, (Down, 138), Some((Down, 138, 138, 84))),
        row((120, 109, 110), 14, (Down, 135), Some((Down, 135, 135, 93))),
        row((115, 108, 113), 10, (Down, 126), Some((Down, 126, 126, 96))),
        row((118, 101, 101), 13, (Down, 126), Some((Down, 126, 126, 96))),
        row((123, 100, 119), 18, (Down, 126), Some((Down, 126, 126, 96))),
        row((135, 118, 130), 17, (Up, 101), Some((Up, 101, 126, 101))),
        row((139, 126, 136), 15, (Up, 110), Some((Up, 110, 154, 110))),
        row((136, 119, 123), 16, (Up, 110), Some((Up, 110, 151, 110))),
        row((124, 116, 121), 12, (Up, 110), Some((Up, 110, 138, 110))),
        row(
            (126, 104, 108),
            17,
            (Down, 140),
            Some((Down, 138, 138, 110)),
        ),
    ];
    check_fixture("ST-2", 2, 1_500, &rows);
}

#[test]
fn fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first() {
    // At 3 the latch's stop is the fresh basic band 118 and the two-band line the
    // ratcheted 124; at 6 the close 119 is below 124 but not below 118, so the
    // two-band form turns down one candle before the latch.
    let rows = [
        row((143, 138, 142), 5, (Up, 135), None),
        row(
            (142, 126, 126),
            10,
            (Down, 144),
            Some((Down, 144, 144, 124)),
        ),
        row((128, 119, 124), 9, (Down, 132), Some((Down, 132, 132, 124))),
        row((144, 123, 140), 15, (Up, 118), Some((Up, 124, 132, 124))),
        row((142, 119, 124), 19, (Up, 118), Some((Up, 124, 149, 124))),
        row((137, 123, 137), 16, (Up, 118), Some((Up, 124, 146, 124))),
        row((141, 119, 119), 19, (Up, 118), Some((Down, 146, 146, 124))),
        row(
            (123, 114, 116),
            14,
            (Down, 132),
            Some((Down, 132, 132, 104)),
        ),
    ];
    check_fixture("ST-3", 2, 1_000, &rows);
}

#[test]
fn fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here() {
    // Five candles of 100: a stock that does not move reads up in the latch
    // (close >= mid at its seed) and down here (the first ATR candle's down,
    // never crossed by a close equal to its band).
    let still = (100, 100, 100);
    let rows = [
        row(still, 0, (Up, 100), None),
        row(still, 0, (Up, 100), None),
        row(still, 0, (Up, 100), Some((Down, 100, 100, 100))),
        row(still, 0, (Up, 100), Some((Down, 100, 100, 100))),
        row(still, 0, (Up, 100), Some((Down, 100, 100, 100))),
    ];
    check_fixture("ST-4", 3, 3_000, &rows);
}

#[test]
fn fixture_st_5_a_wide_candle_turns_the_two_band_form_first() {
    // Both are up on line 106 at candle 4. Candle 5's basic lower band (113) is
    // above its close (111): the two-band form turns down at once, while the
    // latch compares the close with its PREVIOUS stop (106), stays up with a stop
    // above the close, and turns at 6.
    let rows = [
        row((121, 101, 118), 20, (Up, 101), None),
        row((118, 102, 105), 18, (Up, 101), Some((Down, 119, 119, 101))),
        row((106, 101, 103), 11, (Up, 101), Some((Down, 108, 108, 101))),
        row((107, 101, 106), 8, (Up, 101), Some((Down, 108, 108, 101))),
        row((120, 104, 118), 12, (Up, 106), Some((Up, 106, 108, 106))),
        row((138, 109, 111), 20, (Up, 113), Some((Down, 133, 133, 113))),
        row(
            (113, 104, 104),
            14,
            (Down, 115),
            Some((Down, 115, 115, 101)),
        ),
    ];
    check_fixture("ST-5", 2, 500, &rows);
}

#[test]
fn fixture_st_9_equal_bands_do_not_turn_an_up_candle() {
    let rows = [
        row((119, 108, 108), 11, (Down, 118), None),
        row((119, 105, 119), 12, (Up, 106), Some((Down, 118, 118, 106))),
        row((129, 116, 128), 12, (Up, 116), Some((Down, 128, 128, 116))),
        row((139, 128, 136), 11, (Up, 128), Some((Up, 128, 128, 128))),
        row((156, 130, 151), 18, (Up, 134), Some((Up, 134, 152, 134))),
        row((151, 138, 138), 15, (Up, 137), Some((Up, 137, 151, 137))),
        row((142, 137, 141), 10, (Up, 137), Some((Up, 137, 144, 137))),
        row((171, 124, 155), 28, (Up, 137), Some((Up, 137, 144, 137))),
    ];
    check_fixture("ST-9", 2, 500, &rows);
    // At candle 3 the two bands are both 128, so the previous line equals the
    // previous upper band. At candle 4 the close 151 is below the upper band 152:
    // the script's literal comparison would read down, the previous trend reads
    // up (D-8602).
    let candles: Vec<Candle> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| bar(i, r.hlc))
        .collect();
    let all = readings(&candles, 2, 500);
    let (three, four) = (all[3].unwrap(), all[4].unwrap());
    assert_eq!((three.upper, three.lower, three.line), (128, 128, 128));
    assert!(i128::from(candles[4].close) < four.upper);
    assert_eq!(four.trend, Up);
}

#[test]
fn a_gap_through_either_band_turns_on_the_gap_candle() {
    // ST-7: the gap candle's close 120 crosses the upper band 102 and the bands
    // cross, which the rules allow. ST-8 adds a gap down through the lower band.
    let st7 = [
        row((101, 99, 100), 2, (Up, 98), None),
        row((101, 99, 100), 2, (Up, 98), Some((Down, 102, 102, 98))),
        row((102, 100, 101), 2, (Up, 99), Some((Down, 102, 102, 99))),
        row((121, 118, 120), 11, (Up, 108), Some((Up, 108, 102, 108))),
        row((122, 119, 121), 7, (Up, 113), Some((Up, 113, 127, 113))),
    ];
    check_fixture("ST-7", 2, 1_000, &st7);
    let st8 = [
        row((101, 99, 100), 2, (Up, 98), None),
        row((101, 99, 100), 2, (Up, 98), Some((Down, 102, 102, 98))),
        row((102, 100, 101), 2, (Up, 99), Some((Down, 102, 102, 99))),
        row((121, 118, 120), 11, (Up, 108), Some((Up, 108, 102, 108))),
        row((122, 119, 121), 7, (Up, 113), Some((Up, 113, 127, 113))),
        row((96, 93, 94), 17, (Down, 111), Some((Down, 111, 111, 113))),
    ];
    check_fixture("ST-8", 2, 1_000, &st8);
}

#[test]
fn an_all_up_and_an_all_down_series() {
    // Rising by 10 paisa a candle with a constant range of 20: the first close
    // above the upper band (1,080 against 1,070, candle 6) turns it up, and it
    // stays up with the line rising on every candle after.
    let rising: Vec<Candle> = (0..40_usize)
        .map(|step| {
            let rise = 10 * i64::try_from(step).unwrap();
            bar(step, (1_020 + rise, 1_000 + rise, 1_020 + rise))
        })
        .collect();
    let all = readings(&rising, 3, 2_000);
    let turn = all
        .iter()
        .zip(&rising)
        .position(|(reading, candle)| {
            reading.is_some_and(|now| i128::from(candle.close) > now.upper)
        })
        .expect("a rising series closes above its upper band");
    assert_eq!(turn, 6, "1,080 against an upper band of 1,070");
    let trend_of = |reading: &Option<Reading>| reading.unwrap().trend;
    let line_of = |reading: &Option<Reading>| reading.unwrap().line;
    assert!(all[2..turn].iter().all(|reading| trend_of(reading) == Down));
    assert!(all[turn..].iter().all(|reading| trend_of(reading) == Up));
    assert!(
        all[turn..]
            .windows(2)
            .all(|pair| line_of(&pair[0]) < line_of(&pair[1]))
    );
    // Falling: down from the first reading to the last, the line falling.
    let falling: Vec<Candle> = (0..40_usize)
        .map(|step| {
            let fall = 10 * i64::try_from(step).unwrap();
            bar(step, (5_020 - fall, 5_000 - fall, 5_000 - fall))
        })
        .collect();
    let all = readings(&falling, 3, 2_000);
    assert!(all[2..].iter().all(|reading| trend_of(reading) == Down));
    assert!(
        all[2..]
            .windows(2)
            .all(|pair| line_of(&pair[0]) > line_of(&pair[1]))
    );
}

#[test]
fn the_stream_equals_a_from_scratch_transcription_of_the_published_rules() {
    let (mut compared, mut turns, mut equal_bands) = (0_u32, 0_u32, 0_u32);
    for seed in 1..=300_u64 {
        let mut rng = Rng::new(seed);
        let period = rng.period(20);
        let factor = rng.factor(500, 5_000);
        let length = usize::from(period) + 30 + usize::try_from(rng.below(50)).unwrap();
        let candles = random_series(&mut rng, length);
        let mut two = DualBandSuperTrend::new(period, factor).unwrap();
        let mut prior: Option<Trend> = None;
        for (t, candle) in candles.iter().enumerate() {
            two.fold(candle).unwrap();
            let want = transcribe(&candles[..=t], period, factor);
            assert_eq!(
                (two.atr(), two.reading()),
                want,
                "seed {seed}, period {period}, factor {factor}, candle {t}"
            );
            if let Some(now) = two.reading() {
                compared += 1;
                turns += u32::from(prior.is_some_and(|trend| trend != now.trend));
                equal_bands += u32::from(now.upper == now.lower);
                prior = Some(now.trend);
            }
        }
    }
    let exercised = format!(
        "the series must exercise the rules: {compared} readings, {turns} turns, \
         {equal_bands} with equal bands"
    );
    assert!(compared > 10_000, "{exercised}");
    assert!(turns > 500, "{exercised}");
    assert!(equal_bands > 0, "{exercised}");
}

#[test]
fn the_atr_is_the_engine_atr() {
    let (mut cold, mut warm) = (0_u32, 0_u32);
    for seed in 1..=100_u64 {
        let mut rng = Rng::new(seed ^ 0xA7);
        let period = if seed % 10 == 0 { 300 } else { rng.period(30) };
        let candles = random_series(&mut rng, usize::from(period) + 20);
        let mut two = DualBandSuperTrend::new(period, rng.factor(1, MAX_FACTOR_MILLI)).unwrap();
        let mut engine = Atr::new(i128::from(period));
        for (t, candle) in candles.iter().enumerate() {
            two.fold(candle).unwrap();
            engine.fold(candle);
            if engine.warm() {
                assert_eq!(two.atr(), engine.value(), "seed {seed}, candle {t}");
                assert!(two.atr().is_some());
                warm += 1;
            } else {
                assert_eq!(two.atr(), None, "seed {seed}, candle {t}: not warm");
                cold += 1;
            }
        }
    }
    assert!(cold > 0, "some candles were folded before the ATR was warm");
    assert!(warm > 0, "some candles were folded after it");
}

#[test]
fn no_reading_depends_on_a_later_candle() {
    for seed in 1..=60_u64 {
        let mut rng = Rng::new(seed ^ 0x5EED);
        let period = rng.period(20);
        let factor = rng.factor(500, 5_000);
        let candles = random_series(&mut rng, 80);
        let whole = readings(&candles, period, factor);
        for cut in [usize::from(period) - 1, 30, 55, 78] {
            // Redraw every candle after `cut` from another stream.
            let mut redrawn = candles[..=cut].to_vec();
            let mut other = Rng::new(seed.wrapping_mul(31) ^ u64::try_from(cut).unwrap());
            redrawn.extend(random_series(&mut other, 79 - cut));
            assert_ne!(redrawn[cut + 1..], candles[cut + 1..]);
            let again = readings(&redrawn, period, factor);
            assert_eq!(again[..=cut], whole[..=cut], "seed {seed}, cut {cut}");
        }
    }
}

#[test]
fn a_time_gap_reads_as_contiguous_candles() {
    for seed in 1..=60_u64 {
        let mut rng = Rng::new(seed ^ 0x71AE);
        let period = rng.period(20);
        let factor = rng.factor(500, 5_000);
        let contiguous = random_series(&mut rng, 70);
        // The same candles, a minute to two weeks apart.
        let mut stamp = 0_i64;
        let gapped: Vec<Candle> = contiguous
            .iter()
            .map(|candle| {
                stamp += MINUTE * rng.between(1, 20_160);
                Candle {
                    ts_micros: stamp,
                    ..*candle
                }
            })
            .collect();
        let mut minute = DualBandSuperTrend::new(period, factor).unwrap();
        let mut gaps = minute;
        for (t, (a, b)) in contiguous.iter().zip(&gapped).enumerate() {
            minute.fold(a).unwrap();
            gaps.fold(b).unwrap();
            assert_eq!(minute, gaps, "seed {seed}, candle {t}: the whole state");
        }
    }
}

#[test]
fn the_extremes_never_overflow() {
    // The widest true range an evaluable candle has: high i64::MAX, low 1, the
    // close alternating between them, at the widest factor.
    let wide = [(i64::MAX, 1, 1), (i64::MAX, 1, i64::MAX)];
    for period in [1, u16::MAX] {
        let length = usize::from(period) + 6;
        let candles: Vec<Candle> = (0..length).map(|i| bar(i, wide[i % 2])).collect();
        let mut two = DualBandSuperTrend::new(period, MAX_FACTOR_MILLI).unwrap();
        for (t, candle) in candles.iter().enumerate() {
            two.fold(candle).unwrap();
            if t + 3 >= length {
                assert_eq!(
                    (two.atr(), two.reading()),
                    transcribe(&candles[..=t], period, MAX_FACTOR_MILLI),
                    "period {period}, candle {t}"
                );
            }
        }
        // The const bounds, restated at run time on the widest values.
        let atr = two.atr().unwrap();
        assert_eq!(
            atr,
            i64::MAX - 1,
            "period {period}: every true range is i64::MAX - 1"
        );
        let reading = two.reading().unwrap();
        let ceiling = 1_i128 << 81;
        for value in [reading.line, reading.upper, reading.lower] {
            assert!(
                value.abs() < ceiling,
                "period {period}: {value} within 2^81"
            );
        }
        assert!(i128::from(atr) * i128::from(MAX_FACTOR_MILLI) < 1_i128 << 80);
    }
}

#[test]
fn a_corrupt_candle_is_refused_and_changes_nothing() {
    let sane = bar(0, (110, 90, 100));
    let bad = [
        (
            Corrupt::HighBelowLow,
            Candle {
                high: 90,
                low: 110,
                ..sane
            },
        ),
        (
            Corrupt::RangeOverflows,
            Candle {
                high: i64::MAX,
                low: -1,
                open: 0,
                close: 0,
                ..sane
            },
        ),
        (Corrupt::PriceOutsideRange, Candle { close: 111, ..sane }),
        (Corrupt::NegativeVolume, Candle { volume: -1, ..sane }),
        (Corrupt::PriceNotPositive, Candle::default()),
    ];
    let candles = walk(&mut Rng::new(3), 6, 10_000, 50);
    let mut states = vec![DualBandSuperTrend::new(3, 3_000).unwrap()];
    for candle in &candles {
        let mut next = *states.last().unwrap();
        next.fold(candle).unwrap();
        states.push(next);
    }
    assert!(states[1].reading().is_none(), "a warming state");
    assert!(states[6].reading().unwrap().settled, "a settled state");
    for state in states {
        for (why, candle) in bad {
            let mut probe = state;
            assert_eq!(probe.fold(&candle), Err(Refusal::Corrupt(why)), "{why:?}");
            assert_eq!(probe, state, "{why:?}: the refusal changed the state");
        }
    }
}

#[test]
fn the_state_is_a_fixed_size() {
    // `Copy`, so it owns nothing on the heap: the size is the whole state.
    fn copied<T: Copy>(value: T) -> (T, T) {
        (value, value)
    }
    let (one, two) = copied(DualBandSuperTrend::new(10, 3_000).unwrap());
    assert_eq!(one, two);
    let measured = core::mem::size_of::<DualBandSuperTrend>();
    assert!(
        measured <= 256,
        "PARTS 5.5 bounds the state at 256 bytes: {measured}"
    );
    assert_eq!(
        measured, 224,
        "the measured size, stated in the part's report"
    );
}

#[test]
fn the_bench_measures_both_rows_at_tiny_sizes() {
    let step = measure_step(12, 4, 2).expect("the step row's period and factor are in range");
    assert_eq!(step.len(), 2, "two rounds");
    assert!(
        step.iter().all(|round| round.len() == 4),
        "four timings a round"
    );
    let period = measure_period(7, 3, 2).expect("period 7 is in range");
    assert_eq!(period.len(), 2, "two rounds");
    assert!(
        period.iter().all(|round| round.len() == 3),
        "three timings a round"
    );
    assert_eq!(
        measure_step(5, 1, 0),
        Ok(Vec::new()),
        "no rounds, no timings"
    );
    assert_eq!(measure_period(0, 1, 1), Err(Refusal::PeriodZero));
    // The synthetic stream the rows fold is evaluable and turns both ways, so a
    // measured step meets both trends rather than one branch path.
    let mut two = DualBandSuperTrend::new(10, 3_000).unwrap();
    let mut turns = [0_u32; 2];
    let mut prior: Option<Trend> = None;
    for step in 0..2_000 {
        two.fold(&synthetic(step)).unwrap();
        if let Some(now) = two.reading() {
            if prior.is_some_and(|trend| trend != now.trend) {
                turns[usize::from(now.trend == Up)] += 1;
            }
            prior = Some(now.trend);
        }
    }
    // Bound before the assertions: a format argument is evaluated only on
    // failure, so one in the message list is a region a green run never enters.
    let [down, up] = turns;
    let counted =
        format!("the synthetic stream turns down {down} and up {up} times in 2,000 candles");
    assert!(down > 0, "{counted}");
    assert!(up > 0, "{counted}");
}

#[test]
fn a_cost_is_read_by_rank_and_judged_flat_or_not() {
    let ramp: Vec<u64> = (1..=100).collect();
    assert_eq!(
        summarize(&ramp),
        StepCost {
            p50_ns: 50,
            p99_ns: 99,
            max_ns: 100,
            samples: 100
        }
    );
    let short: Vec<u64> = (1..=10).collect();
    assert_eq!(
        summarize(&short),
        StepCost {
            p50_ns: 5,
            p99_ns: 10,
            max_ns: 10,
            samples: 10
        },
        "the nearest rank rounds up"
    );
    assert_eq!(
        summarize(&[]),
        StepCost::default(),
        "an empty round reads zeros"
    );
    let cost = |p99_ns| StepCost {
        p50_ns: p99_ns,
        p99_ns,
        max_ns: p99_ns,
        samples: 1,
    };
    assert!(!flat_within(&[], 3_000, 2_000), "no costs judge false");
    assert!(
        !flat_within(&[cost(0), cost(0)], 3_000, 2_000),
        "a zero first p99 fails"
    );
    assert!(
        flat_within(&[cost(100), cost(300)], 3_000, 2_000),
        "exactly 3.0x holds"
    );
    assert!(
        !flat_within(&[cost(100), cost(301)], 3_000, 2_000),
        "past the ceiling"
    );
    assert!(
        flat_within(&[cost(1_000), cost(2_000)], 3_000, 2_000),
        "at the budget"
    );
    assert!(
        !flat_within(&[cost(1_000), cost(2_001)], 3_000, 2_000),
        "over the budget"
    );
    assert!(
        !flat_within(&[cost(2_001)], 3_000, 2_000),
        "the first size over the budget"
    );
}
