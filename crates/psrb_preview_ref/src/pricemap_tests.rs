#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::market::Bar;
use crate::rng::SplitMix64;

fn minute(i: u32, close: i64, volume: i64) -> Minute {
    Minute {
        start: OPEN + 60 * i,
        bar: Bar::new(close, close, close, close, volume),
    }
}

fn second(at: u32, close: i64) -> Second {
    Second {
        at,
        bar: Bar::flat(close, 1),
    }
}

#[test]
fn t_f31_the_same_minute_or_the_latest_earlier_one_with_both() {
    let minutes = [
        minute(0, 1_000, 5),
        minute(1, 1_010, 5),
        minute(2, 1_020, 5),
    ];
    let seconds = [
        second(OPEN + 5, 5_000),
        second(OPEN + 59, 5_002),
        second(OPEN + 61, 5_050),
    ];
    // T 09:18: the 09:17 minute has no GDFL second, nor 09:16's after... 09:16 has 09:16:01
    assert_eq!(
        ratio_source(&minutes, &seconds, OPEN + 180),
        Some(Ratio {
            z: 1_010,
            g: 5_050,
            minute: OPEN + 60
        })
    );
    assert_eq!(
        ratio_source(&minutes, &seconds, OPEN + 60),
        Some(Ratio {
            z: 1_000,
            g: 5_002,
            minute: OPEN
        })
    );
    assert_eq!(ratio_source(&minutes, &seconds, OPEN + 59), None);
}

/// X-F31: the ratio source at its edges.
#[test]
fn x_f31_ratio_source_extremes() {
    let minutes = [
        minute(0, 1_000, 5),
        minute(1, 1_010, 0),
        minute(2, 1_020, 5),
    ];
    // a padded minute is never Z: T 09:17 skips 09:16 and reads 09:15
    let seconds = [second(OPEN + 1, 999), second(OPEN + 61, 1_009)];
    assert_eq!(
        ratio_source(&minutes, &seconds, OPEN + 120).unwrap().minute,
        OPEN
    );
    // a second stamped at the minute's end belongs to the next minute
    let at_end = [second(OPEN + 60, 1_011)];
    assert_eq!(ratio_source(&minutes, &at_end, OPEN + 60), None);
    // no minute with both: no ratio
    assert_eq!(ratio_source(&minutes, &[], OPEN + 600), None);
    // the first record of the day: no underflow
    assert_eq!(
        ratio_source(&minutes, &[second(OPEN, 998)], OPEN + 60),
        Some(Ratio {
            z: 1_000,
            g: 998,
            minute: OPEN
        })
    );
    // combo: the first minute of the day, T 09:16, GDFL's first trade 09:15:01
    assert_eq!(
        ratio_source(&minutes, &seconds, OPEN + 60),
        Some(Ratio {
            z: 1_000,
            g: 999,
            minute: OPEN
        })
    );
    // combo: a 1:5 split day, Z adjusted and G raw
    assert_eq!(
        ratio_source(
            &[minute(0, 1_000, 5)],
            &[second(OPEN + 30, 5_000)],
            OPEN + 60
        )
        .unwrap()
        .g,
        5_000
    );
    // oob: every GDFL trade of the session inside one minute maps every later candle
    let one = [second(OPEN + 10, 990), second(OPEN + 20, 991)];
    assert_eq!(ratio_source(&minutes, &one, OPEN + 10_000).unwrap().g, 991);
    // a pre-open minute is never Z
    let pre = [Minute {
        start: OPEN - 60,
        bar: Bar::flat(7, 5),
    }];
    assert_eq!(ratio_source(&pre, &[second(OPEN - 30, 7)], OPEN + 60), None);
}

/// X-F-thirty-two (`map_level`): the mapped level at its edges.
#[test]
fn x_map_level_extremes_f_thirty_two() {
    let r = |z: i64, g: i64| Ratio { z, g, minute: OPEN };
    assert_eq!(map_level(1_001, &r(1_000, 5_000), Side::Long), Ok(5_005));
    assert_eq!(map_level(1_001, &r(3, 1), Side::Long), Ok(333));
    assert_eq!(map_level(1_001, &r(3, 1), Side::Short), Ok(334));
    // an exact quotient is not raised by the ceiling
    assert_eq!(map_level(999, &r(3, 1), Side::Short), Ok(333));
    assert_eq!(map_level(77, &r(9, 9), Side::Long), Ok(77));
    assert_eq!(map_level(1, &r(1, 1), Side::Short), Ok(1));
    // combo: a huge ratio at a large level stays exact in i128
    assert_eq!(
        map_level(i128::from(i64::MAX / 1_000), &r(1, 1_000), Side::Long),
        Ok(i64::MAX / 1_000 * 1_000)
    );
    // oob: refusals by name
    assert_eq!(
        map_level(5, &r(0, 1), Side::Long),
        Err(Refusal::ZNotPositive)
    );
    assert_eq!(
        map_level(i128::from(i64::MAX), &r(1, 2), Side::Long),
        Err(Refusal::BeyondI64)
    );
    assert_eq!(
        map_level(-5, &r(1, 1), Side::Long),
        Err(Refusal::NotPositive)
    );
    assert_eq!(
        map_level(0, &r(1, 1), Side::Short),
        Err(Refusal::NotPositive)
    );
    assert_eq!(unmapped_level(5), Ok(5));
    assert_eq!(unmapped_level(0), Err(Refusal::NotPositive));
    assert_eq!(unmapped_level(i128::MAX), Err(Refusal::BeyondI64));
}

/// P-F31-1 and the `map_level` property: generated sessions against a brute-force search over every minute,
/// and mapped levels against exact rational floor and ceiling.
#[test]
fn p_f31_ratio_and_map_level_vs_brute_force() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let mut minutes = Vec::new();
            let mut seconds = Vec::new();
            for i in 0..12u32 {
                if r.chance(4, 5) {
                    minutes.push(minute(
                        i,
                        1_000 + i64::from(i),
                        if r.chance(1, 5) { 0 } else { 3 },
                    ));
                }
                for s in 0..3u32 {
                    if r.chance(1, 3) {
                        seconds.push(second(
                            OPEN + 60 * i + 20 * s + u32::try_from(r.below(20)).unwrap(),
                            5_000 + i64::from(i * 10 + s),
                        ));
                    }
                }
            }
            let t = OPEN + 60 * u32::try_from(r.below(13)).unwrap();
            let brute = minutes
                .iter()
                .rev()
                .filter(|m| m.bar.volume > 0 && m.start + 60 <= t)
                .find_map(|m| {
                    seconds
                        .iter()
                        .rfind(|s| s.at >= m.start && s.at < m.start + 60)
                        .map(|s| (m.bar.close, s.bar.close))
                });
            let got = ratio_source(&minutes, &seconds, t);
            assert_eq!(got.map(|x| (x.z, x.g)), brute);
            let level = i128::from(r.below(1_000_000)) + 1;
            let z = i64::try_from(r.below(10_000)).unwrap() + 1;
            let g = i64::try_from(r.below(10_000)).unwrap() + 1;
            let ratio = Ratio { z, g, minute: OPEN };
            let p = level * i128::from(g);
            let floor = (p - p.rem_euclid(i128::from(z))) / i128::from(z);
            let ceil = if p % i128::from(z) == 0 {
                floor
            } else {
                floor + 1
            };
            assert_eq!(
                map_level(level, &ratio, Side::Long).ok().map(i128::from),
                Some(floor).filter(|v| *v > 0)
            );
            assert_eq!(
                map_level(level, &ratio, Side::Short).ok().map(i128::from),
                Some(ceil).filter(|v| *v > 0)
            );
        }
    }
}
