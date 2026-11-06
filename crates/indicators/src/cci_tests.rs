//! The tests of [`super`], PSRB-CCI (D-8650 to D-8655): every test PARTS-WILLR-CCI.md 6.5 names, fixtures
//! CCI-1, CCI-2, CCI-2m and CCI-3 exactly, the ceiling at `u16::MAX`, a brute-force window and a
//! transcription of the published definition in exact rationals, both written apart from the module. Every
//! series is invented; no vendor row.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a test \
              that cannot panic cannot fail. `expect` panics inside the standard \
              library and so leaves no region in this file that a green run cannot \
              enter."
)]

use std::cmp::Ordering;

use super::*;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// A candle's high, low and close, in paisa.
type Hlc = (i64, i64, i64);

/// CCI-1 (period 4), the table of PARTS-WILLR-CCI.md 6.5.
const CCI_1: [Hlc; 7] = [
    (110, 100, 105),
    (112, 104, 110),
    (108, 98, 100),
    (120, 106, 118),
    (125, 115, 124),
    (126, 120, 121),
    (118, 100, 101),
];

/// CCI-1's `T` at every candle.
const CCI_1_T: [i128; 7] = [315, 326, 306, 344, 364, 367, 319];

/// CCI-1's `S`, `dev`, `D` and `milli` at candles 3 to 6, the four with a full window.
const CCI_1_WARM: [(i128, i128, i128, i64); 4] = [
    (1_291, 85, 196, 115_646),
    (1_340, 116, 304, 101_754),
    (1_381, 87, 324, 71_604),
    (1_394, -118, 272, -115_687),
];

/// The candle the CCI-2 and CCI-3 fixtures start from: (101, 99, 100), `T` 300.
const BASE: Hlc = (101, 99, 100);

/// A candle of `bar`, opened at its close and stamped `minute` minutes in.
fn candle(minute: i64, (high, low, close): Hlc) -> Candle {
    Candle::new(minute * MINUTE, close, high, low, close, 1, OI_NULL)
}

/// A level inside the domain.
fn level(milli: i32) -> Level {
    Level::from_milli(milli).expect("a level inside the domain")
}

/// A fresh window of `period` candles.
fn cci(period: u16) -> Cci {
    Cci::new(period).expect("a period above zero")
}

/// The window of `period` candles after `bars`, folded in order a minute apart.
fn folded(period: u16, bars: &[Hlc]) -> Cci {
    let mut window = cci(period);
    for (minute, &bar) in (0..).zip(bars) {
        window
            .fold(&candle(minute, bar))
            .expect("an evaluable candle");
    }
    window
}

/// The state after each candle of `bars`, in order.
fn states(period: u16, bars: &[Hlc]) -> Vec<Cci> {
    let mut window = cci(period);
    (0..)
        .zip(bars)
        .map(|(minute, &bar)| {
            window
                .fold(&candle(minute, bar))
                .expect("an evaluable candle");
            window.clone()
        })
        .collect()
}

/// The reading after each candle of `bars`, in order.
fn stream(period: u16, bars: &[Hlc]) -> Vec<Option<Reading>> {
    let mut window = cci(period);
    (0..)
        .zip(bars)
        .map(|(minute, &bar)| {
            window
                .fold(&candle(minute, bar))
                .expect("an evaluable candle");
            window.reading()
        })
        .collect()
}

/// The reading after the last candle of `bars`.
fn last_reading(period: u16, bars: &[Hlc]) -> Reading {
    folded(period, bars).reading().expect("a full window")
}

/// `count` copies of `bar`, then `rest`.
fn repeated(bar: Hlc, count: usize, rest: &[Hlc]) -> Vec<Hlc> {
    let mut bars = vec![bar; count];
    bars.extend_from_slice(rest);
    bars
}

/// CCI-2: five (101, 99, 100), then (105, 103, 104), then (113, 111, 113): exactly +200 at period 7.
fn cci_2() -> Vec<Hlc> {
    repeated(BASE, 5, &[(105, 103, 104), (113, 111, 113)])
}

/// CCI-2m, CCI-2's mirror: five (101, 99, 100), then (97, 95, 96), then (89, 87, 87): exactly -200.
fn cci_2m() -> Vec<Hlc> {
    repeated(BASE, 5, &[(97, 95, 96), (89, 87, 87)])
}

/// CCI-3: six (101, 99, 100), then (101, 99, 101): the ceiling `100 n / 3` at period 7.
fn cci_3() -> Vec<Hlc> {
    repeated(BASE, 6, &[(101, 99, 101)])
}

/// `T = H + L + C`, widened, computed here and not by the module.
fn typical((high, low, close): Hlc) -> i128 {
    i128::from(high) + i128::from(low) + i128::from(close)
}

/// BRUTE FORCE, the oracle of the ring: `(S, dev, D)` of the last `period` candles of `bars`, recomputed by
/// loops of its own from the candles, or `None` while fewer than `period` candles are given.
fn brute(period: u16, bars: &[Hlc]) -> Option<(i128, i128, i128)> {
    let start = bars.len().checked_sub(usize::from(period))?;
    let window = bars.get(start..).expect("a start inside the series");
    let n = i128::from(period);
    let mut sum = 0;
    for &bar in window {
        sum += typical(bar);
    }
    let mut total = 0;
    for &bar in window {
        total += (n * typical(bar) - sum).abs();
    }
    let current = typical(*window.last().expect("a window of at least one candle"));
    Some((sum, n * current - sum, total))
}

/// The module's view of the same window: `(S, dev, D)` once the window is full.
fn seen(window: &Cci) -> Option<(i128, i128, i128)> {
    window
        .reading()
        .map(|reading| (window.sum, reading.deviation(), reading.total_deviation()))
}

/// An exact rational `num / den` with `den > 0`, kept in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    /// `num / den`, for `den > 0`, reduced.
    fn new(num: i128, den: i128) -> Self {
        let divisor = gcd(num.abs(), den);
        Self {
            num: num / divisor,
            den: den / divisor,
        }
    }

    /// The whole number `value`.
    const fn whole(value: i128) -> Self {
        Self { num: value, den: 1 }
    }

    /// `self + other`.
    fn plus(self, other: Self) -> Self {
        Self::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }

    /// `self - other`.
    fn minus(self, other: Self) -> Self {
        Self::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }

    /// `self * other`.
    fn times(self, other: Self) -> Self {
        Self::new(self.num * other.num, self.den * other.den)
    }

    /// `self / other`, for `other != 0`.
    fn over(self, other: Self) -> Self {
        Self::new(
            self.num * other.den * other.num.signum(),
            self.den * other.num.abs(),
        )
    }

    /// `|self|`.
    const fn magnitude(self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den,
        }
    }

    /// The order of `self` against `other`, cross-multiplied.
    fn compare(self, other: Self) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }

    /// `floor(self)`.
    const fn floor(self) -> i128 {
        self.num.div_euclid(self.den)
    }
}

/// The greatest common divisor of `a >= 0` and `b > 0`.
fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The published definition (D-8650), transcribed apart from the module in exact rationals: each typical
/// price `TP = (H + L + C) / 3`, `SMA` their mean over the window, and `MD` the mean of `|TP - SMA|`. Returns
/// `(TP - SMA, MD)` for the window's last candle.
fn published_parts(window: &[Hlc]) -> (Ratio, Ratio) {
    let n = Ratio::whole(i128::try_from(window.len()).expect("a window length that fits"));
    let prices: Vec<Ratio> = window
        .iter()
        .map(|&bar| Ratio::new(typical(bar), 3))
        .collect();
    let mean = prices
        .iter()
        .fold(Ratio::whole(0), |acc, &price| acc.plus(price))
        .over(n);
    let mean_deviation = prices
        .iter()
        .fold(Ratio::whole(0), |acc, &price| {
            acc.plus(price.minus(mean).magnitude())
        })
        .over(n);
    let current = *prices.last().expect("a window of at least one candle");
    (current.minus(mean), mean_deviation)
}

/// `CCI = (TP - SMA) / (0.015 MD)` of the window's last candle, from [`published_parts`], or `None` when
/// `MD` is 0, where the published ratio is `0 / 0`.
fn published(window: &[Hlc]) -> Option<Ratio> {
    let (gap, mean_deviation) = published_parts(window);
    (mean_deviation.num != 0).then(|| gap.over(Ratio::new(15, 1_000).times(mean_deviation)))
}

/// A deterministic xorshift, so every run draws the same values.
struct Draw(u64);

impl Draw {
    /// The next value of the generator.
    const fn draw(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value in `low..=high`.
    fn within(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low + 1).expect("a range that holds a value");
        low + i64::try_from(self.draw() % span).expect("a draw below the span")
    }

    /// A period in `1..=most`.
    fn period(&mut self, most: u16) -> u16 {
        u16::try_from(self.within(1, i64::from(most))).expect("a period that fits")
    }
}

/// A candle around `mid`: the high up to 40 paisa above it, the low up to 40 below, the close anywhere between.
fn spread(draw: &mut Draw, mid: i64) -> Hlc {
    let high = mid + draw.within(0, 40);
    let low = mid - draw.within(0, 40);
    (high, low, draw.within(low, high))
}

/// `length` candles of a walk around 100,000 paisa, kept inside `50_000..=150_000`: small moves, a jump of up
/// to 3,000 paisa about one candle in twenty, the previous candle repeated about one in five (so typical prices
/// repeat), and a candle of one price about one in ten.
fn draw_series(draw: &mut Draw, length: usize) -> Vec<Hlc> {
    let mut mid = 100_000;
    let mut previous = (mid, mid, mid);
    let mut series = Vec::with_capacity(length);
    for _ in 0..length {
        let bar = match draw.within(0, 19) {
            0 => {
                mid = (mid + draw.within(-3_000, 3_000)).clamp(50_000, 150_000);
                spread(draw, mid)
            }
            1..=4 => previous,
            5 | 6 => {
                mid = (mid + draw.within(-25, 25)).clamp(50_000, 150_000);
                (mid, mid, mid)
            }
            _ => {
                mid = (mid + draw.within(-25, 25)).clamp(50_000, 150_000);
                spread(draw, mid)
            }
        };
        previous = bar;
        series.push(bar);
    }
    series
}

/// Period 0 is refused by name; 1, 20 and `u16::MAX` are accepted, each empty, not warm, with no reading and
/// a ring reserved for at least its period.
#[test]
fn a_zero_period_is_refused_and_every_other_period_is_accepted() {
    assert_eq!(Cci::new(0), Err(Refusal::PeriodZero));
    for period in [1, 20, u16::MAX] {
        let window = cci(period);
        assert_eq!(window.period(), period);
        assert!(window.ring.is_empty());
        assert!(window.ring.capacity() >= usize::from(period));
        assert_eq!((window.sum, window.last), (0, 0));
        assert!(!window.warm());
        assert_eq!(window.reading(), None);
    }
}

/// `Level::from_milli` accepts 10,000 points, -10,000 points and 0, each read back by `milli()`, and refuses
/// one thousandth past either edge, `i32::MAX` and `i32::MIN`, naming the value.
#[test]
fn a_level_past_ten_thousand_points_is_refused_at_both_edges() {
    assert_eq!(MAX_LEVEL_MILLI, 10_000_000);
    for milli in [10_000_000, -10_000_000, 0, 200_000, -200_000] {
        assert_eq!(Level::from_milli(milli).map(Level::milli), Ok(milli));
    }
    for milli in [10_000_001, -10_000_001, i32::MAX, i32::MIN] {
        assert_eq!(
            Level::from_milli(milli),
            Err(Refusal::LevelOutOfRange { milli })
        );
    }
}

/// At periods 1, 2, 3, 20 and `u16::MAX`: no reading after `period - 1` candles, one after `period`, equal to
/// the brute force.
#[test]
fn the_first_reading_appears_after_exactly_period_candles() {
    let mut draw = Draw(0x2545_F491_4F6C_DD1D);
    for period in [1, 2, 3, 20, u16::MAX] {
        let bars = draw_series(&mut draw, usize::from(period));
        let (last, early) = bars.split_last().expect("a period above zero");
        let mut window = folded(period, early);
        assert!(!window.warm());
        assert_eq!(window.reading(), None);
        let minute = i64::try_from(early.len()).expect("a count that fits");
        window
            .fold(&candle(minute, *last))
            .expect("an evaluable candle");
        assert!(window.warm());
        assert_eq!(seen(&window), brute(period, &bars));
        assert_eq!(
            window.reading().map(|reading| reading.period()),
            Some(period)
        );
    }
}

/// CCI-1 at period 4: `T` at every candle, then `S`, `dev`, `D` and `milli` at candles 3 to 6, and the level
/// results: above +100 at candles 3 and 4 only, below -100 at candle 6 only.
#[test]
fn cci_matches_fixture_cci_1() {
    let mut window = cci(4);
    let mut rows = Vec::new();
    for ((minute, &bar), &t) in (0..).zip(&CCI_1).zip(&CCI_1_T) {
        window
            .fold(&candle(minute, bar))
            .expect("an evaluable candle");
        assert_eq!(window.last, t);
        rows.push((window.sum, window.reading()));
    }
    assert_eq!(
        rows.iter().take(3).map(|row| row.1).collect::<Vec<_>>(),
        [None; 3]
    );
    let warm: Vec<(i128, Reading)> = rows
        .iter()
        .skip(3)
        .map(|&(sum, reading)| (sum, reading.expect("a full window from the fourth candle")))
        .collect();
    let table: Vec<(i128, i128, i128, i64)> = warm
        .iter()
        .map(|&(sum, reading)| {
            let milli = reading.milli().expect("a window with a value");
            (sum, reading.deviation(), reading.total_deviation(), milli)
        })
        .collect();
    assert_eq!(table, CCI_1_WARM);
    let above: Vec<bool> = warm.iter().map(|w| w.1.above(level(100_000))).collect();
    let below: Vec<bool> = warm.iter().map(|w| w.1.below(level(-100_000))).collect();
    assert_eq!(above, [true, true, false, false]);
    assert_eq!(below, [false, false, false, true]);
}

/// CCI-2 reads exactly +200 and CCI-2m exactly -200: a tie is neither above nor below, and one thousandth to
/// either side of it decides.
#[test]
fn a_tie_at_plus_two_hundred_is_neither_above_nor_below() {
    let window = folded(7, &cci_2());
    assert_eq!(window.sum, 2_149);
    let deviations: Vec<i128> = window.ring.iter().map(|&t| 7 * t - window.sum).collect();
    assert_eq!(deviations, [-49, -49, -49, -49, -49, 35, 210]);
    let tie = window.reading().expect("a full window");
    assert_eq!(
        (tie.deviation(), tie.total_deviation(), tie.milli()),
        (210, 490, Some(200_000))
    );
    assert!(!tie.above(level(200_000)));
    assert!(!tie.below(level(200_000)));
    assert!(tie.above(level(199_999)));
    assert!(tie.below(level(200_001)));
    let window = folded(7, &cci_2m());
    assert_eq!(window.sum, 2_051);
    let mirror = window.reading().expect("a full window");
    assert_eq!(
        (mirror.deviation(), mirror.total_deviation(), mirror.milli()),
        (-210, 490, Some(-200_000))
    );
    assert!(!mirror.below(level(-200_000)));
    assert!(!mirror.above(level(-200_000)));
    assert!(mirror.below(level(-199_999)));
    assert!(mirror.above(level(-200_001)));
}

/// Period 7: a reading whose floored `milli` IS the level, 200,000, while the exact CCI is
/// 2,940,008,400 / 14,700,030 = 200.000163...: `above` is true, so a decision on the floored value, which would
/// read false here, is not what the module does. With the tie above, where a floored `>=` would read true, it
/// tells the exact comparison from both floored forms.
#[test]
fn a_floored_milli_equal_to_the_level_is_not_a_tie() {
    let one = (1_000_000, 1_000_000, 1_000_000);
    let bars = repeated(
        one,
        5,
        &[
            (1_040_000, 1_040_000, 1_040_000),
            (1_123_334, 1_123_333, 1_123_334),
        ],
    );
    let window = folded(7, &bars);
    assert_eq!(window.last, 3_370_001);
    assert_eq!(window.sum, 21_490_001);
    let reading = window.reading().expect("a full window");
    assert_eq!(
        (
            reading.deviation(),
            reading.total_deviation(),
            reading.milli()
        ),
        (2_100_006, 4_900_010, Some(200_000))
    );
    assert_eq!(200_000 * 7 * reading.deviation(), 2_940_008_400_000);
    assert_eq!(3 * 200_000 * reading.total_deviation(), 2_940_006_000_000);
    assert!(reading.above(level(200_000)));
    assert!(!reading.below(level(200_000)));
    assert_eq!(reading.milli().map(i128::from), Some(200_000));
}

/// On random warm readings, `CCI > +200` is `n dev > 3 D` and is `TP - SMA > 3 MD` in exact rationals; the
/// mirror at -200 likewise. Both levels are passed by some readings and not by others.
#[test]
fn the_confirmed_level_is_three_mean_deviations() {
    let mut draw = Draw(0x9E37_79B9_7F4A_7C15);
    let (mut above, mut below, mut readings) = (0_usize, 0_usize, 0_usize);
    for _ in 0..300 {
        let period = draw.period(40);
        let bars = draw_series(&mut draw, usize::from(period) + 40);
        for (end, reading) in (1..).zip(stream(period, &bars)) {
            let Some(reading) = reading else {
                continue;
            };
            let start = end - usize::from(period);
            let window = bars.get(start..end).expect("the window's candles");
            let (gap, mean_deviation) = published_parts(window);
            let n = i128::from(period);
            let up = reading.above(level(200_000));
            let down = reading.below(level(-200_000));
            assert_eq!(up, n * reading.deviation() > 3 * reading.total_deviation());
            assert_eq!(
                down,
                n * reading.deviation() < -3 * reading.total_deviation()
            );
            let three = mean_deviation.times(Ratio::whole(3));
            assert_eq!(up, gap.compare(three) == Ordering::Greater);
            assert_eq!(
                down,
                gap.compare(three.times(Ratio::whole(-1))) == Ordering::Less
            );
            above += usize::from(up);
            below += usize::from(down);
            readings += 1;
        }
    }
    assert!(above > 0);
    assert!(below > 0);
    assert!(readings > 10_000);
}

/// CCI-3 reaches the ceiling, 233,333 at period 7. On random warm readings `2 |dev| <= D`; with `dev` not 0,
/// equality holds exactly when the current candle is the only one strictly on its side of the mean, and with
/// `dev` 0 exactly when `D` is 0. At `u16::MAX` the ceiling's `milli` passes `i32::MAX`.
#[test]
fn the_reading_never_passes_one_hundred_n_over_three() {
    let window = folded(7, &cci_3());
    assert_eq!(window.sum, 2_101);
    let ceiling = window.reading().expect("a full window");
    assert_eq!(
        (
            ceiling.deviation(),
            ceiling.total_deviation(),
            ceiling.milli()
        ),
        (6, 12, Some(233_333))
    );
    assert!(ceiling.above(level(200_000)));
    let mut draw = Draw(0xD1B5_4A32_D192_ED03);
    let (mut met, mut zero) = (0_usize, 0_usize);
    for _ in 0..1_000 {
        let period = draw.period(12);
        for window in states(period, &draw_series(&mut draw, usize::from(period) + 20)) {
            let Some(reading) = window.reading() else {
                continue;
            };
            let n = i128::from(period);
            let twice = 2 * reading.deviation().abs();
            let total = reading.total_deviation();
            assert!(twice <= total);
            let side = reading.deviation().signum();
            if side == 0 {
                assert_eq!(twice == total, total == 0);
                zero += 1;
            } else {
                let alone = window
                    .ring
                    .iter()
                    .rev()
                    .skip(1)
                    .all(|&t| (n * t - window.sum).signum() != side);
                assert_eq!(twice == total, alone);
                met += usize::from(alone);
            }
        }
    }
    assert!(met > 0);
    assert!(zero > 0);
    let window = folded(u16::MAX, &repeated((1, 1, 1), 65_534, &[(2, 2, 2)]));
    assert_eq!(window.sum, 196_608);
    let top = window.reading().expect("a full window");
    assert_eq!((top.deviation(), top.total_deviation()), (196_602, 393_204));
    assert_eq!(top.total_deviation(), 2 * top.deviation());
    assert_eq!(top.milli(), Some(2_184_500_000));
    assert_eq!(100_000 * i64::from(u16::MAX) / 3, 2_184_500_000);
    assert!(top.milli() > Some(i64::from(i32::MAX)));
}

/// The period-6 ceiling reads exactly +200 and is not above it; the same shape reads `100 n / 3` at periods 2
/// to 6; and on random series at periods 1 to 6 no reading is above +200.
#[test]
fn a_period_of_six_or_less_never_reads_above_two_hundred() {
    let six = last_reading(6, &repeated(BASE, 5, &[(101, 99, 101)]));
    assert_eq!(
        (six.deviation(), six.total_deviation(), six.milli()),
        (5, 10, Some(200_000))
    );
    assert!(!six.above(level(200_000)));
    assert!(six.above(level(199_999)));
    for (period, milli) in [
        (2, 66_666),
        (3, 100_000),
        (4, 133_333),
        (5, 166_666),
        (6, 200_000),
    ] {
        let shape = repeated(BASE, usize::from(period) - 1, &[(101, 99, 101)]);
        let reading = last_reading(period, &shape);
        assert_eq!(reading.milli(), Some(milli));
        assert!(!reading.above(level(200_000)));
    }
    let mut draw = Draw(0x94D0_49BB_1331_11EB);
    let mut readings = Vec::new();
    for _ in 0..1_000 {
        let period = draw.period(6);
        readings.extend(
            stream(period, &draw_series(&mut draw, 30))
                .into_iter()
                .flatten(),
        );
    }
    assert!(readings.len() > 10_000);
    for reading in readings {
        assert!(!reading.above(level(200_000)));
    }
}

/// Period 3 over (101, 99, 100), (102, 98, 100) and (100, 100, 100): three different candles, one typical
/// price. `D` is 0 and so is `dev`, `flat()` is true, `milli()` is `None`, and `above` and `below` are false at
/// both edges of the domain, at +/-200 and at 0.
#[test]
fn a_flat_window_has_no_value_and_compares_false() {
    let window = folded(3, &[(101, 99, 100), (102, 98, 100), (100, 100, 100)]);
    assert_eq!(window.ring, [300, 300, 300]);
    let reading = window.reading().expect("a full window");
    assert_eq!((reading.deviation(), reading.total_deviation()), (0, 0));
    assert!(reading.flat());
    assert_eq!(reading.milli(), None);
    for milli in [-10_000_000, -200_000, 0, 200_000, 10_000_000] {
        assert!(!reading.above(level(milli)));
        assert!(!reading.below(level(milli)));
    }
}

/// At period 1 every reading of random candles is a window of one typical price ([`Reading::flat`]): `dev`
/// and `D` are 0, there is no `milli`, and nothing compares true.
#[test]
fn a_period_of_one_is_always_flat() {
    let mut draw = Draw(0xBF58_476D_1CE4_E5B9);
    let readings: Vec<Reading> = stream(1, &draw_series(&mut draw, 2_000))
        .into_iter()
        .map(|reading| reading.expect("a window of one candle is full at once"))
        .collect();
    assert_eq!(readings.len(), 2_000);
    for reading in readings {
        assert!(reading.flat());
        assert_eq!((reading.deviation(), reading.milli()), (0, None));
        assert!(!reading.above(level(-MAX_LEVEL_MILLI)));
        assert!(!reading.below(level(MAX_LEVEL_MILLI)));
    }
}

/// Period 2: (110, 90, 100) then (120, 100, 110) reads 66,666; (100, 90, 95) then reads -66,667; the same
/// candle again has no value. On random candles every reading with a value is exactly +200/3 or -200/3.
#[test]
fn a_period_of_two_reads_plus_or_minus_sixty_six_point_six() {
    let readings = stream(
        2,
        &[
            (110, 90, 100),
            (120, 100, 110),
            (100, 90, 95),
            (100, 90, 95),
        ],
    );
    assert_eq!(readings.first(), Some(&None));
    let milli: Vec<Option<i64>> = readings
        .iter()
        .skip(1)
        .map(|reading| reading.expect("a full window").milli())
        .collect();
    assert_eq!(milli, [Some(66_666), Some(-66_667), None]);
    let mut draw = Draw(0x2127_599B_F432_5C37);
    for reading in stream(2, &draw_series(&mut draw, 5_000))
        .into_iter()
        .flatten()
    {
        let up = reading.deviation() > 0;
        let down = reading.deviation() < 0;
        assert_eq!(reading.above(level(66_666)), up);
        assert!(!reading.above(level(66_667)));
        assert_eq!(reading.below(level(-66_666)), down);
        assert!(!reading.below(level(-66_667)));
        assert!([None, Some(66_666), Some(-66_667)].contains(&reading.milli()));
    }
}

/// On random readings with a value, `milli * 3D <= 200_000 n dev < (milli + 1) * 3D`.
#[test]
fn milli_is_the_floor_of_the_exact_ratio() {
    let mut draw = Draw(0x4CF5_AD43_2745_937F);
    let mut checked = Vec::new();
    for _ in 0..500 {
        let period = draw.period(40);
        let bars = draw_series(&mut draw, usize::from(period) + 30);
        checked.extend(
            stream(period, &bars)
                .into_iter()
                .flatten()
                .filter(|reading| !reading.flat()),
        );
    }
    assert!(checked.len() > 10_000);
    for reading in checked {
        let milli = i128::from(reading.milli().expect("a reading with a value"));
        let scaled = 200_000 * i128::from(reading.period()) * reading.deviation();
        let divisor = 3 * reading.total_deviation();
        assert!(milli * divisor <= scaled);
        assert!(scaled < (milli + 1) * divisor);
    }
}

/// BRUTE FORCE: at every candle of 1,000 random series at periods 1 to 40, `S` (also while filling), `dev`
/// and `D` equal a recomputation over the last `period` candles by loops of their own.
#[test]
fn the_ring_equals_a_brute_force_window() {
    let mut draw = Draw(0x6A09_E667_F3BC_C909);
    let mut compared = 0_usize;
    for _ in 0..1_000 {
        let period = draw.period(40);
        let bars = draw_series(&mut draw, usize::from(period) + 40);
        for (end, window) in (1..).zip(states(period, &bars)) {
            let prefix = bars.get(..end).expect("the candles folded so far");
            let filled: i128 = prefix
                .iter()
                .rev()
                .take(usize::from(period))
                .map(|&bar| typical(bar))
                .sum();
            assert_eq!(window.sum, filled);
            assert_eq!(seen(&window), brute(period, prefix));
            compared += 1;
        }
    }
    assert!(compared > 50_000);
}

/// PARITY: on CCI-1, CCI-2, CCI-2m, CCI-3 and 400 random series at periods 1 to 40, every reading equals the
/// published definition in exact rationals: no value exactly when `MD` is 0, `milli` the floor of `1000 CCI`,
/// and every comparison at +/-200, +/-100 and 0.
#[test]
fn parity_with_the_published_definition() {
    let mut cases: Vec<(u16, Vec<Hlc>)> = vec![
        (4, CCI_1.to_vec()),
        (7, cci_2()),
        (7, cci_2m()),
        (7, cci_3()),
    ];
    let mut draw = Draw(0xBB67_AE85_84CA_A73B);
    for _ in 0..400 {
        let period = draw.period(40);
        cases.push((period, draw_series(&mut draw, usize::from(period) + 30)));
    }
    let mut compared = 0_usize;
    for (period, bars) in &cases {
        for (end, reading) in (1..).zip(stream(*period, bars)) {
            let Some(reading) = reading else {
                continue;
            };
            let window = bars
                .get(end - usize::from(*period)..end)
                .expect("the window's candles");
            let exact = published(window);
            assert_eq!(reading.flat(), exact.is_none());
            assert_eq!(
                reading.milli().map(i128::from),
                exact.map(|value| value.times(Ratio::whole(1_000)).floor())
            );
            for milli in [-200_000, -100_000, 0, 100_000, 200_000] {
                let order = exact.map(|value| value.compare(Ratio::new(i128::from(milli), 1_000)));
                assert_eq!(
                    reading.above(level(milli)),
                    order == Some(Ordering::Greater)
                );
                assert_eq!(reading.below(level(milli)), order == Some(Ordering::Less));
            }
            compared += 1;
        }
    }
    assert!(compared > 10_000);
}

/// Period `u16::MAX` over (1, 1, 1) and (`i64::MAX`, `i64::MAX`, `i64::MAX`) alternating, and period 2 over the
/// same, at the levels +/-10,000 points: every product is formed under the overflow checks of this build, the
/// figures equal their closed forms, and the const bounds hold at run time.
#[test]
fn the_largest_prices_and_period_never_overflow() {
    let low = (1, 1, 1);
    let top = (i64::MAX, i64::MAX, i64::MAX);
    let bars: Vec<Hlc> = (0..u16::MAX)
        .map(|k| if k % 2 == 0 { low } else { top })
        .collect();
    let mut window = folded(u16::MAX, &bars);
    let big = 3 * i128::from(i64::MAX);
    assert_eq!(window.sum, 32_768 * 3 + 32_767 * big);
    let reading = window.reading().expect("a full window");
    assert_eq!(reading.deviation(), -32_767 * (big - 3));
    assert_eq!(reading.total_deviation(), 2 * 32_768 * 32_767 * (big - 3));
    assert_eq!(
        reading.milli(),
        Some((-6_553_500_000_i64).div_euclid(98_304))
    );
    assert!(window.sum < 1 << 81);
    assert!(reading.deviation().abs() < 1 << 81);
    assert!(reading.total_deviation() < 1 << 97);
    assert!(200_000 * i128::from(u16::MAX) * reading.deviation().abs() < 1 << 115);
    assert!(3 * i128::from(MAX_LEVEL_MILLI) * reading.total_deviation() < 1 << 123);
    assert!(reading.above(level(-MAX_LEVEL_MILLI)));
    assert!(reading.below(level(MAX_LEVEL_MILLI)));
    assert!(!reading.above(level(MAX_LEVEL_MILLI)));
    assert!(!reading.below(level(-MAX_LEVEL_MILLI)));
    window
        .fold(&candle(65_535, top))
        .expect("an evaluable candle");
    let reading = window.reading().expect("a full window");
    assert_eq!(reading.milli(), Some(6_553_500_000_i64.div_euclid(98_304)));
    assert!(reading.above(level(-MAX_LEVEL_MILLI)));
    assert!(!reading.above(level(MAX_LEVEL_MILLI)));
    let pair: Vec<Option<i64>> = stream(2, &[low, top, low, top])
        .into_iter()
        .map(|reading| reading.and_then(|r| r.milli()))
        .collect();
    assert_eq!(pair, [None, Some(66_666), Some(-66_667), Some(66_666)]);
    let last = last_reading(2, &[low, top]);
    assert!(last.below(level(MAX_LEVEL_MILLI)));
    assert!(!last.below(level(-MAX_LEVEL_MILLI)));
}

/// At periods 3 and 20, a 10% jump in one candle: `S`, `dev` and `D` equal the brute force at every step, and
/// the jump's `T` is in the window for exactly `period` candles, from its own fold on.
#[test]
fn a_price_gap_enters_the_window_and_leaves_it_after_period_candles() {
    for period in [3, 20] {
        let length = 3 * usize::from(period) + 10;
        let jump_at = usize::from(period) + 5;
        let bars: Vec<Hlc> = (0..length)
            .map(|k| {
                let close = 100_000 + 5 * i64::try_from(k % 3).expect("a remainder below 3");
                (close + 10, close - 10, close)
            })
            .collect();
        let bars: Vec<Hlc> = (0..)
            .zip(bars)
            .map(|(k, bar)| {
                if k == jump_at {
                    (110_010, 109_990, 110_000)
                } else {
                    bar
                }
            })
            .collect();
        let mut held = Vec::new();
        for (end, window) in (1..).zip(states(period, &bars)) {
            let prefix = bars.get(..end).expect("the candles folded so far");
            assert_eq!(seen(&window), brute(period, prefix));
            held.push(window.ring.iter().any(|&t| t == 330_000));
        }
        let expected: Vec<bool> = (0..length)
            .map(|k| (jump_at..jump_at + usize::from(period)).contains(&k))
            .collect();
        assert_eq!(held, expected);
    }
}

/// The same highs, lows and closes stamped a minute apart, a day apart, all at one instant and receding, and
/// once more with the open at the low, no volume and an open interest: every state and every reading is equal.
#[test]
fn a_time_gap_reads_as_contiguous_candles() {
    let mut draw = Draw(0x3C6E_F372_FE94_F82B);
    let bars = draw_series(&mut draw, 200);
    let shapes: [fn(i64, Hlc) -> Candle; 5] = [
        |k, (high, low, close)| Candle::new(k * MINUTE, close, high, low, close, 1, OI_NULL),
        |k, (high, low, close)| {
            Candle::new(k * 1_440 * MINUTE, close, high, low, close, 1, OI_NULL)
        },
        |_, (high, low, close)| Candle::new(0, close, high, low, close, 1, OI_NULL),
        |k, (high, low, close)| Candle::new(-k * MINUTE, close, high, low, close, 1, OI_NULL),
        |k, (high, low, close)| Candle::new(k * 7 * MINUTE, low, high, low, close, 0, 7),
    ];
    let runs: Vec<(Cci, Vec<Option<Reading>>)> = shapes
        .iter()
        .map(|shape| {
            let mut window = cci(14);
            let readings = (0..)
                .zip(&bars)
                .map(|(k, &bar)| {
                    window.fold(&shape(k, bar)).expect("an evaluable candle");
                    window.reading()
                })
                .collect();
            (window, readings)
        })
        .collect();
    let first = runs.first().expect("five runs");
    assert_eq!(first.1.iter().flatten().count(), 187);
    for run in &runs {
        assert_eq!(run, first);
    }
}

/// At periods 1, 5 and 20, every candle after a cut redrawn: every state up to the cut is unchanged, and the
/// states after it do change.
#[test]
fn no_reading_depends_on_a_later_candle() {
    let mut draw = Draw(0xA54F_F53A_5F1D_36F1);
    for period in [1, 5, 20] {
        let bars = draw_series(&mut draw, 120);
        let original = states(period, &bars);
        for cut in [0, 10, 37, 60, 100] {
            let mut redrawn = bars.get(..=cut).expect("a cut inside the series").to_vec();
            redrawn.extend(draw_series(&mut draw, 119 - cut));
            let again = states(period, &redrawn);
            assert_eq!(again.get(..=cut), original.get(..=cut));
            let readings = |states: &[Cci]| states.iter().map(Cci::reading).collect::<Vec<_>>();
            assert_eq!(
                readings(again.get(..=cut).expect("the states to the cut")),
                readings(original.get(..=cut).expect("the states to the cut"))
            );
            assert_ne!(again.get(cut + 1..), original.get(cut + 1..));
        }
    }
}

/// Each refusal `check_evaluable` gives, at an empty, a filling, a full and a long-running window of period
/// 10: refused by name, and the whole state, ring, sum, last `T` and period, unchanged.
#[test]
fn a_corrupt_candle_is_refused_and_changes_nothing() {
    let corrupt = [
        (
            Candle::new(0, 100, 90, 110, 100, 1, OI_NULL),
            Corrupt::HighBelowLow,
        ),
        (
            Candle::new(0, 0, i64::MAX, -1, 0, 1, OI_NULL),
            Corrupt::RangeOverflows,
        ),
        (
            Candle::new(0, 120, 110, 90, 100, 1, OI_NULL),
            Corrupt::PriceOutsideRange,
        ),
        (
            Candle::new(0, 100, 110, 90, 80, 1, OI_NULL),
            Corrupt::PriceOutsideRange,
        ),
        (
            Candle::new(0, 100, 110, 90, 100, -1, OI_NULL),
            Corrupt::NegativeVolume,
        ),
        (
            Candle::new(0, 0, 10, 0, 0, 1, OI_NULL),
            Corrupt::PriceNotPositive,
        ),
    ];
    let mut draw = Draw(0x510E_527F_ADE6_82D1);
    let bars = draw_series(&mut draw, 30);
    for prefix in [0, 3, 10, 30] {
        let mut window = folded(10, bars.get(..prefix).expect("a prefix of the series"));
        let before = window.clone();
        for (record, why) in corrupt {
            assert_eq!(window.fold(&record), Err(Refusal::Corrupt(why)));
            assert_eq!(window, before);
        }
    }
}

/// At periods 1, 20 and 1,000, 100,000 folds of the bench's walk: the ring never holds more than `period`
/// values, holds exactly `period` at the end, and its capacity never moves from what `new` reserved.
#[test]
fn the_ring_never_grows_after_new() {
    let mut walk = Walk::new(WALK_BASE);
    for period in [1, 20, 1_000] {
        let mut window = cci(period);
        let reserved = window.ring.capacity();
        assert!(reserved >= usize::from(period));
        for _ in 0..100_000 {
            window
                .fold(&walk.next_candle())
                .expect("the walk's candles fold");
            assert!(window.ring.len() <= usize::from(period));
            assert_eq!(window.ring.capacity(), reserved);
        }
        assert_eq!(window.ring.len(), usize::from(period));
    }
}

/// The state is a fixed set of fields whatever the period: `size_of::<Cci>()` at most 96 bytes (the ring's
/// values live on the heap, reserved once), a `Reading` at most 48 and a `Level` 4.
#[test]
fn the_state_is_a_fixed_size() {
    assert!(size_of::<Cci>() <= 96);
    assert!(size_of::<Reading>() <= 48);
    assert_eq!(size_of::<Level>(), 4);
}

/// A reading kept from the candle before is unchanged by the next fold and is the reading of the window
/// that excludes the current candle: the engine's "anchor excludes the candle" convention.
#[test]
fn the_previous_reading_judges_with_the_current_candle_excluded() {
    let mut draw = Draw(0x9B05_688C_2B3E_6C1F);
    let bars = draw_series(&mut draw, 100);
    let mut window = cci(14);
    let mut previous: Option<Reading> = None;
    for (end, (minute, &bar)) in (1..).zip((0..).zip(&bars)) {
        let kept = previous;
        window
            .fold(&candle(minute, bar))
            .expect("an evaluable candle");
        assert_eq!(kept, previous);
        let before = bars.get(..end - 1).expect("the candles before this one");
        assert_eq!(
            kept.map(|reading| (reading.deviation(), reading.total_deviation())),
            brute(14, before).map(|(_, deviation, total)| (deviation, total))
        );
        previous = window.reading();
    }
    assert!(previous.is_some());
}

/// The three measuring functions at tiny sizes return `rounds` rounds of `samples` timings, warm and cold;
/// period 0 is refused; a walk from 0 paisa refuses its first candle in the history and in a timed step; and
/// the walk's candles are evaluable and `2 * WALK_REACH` paisa wide.
#[test]
fn the_bench_measures_every_row_at_tiny_sizes() {
    let shape = |timings: &[Vec<u64>]| timings.iter().map(Vec::len).collect::<Vec<_>>();
    let warm = measure_step(30, 3, 2).expect("the walk's candles fold");
    assert_eq!(shape(&warm), [3, 3]);
    let cold = measure_step(0, 2, 4).expect("the walk's candles fold");
    assert_eq!(shape(&cold), [2, 2, 2, 2]);
    assert_eq!(measure_step(5, 7, 0), Ok(Vec::new()));
    for period in [1, 20, 200] {
        let alone = measure_period(period, 3, 2).expect("the walk's candles fold");
        assert_eq!(shape(&alone), [3, 3]);
        let read = measure_reading_period(period, 2, 3).expect("the walk's candles fold");
        assert_eq!(shape(&read), [2, 2, 2]);
    }
    assert_eq!(measure_period(0, 1, 1), Err(Refusal::PeriodZero));
    assert_eq!(measure_reading_period(0, 1, 1), Err(Refusal::PeriodZero));
    let refused = Err(Refusal::Corrupt(Corrupt::PriceNotPositive));
    assert_eq!(measure(20, 1, 1, 1, 0, read_and_judge), refused);
    assert_eq!(measure(20, 0, 1, 1, 0, fold_only), refused);
    let mut walk = Walk::new(WALK_BASE);
    for _ in 0..1_000 {
        let next = walk.next_candle();
        assert_eq!(next.check_evaluable(), Ok(()));
        assert_eq!(next.high - next.low, 2 * WALK_REACH);
    }
}

/// `summarize` reads ranks off sorted timings; `flat_within` passes p99s within the ceiling of the first and
/// inside the budget, at both edges, and fails one past the ceiling, one past the budget, a zero first p99, a
/// zero later p99 and nothing measured.
#[test]
fn a_cost_is_read_by_rank_and_judged_flat_or_not() {
    assert_eq!(summarize(&[]), StepCost::default());
    let one = StepCost {
        p50_ns: 7,
        p99_ns: 7,
        max_ns: 7,
        samples: 1,
    };
    assert_eq!(summarize(&[7]), one);
    let hundred: Vec<u64> = (1..=100).collect();
    let ranks = (summarize(&hundred).p50_ns, summarize(&hundred).p99_ns);
    assert_eq!(ranks, (50, 99));
    let thousand: Vec<u64> = (1..=1_000).collect();
    let read = summarize(&thousand);
    assert_eq!(
        (read.p50_ns, read.p99_ns, read.max_ns, read.samples),
        (500, 990, 1_000, 1_000)
    );
    let cost = |p99_ns: u64| StepCost {
        p50_ns: 1,
        p99_ns,
        max_ns: p99_ns,
        samples: 100,
    };
    assert!(flat_within(
        &[cost(100), cost(300), cost(250)],
        3_000,
        2_000
    ));
    assert!(flat_within(&[cost(2_000)], 3_000, 2_000));
    assert!(!flat_within(&[cost(100), cost(301)], 3_000, 2_000));
    assert!(!flat_within(&[cost(1_000), cost(2_001)], 3_000, 2_000));
    assert!(!flat_within(&[cost(0), cost(0)], 3_000, 2_000));
    assert!(!flat_within(&[cost(100), cost(0)], 3_000, 2_000));
    assert!(!flat_within(&[], 3_000, 2_000));
}
