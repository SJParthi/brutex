#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::atr::Atr;
use super::supertrend::SuperTrend;
use super::*;

fn hlc(h: i64, l: i64, c: i64) -> Bar {
    Bar::new(c, h, l, c, 1)
}

/// One fixture row: the expected two-band reading (trend up?, line, upper, lower), `None`
/// before the ATR exists.
type Want = Option<(bool, i128, i128, i128)>;

fn check(period: u16, factor: u32, candles: &[(i64, i64, i64)], want: &[Want]) {
    let mut st = SuperTrend::new(period, factor);
    for (i, ((h, l, c), w)) in candles.iter().zip(want).enumerate() {
        st.fold(&hlc(*h, *l, *c));
        let got = st
            .reading()
            .map(|r| (r.trend == Trend::Up, r.line, r.upper, r.lower));
        assert_eq!(got, *w, "candle {i}");
        if let Some(r) = st.reading() {
            assert_eq!(
                r.settled,
                w.is_some() && i > first_reading(want),
                "settled at {i}"
            );
        }
    }
}

fn first_reading(want: &[Want]) -> usize {
    want.iter().position(Option::is_some).unwrap()
}

const D: bool = false;
const U: bool = true;

/// PARTS 5.4 ST-1 (period 2, factor 1.000): the two-band form starts down.
#[test]
fn st1_the_seed_starts_down() {
    check(
        2,
        1_000,
        &[
            (110, 90, 100),
            (112, 100, 110),
            (115, 105, 114),
            (116, 108, 115),
            (116, 109, 111),
            (112, 96, 98),
            (100, 90, 95),
        ],
        &[
            None,
            Some((D, 122, 122, 90)),
            Some((D, 122, 122, 97)),
            Some((D, 122, 122, 102)),
            Some((D, 120, 120, 104)),
            Some((D, 116, 116, 104)),
            Some((D, 106, 106, 84)),
        ],
    );
}

/// ST-2 (period 2, factor 1.500): a tighter line at a flip down.
#[test]
fn st2_keeps_the_ratcheted_upper_band_at_a_flip_down() {
    check(
        2,
        1_500,
        &[
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
            (126, 104, 108),
        ],
        &[
            None,
            Some((D, 138, 138, 84)),
            Some((D, 135, 135, 93)),
            Some((D, 126, 126, 96)),
            Some((D, 126, 126, 96)),
            Some((D, 126, 126, 96)),
            Some((U, 101, 126, 101)),
            Some((U, 110, 154, 110)),
            Some((U, 110, 151, 110)),
            Some((U, 110, 138, 110)),
            Some((D, 138, 138, 110)),
        ],
    );
}

/// ST-3 (period 2, factor 1.000): a tighter line at a flip up, then an earlier flip down.
#[test]
fn st3_turns_down_one_candle_before_the_latch() {
    check(
        2,
        1_000,
        &[
            (143, 138, 142),
            (142, 126, 126),
            (128, 119, 124),
            (144, 123, 140),
            (142, 119, 124),
            (137, 123, 137),
            (141, 119, 119),
            (123, 114, 116),
        ],
        &[
            None,
            Some((D, 144, 144, 124)),
            Some((D, 132, 132, 124)),
            Some((U, 124, 132, 124)),
            Some((U, 124, 149, 124)),
            Some((U, 124, 146, 124)),
            Some((D, 146, 146, 124)),
            Some((D, 132, 132, 104)),
        ],
    );
}

/// ST-4 (period 3, factor 3.000): a flat series reads down.
#[test]
fn st4_a_flat_series_reads_down() {
    let flat = (100, 100, 100);
    check(
        3,
        3_000,
        &[flat; 5],
        &[
            None,
            None,
            Some((D, 100, 100, 100)),
            Some((D, 100, 100, 100)),
            Some((D, 100, 100, 100)),
        ],
    );
}

/// ST-5 (period 2, factor 0.500): a wide candle's basic lower band above its close.
#[test]
fn st5_a_wide_candle_turns_down_at_once() {
    check(
        2,
        500,
        &[
            (121, 101, 118),
            (118, 102, 105),
            (106, 101, 103),
            (107, 101, 106),
            (120, 104, 118),
            (138, 109, 111),
            (113, 104, 104),
        ],
        &[
            None,
            Some((D, 119, 119, 101)),
            Some((D, 108, 108, 101)),
            Some((D, 108, 108, 101)),
            Some((U, 106, 108, 106)),
            Some((D, 133, 133, 113)),
            Some((D, 115, 115, 101)),
        ],
    );
}

/// ST-6 (period 1, factor 1.000): the first candle's upper band is the previous 0.
#[test]
fn st6_period_one_keeps_the_zero_upper_band() {
    check(
        1,
        1_000,
        &[(110, 90, 100), (106, 100, 105), (109, 104, 108)],
        &[
            Some((D, 0, 0, 80)),
            Some((D, 109, 109, 97)),
            Some((D, 109, 109, 101)),
        ],
    );
}

/// ST-7 and ST-8 (period 2, factor 1.000): gaps through the bands.
#[test]
fn st7_and_st8_gaps_cross_the_bands() {
    let up = [
        (101, 99, 100),
        (101, 99, 100),
        (102, 100, 101),
        (121, 118, 120),
        (122, 119, 121),
    ];
    let up_want = [
        None,
        Some((D, 102, 102, 98)),
        Some((D, 102, 102, 99)),
        Some((U, 108, 102, 108)),
        Some((U, 113, 127, 113)),
    ];
    check(2, 1_000, &up, &up_want);
    let mut down = up.to_vec();
    down.push((96, 93, 94));
    let mut down_want = up_want.to_vec();
    down_want.push(Some((D, 111, 111, 113)));
    check(2, 1_000, &down, &down_want);
}

/// ST-9 (period 2, factor 0.500): equal bands while up keep the published meaning.
#[test]
fn st9_equal_bands_follow_the_previous_trend() {
    check(
        2,
        500,
        &[
            (119, 108, 108),
            (119, 105, 119),
            (129, 116, 128),
            (139, 128, 136),
            (156, 130, 151),
            (151, 138, 138),
            (142, 137, 141),
            (171, 124, 155),
        ],
        &[
            None,
            Some((D, 118, 118, 106)),
            Some((D, 128, 128, 116)),
            Some((U, 128, 128, 128)),
            Some((U, 134, 152, 134)),
            Some((U, 137, 151, 137)),
            Some((U, 137, 144, 137)),
            Some((U, 137, 144, 137)),
        ],
    );
}

#[test]
fn the_atr_seeds_on_the_mean_then_smooths_floored_and_reads_whole_paisa() {
    let mut a = Atr::new(2);
    assert_eq!(a.value(), None);
    a.fold(&hlc(110, 90, 100));
    assert_eq!(a.value(), None);
    assert_eq!(a.scaled(), 20 * SCALE);
    a.fold(&hlc(112, 100, 110));
    assert_eq!(a.value(), Some(16));
    a.fold(&hlc(115, 105, 114));
    assert_eq!(a.value(), Some(13));
    a.fold(&hlc(116, 108, 115));
    assert_eq!(a.scaled(), 10 * SCALE + SCALE / 2);
    assert_eq!(a.value(), Some(10));
    assert_eq!(Atr::true_range(&hlc(10, 8, 9), None), 2);
    assert_eq!(Atr::true_range(&hlc(10, 8, 9), Some(14)), 6);
    assert_eq!(Atr::true_range(&hlc(10, 8, 9), Some(4)), 6);
    let zero = Atr::new(0);
    assert_eq!(zero, Atr::new(1));
}

#[test]
fn the_supertrend_clamps_its_factor_and_reads_its_atr() {
    assert_eq!(SuperTrend::new(10, 0), SuperTrend::new(10, 1));
    assert_eq!(SuperTrend::new(10, 200_000), SuperTrend::new(10, 100_000));
    let mut st = SuperTrend::new(1, 1_000);
    assert_eq!(st.atr(), None);
    st.fold(&hlc(110, 90, 100));
    assert_eq!(st.atr(), Some(20));
}

#[test]
fn the_candle_check_names_the_first_failing_rule() {
    let ok = Bar::new(100, 110, 90, 105, 0);
    assert_eq!(check_evaluable(&ok), Ok(()));
    let cases = [
        (Bar::new(100, 110, 0, 105, 1), Corrupt::NonPositive),
        (Bar::new(100, 0, 90, 105, 1), Corrupt::NonPositive),
        (Bar::new(0, 110, 90, 105, 1), Corrupt::NonPositive),
        (Bar::new(100, 110, 90, 0, 1), Corrupt::NonPositive),
        (Bar::new(100, 90, 110, 100, 1), Corrupt::Inverted),
        (Bar::new(80, 110, 90, 105, 1), Corrupt::OutOfRange),
        (Bar::new(120, 110, 90, 105, 1), Corrupt::OutOfRange),
        (Bar::new(100, 110, 90, 80, 1), Corrupt::OutOfRange),
        (Bar::new(100, 110, 90, 120, 1), Corrupt::OutOfRange),
        (Bar::new(100, 110, 90, 105, -1), Corrupt::NegativeVolume),
    ];
    for (bar, want) in cases {
        assert_eq!(check_evaluable(&bar), Err(want), "{bar:?}");
    }
}

use super::bollinger::Bollinger;
use super::cci::Cci;
use super::ema::Ema;
use super::rsi::Rsi;
use super::willr::WilliamsR;

fn close(c: i64) -> Bar {
    Bar::new(c, c, c, c, 1)
}

/// PARTS 3.5 fixture RSI-1 (period 3): every state exact.
#[test]
fn rsi1_every_reading_is_exact() {
    let mut rsi = Rsi::new(3);
    let want: [Option<(i128, i128, i128)>; 8] = [
        None,
        None,
        None,
        Some((2_000_000, 666_666, 750_000)),
        Some((1_333_333, 444_444, 750_000)),
        Some((888_888, 2_296_296, 279_069)),
        Some((1_592_592, 1_530_864, 509_881)),
        Some((3_061_728, 1_020_576, 750_000)),
    ];
    for (c, w) in [100, 103, 101, 104, 104, 98, 101, 107]
        .into_iter()
        .zip(want)
    {
        rsi.fold(&close(c));
        let got = rsi.reading().map(|r| (r.gain, r.loss, r.ppm()));
        assert_eq!(got, w, "close {c}");
    }
    let r = rsi.reading().unwrap();
    assert!(r.above(749_999) && !r.above(750_000) && !r.below(750_000) && r.below(750_001));
    assert!(r.faithful());
    assert_eq!(Rsi::new(0), Rsi::new(1));
}

#[test]
fn rsi_flat_reads_one_hundred_and_is_never_faithful_or_below() {
    let mut rsi = Rsi::new(2);
    for _ in 0..4 {
        rsi.fold(&close(100));
    }
    let r = rsi.reading().unwrap();
    assert!(r.flat() && !r.faithful());
    assert_eq!(r.ppm(), 1_000_000);
    assert!(r.above(999_999) && !r.above(1_000_000) && !r.below(1));
    let mut up = Rsi::new(2);
    for c in [100, 101, 102] {
        up.fold(&close(c));
    }
    let r = up.reading().unwrap();
    assert!(!r.flat() && r.loss == 0 && r.ppm() == 1_000_000 && r.above(999_999));
    let mut down = Rsi::new(2);
    for c in [102, 101, 100] {
        down.fold(&close(c));
    }
    let r = down.reading().unwrap();
    assert_eq!(r.ppm(), 0);
    assert!(r.below(1) && !r.above(0));
}

/// PARTS 4.5 fixtures BOLL-1 (n 8, mean 500, sigma 200) and BOLL-2 (n 20, the steady trend).
#[test]
fn boll1_and_boll2_decide_exactly_at_their_bands() {
    let mut b = Bollinger::new(8);
    for c in [200, 400, 400, 400, 500, 500, 700, 900] {
        assert!(b.window().is_none());
        b.fold(&close(c));
    }
    let w = b.window().unwrap();
    assert_eq!(
        (w.sum, w.sum_sq, w.variance_numerator(), w.deviation()),
        (4_000, 2_320_000, 2_560_000, 3_200)
    );
    assert!(!w.above_upper(2_000) && w.above_upper(1_999) && !w.above_upper(2_001));
    assert!(!w.below_lower(1_000));
    let mut t = Bollinger::new(20);
    for c in 100..=119 {
        t.fold(&close(c));
    }
    let w = t.window().unwrap();
    assert_eq!(
        (w.sum, w.sum_sq, w.variance_numerator(), w.deviation()),
        (2_190, 240_470, 13_300, 190)
    );
    assert!(w.above_upper(1_647) && !w.above_upper(1_648));
    let mut m = Bollinger::new(20);
    for c in (100..=119).rev() {
        m.fold(&close(c));
    }
    let w = m.window().unwrap();
    assert!(w.below_lower(1_647) && !w.below_lower(1_648) && !w.above_upper(0));
    let mut flat = Bollinger::new(3);
    for _ in 0..5 {
        flat.fold(&close(7));
    }
    let w = flat.window().unwrap();
    assert!(!w.above_upper(0) && !w.below_lower(0) && w.variance_numerator() == 0);
    assert_eq!(Bollinger::new(0), Bollinger::new(1));
}

/// PWC 5.5 fixtures WR-1 and WR-2 (period 3).
#[test]
fn wr1_and_wr2_compare_exactly_on_both_levels() {
    let mut wr = WilliamsR::new(3);
    let rows = [
        (110, 100, 105),
        (115, 104, 114),
        (113, 101, 102),
        (120, 108, 119),
        (118, 112, 112),
        (110, 100, 103),
        (121, 105, 120),
    ];
    let want = [
        None,
        None,
        Some((false, true)),
        Some((true, false)),
        Some((true, true)),
        Some((false, true)),
        Some((true, false)),
    ];
    for ((h, l, c), w) in rows.into_iter().zip(want) {
        wr.fold(&Bar::new(c, h, l, c, 1));
        let got = wr.reading().map(|r| (r.above(-850_000), r.below(-150_000)));
        assert_eq!(got, w, "candle {h} {l} {c}");
    }
    let mut flat = WilliamsR::new(3);
    for (h, l, c) in [(500, 500, 500), (500, 500, 500), (500, 500, 500)] {
        flat.fold(&Bar::new(c, h, l, c, 1));
    }
    let r = flat.reading().unwrap();
    assert!(
        r.flat() && !r.above(-850_000) && !r.below(-150_000) && !r.above(-1_000_000) && !r.below(0)
    );
    flat.fold(&Bar::new(501, 501, 500, 501, 1));
    let r = flat.reading().unwrap();
    assert!(!r.flat() && r.above(-850_000) && !r.below(-150_000));
    flat.fold(&Bar::new(499, 501, 499, 499, 1));
    let r = flat.reading().unwrap();
    assert!(r.below(-150_000) && r.below(-850_000) && !r.above(-1_000_000));
    assert_eq!(WilliamsR::new(0), WilliamsR::new(1));
}

/// PWC 6.5 fixtures CCI-1 (period 4), CCI-2 and its mirror (the +-200 ties), CCI-3 (the ceiling).
#[test]
fn cci_fixtures_decide_exactly_at_their_levels() {
    let mut cci = Cci::new(4);
    let rows = [
        (110, 100, 105),
        (112, 104, 110),
        (108, 98, 100),
        (120, 106, 118),
        (125, 115, 124),
        (126, 120, 121),
        (118, 100, 101),
    ];
    let want = [
        None,
        None,
        None,
        Some((85, 196)),
        Some((116, 304)),
        Some((87, 324)),
        Some((-118, 272)),
    ];
    for ((h, l, c), w) in rows.into_iter().zip(want) {
        cci.fold(&Bar::new(c, h, l, c, 1));
        let got = cci.reading().map(|r| (r.deviation, r.total_deviation));
        assert_eq!(got, w, "candle {h} {l} {c}");
    }
    let r = cci.reading().unwrap();
    assert!(r.below(-100_000) && !r.above(100_000));
    let fold7 = |rows: &[(i64, i64, i64)]| {
        let mut c = Cci::new(7);
        for (h, l, cl) in rows {
            c.fold(&Bar::new(*cl, *h, *l, *cl, 1));
        }
        c.reading().unwrap()
    };
    let base = [(101, 99, 100); 5];
    let tie = fold7(&[base.as_slice(), &[(105, 103, 104), (113, 111, 113)]].concat());
    assert!(!tie.above(200_000) && !tie.below(200_000) && tie.above(199_999) && tie.below(200_001));
    let mirror = fold7(&[base.as_slice(), &[(97, 95, 96), (89, 87, 87)]].concat());
    assert!(!mirror.below(-200_000) && mirror.below(-199_999) && mirror.above(-200_001));
    let ceiling = fold7(&[[(101, 99, 100); 6].as_slice(), &[(101, 99, 101)]].concat());
    assert_eq!((ceiling.deviation, ceiling.total_deviation), (6, 12));
    assert!(ceiling.above(200_000));
    let flat = fold7(&[
        (101, 99, 100),
        (102, 98, 100),
        (101, 99, 100),
        (101, 99, 100),
        (101, 99, 100),
        (101, 99, 100),
        (101, 99, 100),
    ]);
    assert!(flat.flat() && !flat.above(-10_000_000) && !flat.below(10_000_000));
    assert_eq!(Cci::new(0), Cci::new(1));
}

/// AD 572-578: 199 closes of 101 then one of 100: the warm EMA 200 is above the close.
#[test]
fn the_ema_seeds_on_the_simple_mean_and_smooths_floored() {
    let mut ema = Ema::new(200);
    for _ in 0..199 {
        ema.fold(&close(101));
        assert!(!ema.warm());
    }
    ema.fold(&close(100));
    assert!(ema.warm());
    assert_eq!(ema.scaled(), 100_995_000);
    ema.fold(&close(100));
    assert_eq!(
        ema.scaled(),
        100_995_000 + ((100 * SCALE - 100_995_000) * 2).div_euclid(201)
    );
    let mut one = Ema::new(3);
    one.fold(&close(10));
    assert_eq!(one.scaled(), 10 * SCALE);
    one.fold(&close(11));
    assert_eq!(one.scaled(), 10 * SCALE + SCALE / 2);
    assert_eq!(Ema::new(0), Ema::new(1));
}

/// P-F15-1 and P-F18-1: RSI and EMA streams against from-scratch recomputations over the
/// whole history at every candle, on the four seeds.
#[test]
fn p_f15_f18_rsi_and_ema_vs_from_scratch() {
    for seed in crate::generator::SEEDS {
        let mut r = crate::rng::SplitMix64::new(seed);
        for _ in 0..20 {
            let period = u16::try_from(r.below(20) + 1).unwrap();
            let n = i128::from(period);
            let mut rsi = Rsi::new(period);
            let mut ema = Ema::new(period);
            let mut closes: Vec<i64> = Vec::new();
            for _ in 0..125 {
                let c = 1_000 + i64::try_from(r.below(60)).unwrap();
                closes.push(c);
                rsi.fold(&close(c));
                ema.fold(&close(c));
                // RSI from scratch: seed on the first `period` changes, then Wilder.
                let changes: Vec<i128> =
                    closes.windows(2).map(|w| i128::from(w[1] - w[0])).collect();
                let want = (changes.len() >= usize::from(period)).then(|| {
                    let (seed_part, rest) = changes.split_at(usize::from(period));
                    let mut g = seed_part
                        .iter()
                        .map(|d| d.max(&0) * SCALE)
                        .sum::<i128>()
                        .div_euclid(n);
                    let mut l = seed_part
                        .iter()
                        .map(|d| (-d).max(0) * SCALE)
                        .sum::<i128>()
                        .div_euclid(n);
                    for d in rest {
                        g += (d.max(&0) * SCALE - g).div_euclid(n);
                        l += ((-d).max(0) * SCALE - l).div_euclid(n);
                    }
                    (g, l)
                });
                assert_eq!(rsi.reading().map(|x| (x.gain, x.loss)), want);
                // EMA from scratch: the running mean, then the recursion.
                let k = closes.len().min(usize::from(period));
                let mut e = closes[..k]
                    .iter()
                    .map(|c| i128::from(*c) * SCALE)
                    .sum::<i128>()
                    .div_euclid(i128::try_from(k).unwrap());
                for c in &closes[k..] {
                    e += ((i128::from(*c) * SCALE - e) * 2).div_euclid(n + 1);
                }
                assert_eq!(ema.scaled(), e);
                assert_eq!(ema.warm(), closes.len() >= usize::from(period));
            }
        }
    }
}
