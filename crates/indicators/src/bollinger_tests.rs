//! The tests of [`super`], PSRB-BOLL (D-8590 to D-8595): every test PARTS 4.5
//! names, fixtures BOLL-1 and BOLL-2 exactly, and a transcription of the
//! published definition written apart from the module. Every series is
//! invented; no vendor row.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a test \
              that cannot panic cannot fail. `expect` panics inside the standard \
              library and so leaves no region in this file that a green run cannot \
              enter."
)]

use super::*;

/// BOLL-1: the textbook population example, mean 500 and sigma 200 (n 8).
const BOLL_1: [i64; 8] = [200, 400, 400, 400, 500, 500, 700, 900];

/// BOLL-1 mirrored about its mean: mean 500, sigma 200, the last close 100.
const BOLL_1_MIRROR: [i64; 8] = [800, 600, 600, 600, 500, 500, 300, 100];

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// The widths the brute-force and parity tests judge at, in thousandths.
const WIDTHS: [u32; 6] = [0, 1, 1_000, 2_000, 2_500, 10_000];

/// A candle whose four prices are `close`, stamped `minute` minutes in.
fn candle(minute: i64, close: i64) -> Candle {
    Candle::new(minute * MINUTE, close, close, close, close, 1, OI_NULL)
}

/// A close inside the domain.
fn close(paisa: i64) -> Close {
    Close::new(paisa).expect("a close inside the domain")
}

/// A width inside the domain.
fn width(milli: u32) -> Width {
    Width::from_milli(milli).expect("a width inside the domain")
}

/// A window of `period` closes that has folded `closes`, in order.
fn folded(period: u16, closes: &[i64]) -> Bollinger {
    let mut bands = Bollinger::new(period).expect("a period inside the domain");
    for (minute, &paisa) in (0..).zip(closes) {
        bands
            .fold(&candle(minute, paisa))
            .expect("an evaluable candle");
    }
    bands
}

/// The window of exactly `closes`.
fn window_of(closes: &[i64]) -> Window {
    let period = u16::try_from(closes.len()).expect("a period that fits");
    folded(period, closes).window().expect("a full window")
}

/// BOLL-2: the twenty closes 100..=119, a steady trend.
fn boll_2() -> Vec<i64> {
    (100..=119).collect()
}

/// BOLL-2 falling: 119 down to 100.
fn boll_2_falling() -> Vec<i64> {
    (100..=119).rev().collect()
}

/// A deterministic xorshift, so every run draws the same values.
struct Draw(u64);

impl Draw {
    /// The next value of the generator.
    fn draw(&mut self) -> u64 {
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
}

/// `length` closes of a walk from 100,000 paisa: small moves, a jump of up to
/// 3,000 paisa about one candle in twenty and a repeated close about one in
/// five, kept inside `50_000..=150_000`.
fn draw_series(draw: &mut Draw, length: usize) -> Vec<i64> {
    let mut paisa = 100_000;
    let mut series = Vec::with_capacity(length);
    for _ in 0..length {
        let step = match draw.within(0, 19) {
            0 => draw.within(-3_000, 3_000),
            1..=4 => 0,
            _ => draw.within(-25, 25),
        };
        paisa = (paisa + step).clamp(50_000, 150_000);
        series.push(paisa);
    }
    series
}

/// The published definition (D-8590), transcribed apart from the module: the
/// basis is `S / n` and the population variance is
/// `sum((x - S / n)^2) / n`, which is `spread / n^3` with
/// `spread = sum((n * x - S)^2)` read off each close's own distance from the
/// mean, never off `n * Q - S^2`. Every test is decided by exact squares.
struct Published {
    n: i128,
    total: i128,
    squares: i128,
    spread: i128,
}

impl Published {
    /// The definition's quantities for the window `closes`.
    fn of(closes: &[i64]) -> Self {
        let n = i128::try_from(closes.len()).expect("a length that fits");
        let total: i128 = closes.iter().copied().map(i128::from).sum();
        let squares = closes.iter().map(|&x| i128::from(x) * i128::from(x)).sum();
        let spread = closes
            .iter()
            .map(|&x| (n * i128::from(x) - total) * (n * i128::from(x) - total))
            .sum();
        Self {
            n,
            total,
            squares,
            spread,
        }
    }

    /// `n * c - S`: `n` times the close's distance from the mean.
    fn gap(&self, paisa: i64) -> i128 {
        self.n * i128::from(paisa) - self.total
    }

    /// `(c - S / n)^2 > (z / 1000)^2 * spread / n^3`, multiplied out:
    /// `10^6 * n * gap^2 > z_milli^2 * spread`.
    fn outside(&self, paisa: i64, milli: u32) -> bool {
        let gap = self.gap(paisa);
        let z = i128::from(milli);
        1_000_000 * self.n * gap * gap > z * z * self.spread
    }

    /// The sample variance is `spread / (n^2 * (n - 1))`, so the sample test
    /// is `10^6 * (n - 1) * gap^2 > z_milli^2 * spread`.
    fn outside_sample(&self, paisa: i64, milli: u32) -> bool {
        let gap = self.gap(paisa);
        let z = i128::from(milli);
        1_000_000 * (self.n - 1) * gap * gap > z * z * self.spread
    }

    /// `c > S / n + z * sigma`.
    fn above(&self, paisa: i64, milli: u32) -> bool {
        self.gap(paisa) > 0 && self.outside(paisa, milli)
    }

    /// `c < S / n - z * sigma`.
    fn below(&self, paisa: i64, milli: u32) -> bool {
        self.gap(paisa) < 0 && self.outside(paisa, milli)
    }

    /// `c > S / n + z * s`, `s` the sample sigma.
    fn above_sample(&self, paisa: i64, milli: u32) -> bool {
        self.gap(paisa) > 0 && self.outside_sample(paisa, milli)
    }

    /// `c < S / n - z * s`.
    fn below_sample(&self, paisa: i64, milli: u32) -> bool {
        self.gap(paisa) < 0 && self.outside_sample(paisa, milli)
    }

    /// `1000 * (c - S / n) / sigma >= z`, for a window whose spread is not 0:
    /// the left side is `1000 * gap * sqrt(n / spread)`, compared by squares
    /// with the signs read first.
    fn z_at_least(&self, paisa: i64, z: i128) -> bool {
        let gap = self.gap(paisa);
        let left = 1_000_000 * self.n * gap * gap;
        let right = z * z * self.spread;
        match (gap >= 0, z > 0) {
            (true, false) => true,
            (true, true) => left >= right,
            (false, true) => false,
            (false, false) => z < 0 && left <= right,
        }
    }
}

/// Zero, one past the ceiling and `u16::MAX` are refused by name; 1 and 4,096
/// are accepted, each with a ring of exactly its own length.
#[test]
fn a_zero_period_and_one_past_the_ceiling_are_refused() {
    assert_eq!(Bollinger::new(0), Err(Refusal::PeriodZero));
    assert_eq!(
        Bollinger::new(4_097),
        Err(Refusal::PeriodAboveMax { period: 4_097 })
    );
    assert_eq!(
        Bollinger::new(u16::MAX),
        Err(Refusal::PeriodAboveMax { period: 65_535 })
    );
    for period in [1, MAX_PERIOD] {
        let bands = Bollinger::new(period).expect("a period inside the domain");
        assert_eq!(bands.period(), period);
        assert_eq!(bands.ring.len(), usize::from(period));
        assert!(!bands.warm());
        assert_eq!(bands.window(), None);
    }
}

/// `Close::new` accepts 1 and `MAX_CLOSE` and reads each back, and refuses 0,
/// -1, `i64::MIN`, `MAX_CLOSE + 1` and `i64::MAX` naming the value. Through
/// `fold`, `MAX_CLOSE` folds, one paisa more and `i64::MAX` are refused with
/// the state unchanged, and a close of 0 or below is the candle's own refusal,
/// which comes first.
#[test]
fn a_close_outside_its_range_is_refused_and_changes_nothing() {
    assert_eq!(Close::new(1).map(Close::paisa), Ok(1));
    assert_eq!(Close::new(MAX_CLOSE).map(Close::paisa), Ok(4_294_967_296));
    for paisa in [0, -1, i64::MIN, MAX_CLOSE + 1, i64::MAX] {
        assert_eq!(
            Close::new(paisa),
            Err(Refusal::CloseOutOfRange { close: paisa })
        );
    }
    let mut bands = folded(3, &[MAX_CLOSE, MAX_CLOSE]);
    let before = bands.clone();
    for paisa in [MAX_CLOSE + 1, i64::MAX] {
        assert_eq!(
            bands.fold(&candle(9, paisa)),
            Err(Refusal::CloseOutOfRange { close: paisa })
        );
        assert_eq!(bands, before);
    }
    for paisa in [0, -1] {
        assert_eq!(
            bands.fold(&candle(9, paisa)),
            Err(Refusal::Corrupt(Corrupt::PriceNotPositive))
        );
        assert_eq!(bands, before);
    }
    bands
        .fold(&candle(9, MAX_CLOSE))
        .expect("the largest close folds");
    let window = bands.window().expect("three closes fill the window");
    assert_eq!(window.last(), close(MAX_CLOSE));
    assert_eq!(window.sum(), 3 * i128::from(MAX_CLOSE));
}

/// 10,000 thousandths and 0 are accepted and read back; 10,001 and `u32::MAX`
/// are refused naming the value.
#[test]
fn a_width_past_its_ceiling_is_refused() {
    for milli in [0, 1, 2_000, MAX_WIDTH_MILLI] {
        assert_eq!(Width::from_milli(milli).map(Width::milli), Ok(milli));
    }
    for milli in [10_001, u32::MAX] {
        assert_eq!(
            Width::from_milli(milli),
            Err(Refusal::WidthAboveMax { milli })
        );
    }
}

/// At width 0 both bands are the basis: on BOLL-1 (`V` 2,560,000) 501 is
/// above, 500 is neither and 499 is below, and both band values are the basis
/// 500. On a mean that is not whole (BOLL-1 with its last close 901, basis
/// 500.125) the floor of the upper band is 500 and the ceiling of the lower
/// band 501, so 500 is below and 501 above. The sample readings agree at
/// width 0.
#[test]
fn a_zero_width_band_is_the_basis() {
    let zero = width(0);
    let whole = window_of(&BOLL_1);
    assert_eq!(whole.variance_numerator(), 2_560_000);
    assert!(whole.above_upper(close(501), zero));
    assert!(!whole.above_upper(close(500), zero));
    assert!(!whole.below_lower(close(500), zero));
    assert!(whole.below_lower(close(499), zero));
    assert_eq!(whole.upper_floor(zero), 500);
    assert_eq!(whole.basis_floor(), 500);
    assert_eq!(whole.lower_ceil(zero), 500);

    let uneven = window_of(&[200, 400, 400, 400, 500, 500, 700, 901]);
    assert_eq!(uneven.sum(), 4_001);
    assert_eq!(uneven.basis_floor(), 500);
    assert_eq!(uneven.upper_floor(zero), 500);
    assert_eq!(uneven.lower_ceil(zero), 501);
    assert!(uneven.below_lower(close(500), zero));
    assert!(!uneven.above_upper(close(500), zero));
    assert!(uneven.above_upper(close(501), zero));
    assert!(!uneven.below_lower(close(501), zero));

    for (window, paisa) in [
        (whole, 499),
        (whole, 500),
        (whole, 501),
        (uneven, 500),
        (uneven, 501),
    ] {
        assert_eq!(
            window.above_upper_sample(close(paisa), zero),
            Some(window.above_upper(close(paisa), zero))
        );
        assert_eq!(
            window.below_lower_sample(close(paisa), zero),
            Some(window.below_lower(close(paisa), zero))
        );
    }
}

/// Each kind of record `check_evaluable` refuses is refused by `fold` as that
/// same kind, and the whole state is unchanged after each.
#[test]
fn a_corrupt_candle_is_refused_and_changes_nothing() {
    let mut bands = folded(4, &[1_000, 1_010, 990, 1_005, 1_002]);
    let before = bands.clone();
    let good = candle(9, 1_000);
    for (bad, why) in [
        (
            Candle {
                high: 900,
                low: 1_100,
                ..good
            },
            Corrupt::HighBelowLow,
        ),
        (
            Candle {
                high: i64::MAX,
                low: i64::MIN,
                ..good
            },
            Corrupt::RangeOverflows,
        ),
        (
            Candle {
                open: 1_200,
                high: 1_100,
                low: 900,
                ..good
            },
            Corrupt::PriceOutsideRange,
        ),
        (
            Candle {
                close: 800,
                high: 1_100,
                low: 900,
                ..good
            },
            Corrupt::PriceOutsideRange,
        ),
        (Candle { volume: -1, ..good }, Corrupt::NegativeVolume),
        (
            Candle {
                open: 0,
                high: 0,
                low: 0,
                close: 0,
                ..good
            },
            Corrupt::PriceNotPositive,
        ),
    ] {
        assert_eq!(bad.check_evaluable(), Err(why));
        assert_eq!(bands.fold(&bad), Err(Refusal::Corrupt(why)));
        assert_eq!(bands, before);
    }
}

/// No window until `period` closes are held, and one at the `period`-th,
/// holding exactly those closes, at periods 1, 2, 3, 20 and 4,096; it stays
/// warm after.
#[test]
fn no_window_before_period_closes_and_one_at_the_period_th() {
    for period in [1_u16, 2, 3, 20, MAX_PERIOD] {
        let mut bands = Bollinger::new(period).expect("a period inside the domain");
        let mut total = 0_i128;
        for minute in 1..i64::from(period) {
            bands
                .fold(&candle(minute, 1_000 + minute))
                .expect("an evaluable candle");
            total += i128::from(1_000 + minute);
            assert!(!bands.warm());
            assert_eq!(bands.window(), None);
        }
        bands.fold(&candle(0, 5_000)).expect("an evaluable candle");
        assert!(bands.warm());
        let window = bands
            .window()
            .expect("the period-th close makes the window");
        assert_eq!(window.period(), period);
        assert_eq!(window.last(), close(5_000));
        assert_eq!(window.sum(), total + 5_000);
        bands.fold(&candle(1, 5_001)).expect("an evaluable candle");
        assert!(bands.warm());
    }
}

/// BOLL-1: `S` 4,000, `Q` 2,320,000, `V` 2,560,000, so the basis is 500 and
/// sigma `sqrt(V) / n` = 1,600 / 8 = 200 exactly; `D` for the close 900 is
/// 3,200, two sigmas, so its z-score is 2.000 and the close 100's is -2.000.
#[test]
fn the_basis_and_sigma_match_fixture_boll_1() {
    let window = window_of(&BOLL_1);
    assert_eq!(window.period(), 8);
    assert_eq!(window.last(), close(900));
    assert_eq!(window.sum(), 4_000);
    assert_eq!(window.sum_sq(), 2_320_000);
    assert_eq!(window.variance_numerator(), 2_560_000);
    assert_eq!(window.basis_floor(), 500);
    assert_eq!(isqrt_i128(window.variance_numerator()), 1_600);
    assert_eq!(square(1_600), window.variance_numerator());
    assert_eq!(window.reach(close(900)), 3_200);
    assert_eq!(window.z_milli(close(900)), Some(2_000));
    assert_eq!(window.z_milli(close(700)), Some(1_000));
    assert_eq!(window.z_milli(close(500)), Some(0));
    assert_eq!(window.z_milli(close(300)), Some(-1_000));
    assert_eq!(window.z_milli(close(100)), Some(-2_000));
}

/// BOLL-1 at z 1.999, 2.000 and 2.001: the close 900 is above the upper band
/// at 1.999 only, ON it at 2.000, and the band's floor is 899, 900 and 900,
/// the lower band's ceiling 101, 100 and 100. The lower side is the mirrored
/// fixture's close 100, the same three ways.
#[test]
fn a_close_exactly_on_the_band_is_not_beyond_it() {
    let rising = window_of(&BOLL_1);
    let falling = window_of(&BOLL_1_MIRROR);
    for (milli, beyond, upper, lower) in [
        (1_999, true, 899, 101),
        (2_000, false, 900, 100),
        (2_001, false, 900, 100),
    ] {
        let z = width(milli);
        assert_eq!(rising.above_upper(close(900), z), beyond);
        assert_eq!(rising.upper_floor(z), upper);
        assert_eq!(rising.lower_ceil(z), lower);
        assert_eq!(falling.below_lower(close(100), z), beyond);
        assert_eq!(falling.lower_ceil(z), lower);
        assert_eq!(falling.upper_floor(z), upper);
        assert!(!rising.below_lower(close(900), z));
        assert!(!falling.above_upper(close(100), z));
    }
}

/// BOLL-2: `S` 2,190, `Q` 240,470, `V` 13,300, `D` 190, so
/// `z^2 = D^2 / V = 57 / 21 = 3 * (n - 1) / (n + 1)`, z = 1.6475...: above at
/// z 1.647 (upper floor 118), not at 1.648 (119), z-score 1.647. Falling
/// (119 down to 100): below at 1.647 (lower ceiling 101), not at 1.648 (100),
/// and the z-score's floor is -1.648.
#[test]
fn a_steady_trend_sits_one_point_six_four_seven_sigma_above_its_mean() {
    let rising = window_of(&boll_2());
    let falling = window_of(&boll_2_falling());
    for window in [rising, falling] {
        assert_eq!(
            (window.sum(), window.sum_sq(), window.variance_numerator()),
            (2_190, 240_470, 13_300)
        );
    }
    assert_eq!(rising.reach(close(119)), 190);
    assert_eq!(falling.reach(close(100)), -190);
    assert_eq!(square(190) * 21, 13_300 * 57);

    assert!(rising.above_upper(close(119), width(1_647)));
    assert_eq!(rising.upper_floor(width(1_647)), 118);
    assert!(!rising.above_upper(close(119), width(1_648)));
    assert_eq!(rising.upper_floor(width(1_648)), 119);
    assert_eq!(rising.z_milli(close(119)), Some(1_647));

    assert!(falling.below_lower(close(100), width(1_647)));
    assert_eq!(falling.lower_ceil(width(1_647)), 101);
    assert!(!falling.below_lower(close(100), width(1_648)));
    assert_eq!(falling.lower_ceil(width(1_648)), 100);
    assert_eq!(falling.z_milli(close(100)), Some(-1_648));
}

/// Twenty equal closes: `V` 0, so no z-score; the close itself is neither
/// above nor below at any width, population or sample, and both band values
/// are the basis. With no spread, any other close is beyond its side's band.
#[test]
fn a_flat_window_has_zero_sigma_and_the_close_on_both_bands() {
    let window = window_of(&[2_500_000; 20]);
    assert_eq!(window.variance_numerator(), 0);
    assert_eq!(window.z_milli(close(2_500_000)), None);
    assert_eq!(window.z_milli(close(2_500_001)), None);
    assert_eq!(window.basis_floor(), 2_500_000);
    for milli in [0, 1, 2_000, MAX_WIDTH_MILLI] {
        let z = width(milli);
        assert!(!window.above_upper(close(2_500_000), z));
        assert!(!window.below_lower(close(2_500_000), z));
        assert_eq!(window.above_upper_sample(close(2_500_000), z), Some(false));
        assert_eq!(window.below_lower_sample(close(2_500_000), z), Some(false));
        assert_eq!(window.upper_floor(z), 2_500_000);
        assert_eq!(window.lower_ceil(z), 2_500_000);
        assert!(window.above_upper(close(2_500_001), z));
        assert!(window.below_lower(close(2_499_999), z));
    }
}

/// n = 1: every fold replaces the whole window, `V` is 0, there is no z-score
/// and no sample reading, and both band values are the close.
#[test]
fn a_window_of_one_close_has_zero_sigma_and_no_sample_sigma() {
    let mut bands = Bollinger::new(1).expect("a period inside the domain");
    for (minute, paisa) in (0..).zip([1_000, 1_250, 1_250, 990]) {
        bands
            .fold(&candle(minute, paisa))
            .expect("an evaluable candle");
        let window = bands.window().expect("one close is a full window");
        assert_eq!(
            (window.sum(), window.sum_sq()),
            (i128::from(paisa), square(i128::from(paisa)))
        );
        assert_eq!(window.variance_numerator(), 0);
        assert_eq!(window.z_milli(close(paisa)), None);
        for milli in [0, 2_000] {
            let z = width(milli);
            assert_eq!(window.above_upper_sample(close(paisa), z), None);
            assert_eq!(window.below_lower_sample(close(paisa), z), None);
            assert_eq!(window.upper_floor(z), paisa);
            assert_eq!(window.lower_ceil(z), paisa);
            assert!(!window.above_upper(close(paisa), z));
            assert!(!window.below_lower(close(paisa), z));
        }
    }
}

/// BOLL-1: the sample sigma is `200 * sqrt(8 / 7)` = 213.8..., so at z 2.000
/// the population band is 100 to 900 and the sample band 72.4... to
/// 927.6...: 901 and 927 are above the first only, 928 above both, and the
/// mirror below. The close 900 is above the sample band at z 1.870 and not at
/// 1.871 (`2 * sqrt(7 / 8)` = 1.8708...).
#[test]
fn sample_sigma_is_wider_by_the_root_of_n_over_n_minus_one() {
    let window = window_of(&BOLL_1);
    let two = width(2_000);
    for (paisa, population, sample) in [
        (900, false, false),
        (901, true, false),
        (927, true, false),
        (928, true, true),
    ] {
        assert_eq!(window.above_upper(close(paisa), two), population);
        assert_eq!(window.above_upper_sample(close(paisa), two), Some(sample));
    }
    for (paisa, population, sample) in [
        (100, false, false),
        (99, true, false),
        (73, true, false),
        (72, true, true),
    ] {
        assert_eq!(window.below_lower(close(paisa), two), population);
        assert_eq!(window.below_lower_sample(close(paisa), two), Some(sample));
    }
    assert_eq!(
        window.above_upper_sample(close(900), width(1_870)),
        Some(true)
    );
    assert_eq!(
        window.above_upper_sample(close(900), width(1_871)),
        Some(false)
    );
    let mirror = window_of(&BOLL_1_MIRROR);
    assert_eq!(
        mirror.below_lower_sample(close(100), width(1_870)),
        Some(true)
    );
    assert_eq!(
        mirror.below_lower_sample(close(100), width(1_871)),
        Some(false)
    );
}

/// `n` consecutive closes, rising and falling, at every n in 2..=60: `D` is
/// `n * (n - 1) / 2` and `V` is `n^2 * (n^2 - 1) / 12`, so
/// `z^2 = 3 * (n - 1) / (n + 1)` and the last close is beyond its band
/// exactly when `z_milli^2 * (n + 1) < 3 * (n - 1) * 10^6`. Two of them sit on
/// the band: n 2 at z 1.000 and n 7 at z 1.500.
#[test]
fn an_all_up_and_an_all_down_series() {
    for n in 2..=60_i64 {
        let up: Vec<i64> = (100..100 + n).collect();
        let down: Vec<i64> = up.iter().rev().copied().collect();
        let (rising, falling) = (window_of(&up), window_of(&down));
        let (top, bottom) = (close(99 + n), close(100));
        for milli in [0_u32, 500, 999, 1_000, 1_499, 1_500, 1_647, 1_648, 2_000] {
            let z = i128::from(milli);
            let beyond = z * z * i128::from(n + 1) < 3 * i128::from(n - 1) * 1_000_000;
            assert_eq!(rising.above_upper(top, width(milli)), beyond);
            assert_eq!(falling.below_lower(bottom, width(milli)), beyond);
        }
    }
    assert!(!window_of(&[100, 101]).above_upper(close(101), width(1_000)));
    assert!(window_of(&[100, 101]).above_upper(close(101), width(999)));
    let seven: Vec<i64> = (100..107).collect();
    assert!(!window_of(&seven).above_upper(close(106), width(1_500)));
    assert!(window_of(&seven).above_upper(close(106), width(1_499)));
}

/// A price gap enters the window and leaves it exactly `period` closes later:
/// the sum and the sum of squares equal a brute force over the last 5 closes
/// at every candle, and the 10% jump at candle 10 is gone at candle 15. A time
/// gap, and every field but the close, change nothing: stamps a minute apart,
/// a day apart or all equal, with other opens, highs, lows, volumes and open
/// interests, fold to equal states at every candle.
#[test]
fn a_gap_enters_the_window_and_leaves_it_after_period_closes() {
    let closes: Vec<i64> = (0..24)
        .map(|m| if m == 10 { 110_000 } else { 100_000 + m % 3 })
        .collect();
    let mut bands = Bollinger::new(5).expect("a period inside the domain");
    for (t, (minute, &paisa)) in (0_i64..).zip(&closes).enumerate() {
        bands
            .fold(&candle(minute, paisa))
            .expect("an evaluable candle");
        let seen = closes.get(..=t).expect("the closes so far");
        let last = seen
            .get(seen.len().saturating_sub(5)..)
            .expect("the last five");
        let brute = Published::of(last);
        assert_eq!((bands.sum, bands.sum_sq), (brute.total, brute.squares));
        assert_eq!(last.contains(&110_000), (10..15).contains(&t));
    }
    let tail = closes
        .get(closes.len() - 5..)
        .expect("the last five closes");
    assert_eq!(bands.window(), folded(5, tail).window());

    let mut by_minute = Bollinger::new(5).expect("a period inside the domain");
    let mut by_day = by_minute.clone();
    let mut by_nothing = by_minute.clone();
    for (minute, &paisa) in (0_i64..).zip(&closes) {
        by_minute
            .fold(&candle(minute, paisa))
            .expect("an evaluable candle");
        by_day
            .fold(&Candle {
                ts_micros: minute * 1_440 * MINUTE,
                ..candle(minute, paisa)
            })
            .expect("an evaluable candle");
        by_nothing
            .fold(&Candle {
                ts_micros: 0,
                open: paisa - 7,
                high: paisa + 50,
                low: paisa - 50,
                close: paisa,
                volume: 9_999,
                open_interest: 42,
            })
            .expect("an evaluable candle");
        assert_eq!(by_day, by_minute);
        assert_eq!(by_nothing, by_minute);
    }
}

/// On 2,000 random windows (n 1 to 40, closes 100,000 to 101,000, widths 0
/// to 10,000): for every close from three below the upper band's floor to
/// three above it, the squares test says above exactly when the close is
/// above the floor, and the lower mirror. Two exact readings of one fact.
#[test]
fn the_square_test_and_the_floor_band_agree_on_every_integer_close() {
    let mut draw = Draw(0x2545_F491_4F6C_DD1D);
    for _ in 0..2_000 {
        let n = draw.within(1, 40);
        let closes: Vec<i64> = (0..n).map(|_| draw.within(100_000, 101_000)).collect();
        let window = window_of(&closes);
        let z = width(u32::try_from(draw.within(0, 10_000)).expect("a width that fits"));
        let upper = window.upper_floor(z);
        for paisa in upper - 3..=upper + 3 {
            assert_eq!(window.above_upper(close(paisa), z), paisa > upper);
        }
        let lower = window.lower_ceil(z);
        for paisa in lower - 3..=lower + 3 {
            assert_eq!(window.below_lower(close(paisa), z), paisa < lower);
        }
    }
}

/// BRUTE FORCE: random walks with jumps and repeats, periods 1 to 64. At
/// every candle the window exists exactly when `period` closes have been
/// folded, its sums equal a direct recomputation over the last `period`
/// closes, and every comparison, population and sample, equals the published
/// definition over those closes, at seven closes and six widths. Each reading
/// at a candle comes from the closes up to it, so none depends on a later
/// candle.
#[test]
fn the_ring_equals_a_brute_force_window() {
    let mut draw = Draw(0x9E37_79B9_7F4A_7C15);
    for period in 1..=64_u16 {
        let size = usize::from(period);
        let series = draw_series(&mut draw, 3 * size + 7);
        let mut bands = Bollinger::new(period).expect("a period inside the domain");
        for (t, (minute, &paisa)) in (0_i64..).zip(&series).enumerate() {
            bands
                .fold(&candle(minute, paisa))
                .expect("an evaluable candle");
            assert_eq!(bands.window().is_some(), t + 1 >= size);
            let Some(window) = bands.window() else {
                continue;
            };
            let last = series
                .get(t + 1 - size..=t)
                .expect("the last period closes");
            let brute = Published::of(last);
            assert_eq!(
                (window.sum(), window.sum_sq()),
                (brute.total, brute.squares)
            );
            let low = last.iter().copied().min().expect("a close");
            let high = last.iter().copied().max().expect("a close");
            let mean = window.basis_floor();
            for probe in [low - 1, low, mean, mean + 1, paisa, high, high + 1] {
                for milli in WIDTHS {
                    let z = width(milli);
                    assert_eq!(
                        window.above_upper(close(probe), z),
                        brute.above(probe, milli)
                    );
                    assert_eq!(
                        window.below_lower(close(probe), z),
                        brute.below(probe, milli)
                    );
                    assert_eq!(
                        window.above_upper_sample(close(probe), z),
                        (period > 1).then(|| brute.above_sample(probe, milli))
                    );
                    assert_eq!(
                        window.below_lower_sample(close(probe), z),
                        (period > 1).then(|| brute.below_sample(probe, milli))
                    );
                }
            }
        }
    }
}

/// PARITY with the published definition (D-8590), in exact rationals, on the
/// two fixtures, their mirrors, a window of equal closes and 300 random
/// windows: the basis's floor brackets `S / n`; `n * V` is the definition's
/// `sum((n * x - S)^2)`, so `V / n^2` is its variance; every band test equals
/// the definition's; the upper floor is the last close not above the band and
/// the lower ceiling the last not below it; and the z-score is the floor of
/// `1000 * (c - S / n) / sigma`, with no value where sigma is 0.
#[test]
fn parity_with_the_published_definition() {
    let mut windows = vec![
        BOLL_1.to_vec(),
        BOLL_1_MIRROR.to_vec(),
        boll_2(),
        boll_2_falling(),
        vec![2_500_000; 20],
    ];
    let mut draw = Draw(0xD1B5_4A32_D192_ED03);
    for _ in 0..300 {
        let n = usize::try_from(draw.within(1, 50)).expect("a length that fits");
        windows.push(draw_series(&mut draw, n));
    }
    for closes in &windows {
        let window = window_of(closes);
        let published = Published::of(closes);
        let basis = i128::from(window.basis_floor());
        assert!(basis * published.n <= published.total);
        assert!(published.total < (basis + 1) * published.n);
        assert_eq!(window.variance_numerator() * published.n, published.spread);
        let low = closes.iter().copied().min().expect("a close");
        let high = closes.iter().copied().max().expect("a close");
        for probe in [low, high, window.basis_floor(), window.last().paisa()] {
            for milli in WIDTHS {
                let z = width(milli);
                assert_eq!(
                    window.above_upper(close(probe), z),
                    published.above(probe, milli)
                );
                assert_eq!(
                    window.below_lower(close(probe), z),
                    published.below(probe, milli)
                );
                let upper = window.upper_floor(z);
                assert!(!published.above(upper, milli));
                assert!(published.above(upper + 1, milli));
                let lower = window.lower_ceil(z);
                assert!(!published.below(lower, milli));
                assert!(published.below(lower - 1, milli));
            }
            match window.z_milli(close(probe)) {
                None => assert_eq!(published.spread, 0),
                Some(z) => {
                    let z = i128::from(z);
                    assert!(published.z_at_least(probe, z));
                    assert!(!published.z_at_least(probe, z + 1));
                }
            }
        }
    }
    // The four sign cases of the slow path, read once each by name on BOLL-1.
    let boll_1 = Published::of(&BOLL_1);
    assert!(boll_1.z_at_least(900, 0));
    assert!(boll_1.z_at_least(900, 2_000));
    assert!(!boll_1.z_at_least(100, 1));
    assert!(!boll_1.z_at_least(100, 0));
    assert!(boll_1.z_at_least(100, -2_000));
    assert!(!boll_1.z_at_least(100, -1_999));
}

/// The widest window of the widest spread at the widest width: 4,096 closes
/// alternating 1 and `MAX_CLOSE`, z 10.000. Sigma is `(2^32 - 1) / 2`, the
/// largest the domain allows, each extreme close sits exactly one sigma from
/// the mean, and every reading, both dispersions and both band values, is
/// exact. The compile-time bounds are restated here at those extremes.
#[test]
fn the_extremes_never_overflow() {
    let closes: Vec<i64> = (0..4_096)
        .map(|i| if i % 2 == 0 { 1 } else { MAX_CLOSE })
        .collect();
    let window = window_of(&closes);
    let span = i128::from(MAX_CLOSE - 1);
    assert_eq!(window.sum(), 2_048 * i128::from(MAX_CLOSE + 1));
    assert_eq!(window.variance_numerator(), square(2_048 * span));
    let (top, bottom, widest) = (close(MAX_CLOSE), close(1), width(MAX_WIDTH_MILLI));

    assert_eq!(window.z_milli(top), Some(1_000));
    assert_eq!(window.z_milli(bottom), Some(-1_000));
    assert!(window.above_upper(top, width(999)));
    assert!(!window.above_upper(top, width(1_000)));
    assert!(!window.above_upper(top, widest));
    assert!(window.below_lower(bottom, width(999)));
    assert!(!window.below_lower(bottom, widest));
    assert_eq!(window.above_upper_sample(top, width(999)), Some(true));
    assert_eq!(window.above_upper_sample(top, width(1_000)), Some(false));
    assert_eq!(window.below_lower_sample(bottom, width(999)), Some(true));
    assert_eq!(window.below_lower_sample(bottom, widest), Some(false));
    assert_eq!(window.basis_floor(), 2_147_483_648);
    assert_eq!(window.upper_floor(widest), 23_622_320_123);
    assert_eq!(window.lower_ceil(widest), -19_327_352_826);
    assert!(window.upper_floor(widest) < BAND_EDGE);
    assert!(window.lower_ceil(widest) > -BAND_EDGE);

    let n = i128::from(MAX_PERIOD);
    let reach = window.reach(top);
    let variance = window.variance_numerator();
    assert!(reach.abs() <= 1 << 44);
    assert!(variance <= 1 << 86);
    let scaled = reach.checked_mul(WIDTH_DEN).expect("1000 * D fits");
    let left = scaled
        .checked_mul(scaled)
        .and_then(|squared| squared.checked_mul(n - 1))
        .expect("(1000 * D)^2 * (n - 1) fits");
    assert!(left <= 1 << 120);
    let right = square(i128::from(MAX_WIDTH_MILLI))
        .checked_mul(n)
        .and_then(|product| product.checked_mul(variance))
        .expect("z_milli^2 * n * V fits");
    assert!(right <= 1 << 125);
    let numerator = square(reach)
        .checked_mul(WIDTH_DEN * WIDTH_DEN)
        .expect("10^6 * D^2 fits");
    assert!(numerator <= 1 << 108);
}

/// WHITE-BOX: a ring whose write position is `period`, one past its end, is
/// refused as `RingOutOfStep` with the whole state unchanged. No call of the
/// API makes that state: through 50 folds the position stays inside the ring.
#[test]
fn a_ring_out_of_step_is_refused_by_name_and_changes_nothing() {
    let mut walking = Bollinger::new(4).expect("a period inside the domain");
    for minute in 0..50 {
        walking
            .fold(&candle(minute, 1_000 + minute % 7))
            .expect("an evaluable candle");
        assert!(walking.next < walking.ring.len());
    }
    let mut bands = folded(4, &[1_000, 1_001, 1_002]);
    bands.next = usize::from(bands.period);
    let before = bands.clone();
    assert_eq!(bands.fold(&candle(9, 1_003)), Err(Refusal::RingOutOfStep));
    assert_eq!(bands, before);
}

/// A caller that wants the window to exclude the candle keeps the window of
/// the candle before (a `Copy`) and judges the next close against it: that
/// copy equals the published definition over the 20 closes before the candle,
/// while the current window holds the candle itself.
#[test]
fn the_previous_window_reads_the_current_candle_excluded() {
    let mut draw = Draw(0x6A09_E667_F3BC_C908);
    let series = draw_series(&mut draw, 240);
    let mut bands = Bollinger::new(20).expect("a period inside the domain");
    let mut previous: Option<Window> = None;
    for (t, (minute, &paisa)) in (0_i64..).zip(&series).enumerate() {
        bands
            .fold(&candle(minute, paisa))
            .expect("an evaluable candle");
        if let Some(kept) = previous {
            let before = Published::of(series.get(t - 20..t).expect("the 20 closes before"));
            assert_eq!(kept.sum(), before.total);
            for milli in [1_000, 2_000] {
                let z = width(milli);
                assert_eq!(
                    kept.above_upper(close(paisa), z),
                    before.above(paisa, milli)
                );
                assert_eq!(
                    kept.below_lower(close(paisa), z),
                    before.below(paisa, milli)
                );
            }
            let now = bands.window().expect("a full window");
            assert_eq!(now.last(), close(paisa));
            assert_eq!(
                now.sum() - kept.sum(),
                i128::from(paisa) - before_oldest(&series, t)
            );
        }
        previous = bands.window();
    }
}

/// The close that left the window when candle `t` entered it.
fn before_oldest(series: &[i64], t: usize) -> i128 {
    i128::from(
        *series
            .get(t - 20)
            .expect("the oldest close of the window before"),
    )
}

/// The ring is allocated once: its length and capacity are the same after
/// 100,000 folds as after `new`, at 20 closes and at 4,096.
#[test]
fn the_ring_never_grows_after_new() {
    for period in [20_u16, MAX_PERIOD] {
        let mut bands = Bollinger::new(period).expect("a period inside the domain");
        let (length, capacity) = (bands.ring.len(), bands.ring.capacity());
        assert_eq!(length, usize::from(period));
        let mut walk = Walk::new(WALK_BASE);
        for _ in 0..100_000 {
            bands
                .fold(&walk.next_candle())
                .expect("an evaluable candle");
        }
        assert_eq!(
            (bands.ring.len(), bands.ring.capacity()),
            (length, capacity)
        );
    }
}

/// The bench's measuring functions at tiny sizes: the shapes they return, the
/// refusals of a period `new` refuses, a walk whose candles refuse before and
/// inside the timed rounds, no rounds, and a window not yet full, which is
/// timed too. The walk stays within 4 paisa a minute of its start.
#[test]
fn the_bench_measures_both_rows_at_tiny_sizes() {
    let step = measure_step(25, 4, 3).expect("the walk folds");
    assert_eq!(step.len(), 3);
    assert!(step.iter().all(|round| round.len() == 4));
    let period = measure_period(7, 5, 2).expect("the walk folds");
    assert_eq!(period.len(), 2);
    assert!(period.iter().all(|round| round.len() == 5));
    assert_eq!(measure_period(0, 1, 1), Err(Refusal::PeriodZero));
    assert_eq!(
        measure_period(4_097, 1, 1),
        Err(Refusal::PeriodAboveMax { period: 4_097 })
    );
    for history in [0, 1] {
        assert!(matches!(
            measure(20, history, 1, 1, MAX_CLOSE + 100),
            Err(Refusal::CloseOutOfRange { .. })
        ));
    }
    assert_eq!(measure(20, 0, 3, 0, WALK_BASE), Ok(Vec::new()));
    let unwarmed = measure(20, 0, 3, 2, WALK_BASE).expect("the walk folds");
    assert_eq!(unwarmed.len(), 2);
    let mut walk = Walk::new(WALK_BASE);
    for minute in 1..=1_000_i64 {
        let candle = walk.next_candle();
        assert!((WALK_BASE - 4 * minute..=WALK_BASE + 4 * minute).contains(&candle.close));
        assert_eq!(candle.ts_micros, minute * MINUTE);
        assert_eq!(candle.check_evaluable(), Ok(()));
    }
}

/// `summarize` reads nearest ranks off a sorted round (none, one, 100 and 200
/// samples), and `flat_within` refuses no costs, a zero first p99, a rise
/// past the ceiling and a p99 over the budget, and passes a rise of exactly
/// the ceiling and a p99 of exactly the budget.
#[test]
fn a_cost_is_read_by_rank_and_judged_flat_or_not() {
    assert_eq!(summarize(&[]), StepCost::default());
    assert_eq!(
        summarize(&[5]),
        StepCost {
            p50_ns: 5,
            p99_ns: 5,
            max_ns: 5,
            samples: 1
        }
    );
    let hundred: Vec<u64> = (1..=100).collect();
    assert_eq!(
        summarize(&hundred),
        StepCost {
            p50_ns: 50,
            p99_ns: 99,
            max_ns: 100,
            samples: 100
        }
    );
    let two_hundred: Vec<u64> = (1..=200).collect();
    assert_eq!(
        summarize(&two_hundred),
        StepCost {
            p50_ns: 100,
            p99_ns: 198,
            max_ns: 200,
            samples: 200
        }
    );
    let cost = |p99_ns| StepCost {
        p50_ns: 1,
        p99_ns,
        max_ns: p99_ns,
        samples: 100,
    };
    assert!(!flat_within(&[], 3_000, 2_000));
    assert!(!flat_within(&[cost(0), cost(0)], 3_000, 2_000));
    assert!(flat_within(
        &[cost(100), cost(300), cost(250)],
        3_000,
        2_000
    ));
    assert!(!flat_within(&[cost(100), cost(301)], 3_000, 2_000));
    assert!(flat_within(&[cost(1_000), cost(2_000)], 3_000, 2_000));
    assert!(!flat_within(&[cost(1_000), cost(2_001)], 3_000, 2_000));
    assert!(!flat_within(&[cost(2_001)], 3_000, 2_000));
}
