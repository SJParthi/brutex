#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::{calendar, day};
use crate::market::DayKind;
use crate::rng::SplitMix64;

fn m(y: i32, mo: u32) -> Month {
    Month::of(y, mo)
}

fn window() -> Window {
    Window { first: m(2018, 9), last: m(2026, 10) }
}

#[test]
fn t_f04_common_names_and_late_starters() {
    let cal = calendar(&[]);
    let z: Vec<Month> = (0..106).map(|i| m(2018, 1).plus(i)).collect();
    let g: Vec<Month> = (0..98).map(|i| m(2018, 9).plus(i)).collect();
    assert_eq!(trading_span(&z, &g, &window(), &cal), Ok(Some((day(2018, 9, 3), day(2026, 10, 30)))));
    let irfc_g: Vec<Month> = (0..70).map(|i| m(2021, 1).plus(i)).collect();
    assert_eq!(trading_span(&z, &irfc_g, &window(), &cal).unwrap().unwrap().0, day(2021, 1, 1));
    // 9 cases: Zerodha's first month before / equal / after GDFL's, the later one before /
    // at / after 2018-09.
    for (zf, gf, want) in [
        (m(2018, 1), m(2018, 3), day(2018, 9, 3)),
        (m(2018, 3), m(2018, 3), day(2018, 9, 3)),
        (m(2018, 5), m(2018, 3), day(2018, 9, 3)),
        (m(2018, 1), m(2018, 9), day(2018, 9, 3)),
        (m(2018, 9), m(2018, 9), day(2018, 9, 3)),
        (m(2018, 10), m(2018, 9), day(2018, 10, 1)),
        (m(2018, 1), m(2019, 2), day(2019, 2, 1)),
        (m(2019, 2), m(2019, 2), day(2019, 2, 1)),
        (m(2019, 3), m(2019, 2), day(2019, 3, 1)),
    ] {
        assert_eq!(trading_span(&[zf], &[gf], &window(), &cal).unwrap().unwrap().0, want, "{zf:?} {gf:?}");
    }
}

/// X-F04: spans at their edges.
#[test]
fn x_f04_trading_span_extremes() {
    let cal = calendar(&[]);
    let w = window();
    assert_eq!(trading_span(&[m(2018, 1)], &[], &w, &cal), Ok(None));
    assert_eq!(trading_span(&[], &[m(2018, 9)], &w, &cal), Ok(None));
    assert_eq!(trading_span(&[m(2018, 1)], &[m(2026, 11)], &w, &cal), Ok(None));
    assert_eq!(trading_span(&[m(2026, 10)], &[m(2026, 10)], &w, &cal), Ok(Some((day(2026, 10, 1), day(2026, 10, 30)))));
    // combo: an internal gap in both feeds changes nothing; the end stays the window's.
    assert_eq!(trading_span(&[m(2018, 9), m(2024, 2)], &[m(2018, 9), m(2024, 2)], &w, &cal).unwrap().unwrap().1, day(2026, 10, 30));
    // oob: unsorted or repeated month lists refuse by name.
    assert_eq!(trading_span(&[m(2018, 10), m(2018, 9)], &[m(2018, 9)], &w, &cal), Err(Refusal::Unsorted(m(2018, 9))));
    assert_eq!(trading_span(&[m(2018, 9)], &[m(2018, 9), m(2018, 9)], &w, &cal), Err(Refusal::Duplicate(m(2018, 9))));
    let closed: Vec<(Day, DayKind)> = m(2018, 9).days().into_iter().map(|d| (d, DayKind::Closed)).collect();
    let short = Window { first: m(2018, 9), last: m(2018, 9) };
    assert_eq!(trading_span(&[m(2018, 9)], &[m(2018, 9)], &short, &calendar(&closed)), Ok(None));
    assert_eq!(window_days(&cal, &Window { first: m(2300, 1), last: m(2300, 1) }), None);
    assert_eq!(trading_span(&[m(2300, 1)], &[m(2300, 1)], &Window { first: m(2018, 9), last: m(2300, 1) }, &cal), Ok(None));
    // the late start month has no full session left inside the window
    let last_month_closed: Vec<(Day, DayKind)> = m(2018, 10).days().into_iter().map(|d| (d, DayKind::Closed)).collect();
    let two = Window { first: m(2018, 9), last: m(2018, 10) };
    assert_eq!(trading_span(&[m(2018, 10)], &[m(2018, 10)], &two, &calendar(&last_month_closed)), Ok(None));
}

/// X-F05: the warm-up start at its edges.
#[test]
fn x_f05_warmup_start_extremes() {
    let run: Vec<Month> = (0..9).map(|i| m(2018, 1).plus(i)).collect();
    assert_eq!(warmup_start(&run, m(2018, 9)), Ok(m(2018, 1)));
    assert_eq!(warmup_start(&[m(2018, 9)], m(2018, 9)), Ok(m(2018, 9)));
    assert_eq!(warmup_start(&[m(2018, 1), m(2018, 2), m(2018, 3), m(2018, 4), m(2018, 6), m(2018, 7), m(2018, 8), m(2018, 9)], m(2018, 9)), Ok(m(2018, 6)));
    assert_eq!(warmup_start(&[m(2018, 1), m(2018, 9)], m(2018, 9)), Ok(m(2018, 9)));
    assert_eq!(warmup_start(&[m(2018, 1), m(2018, 3), m(2021, 1)], m(2021, 1)), Ok(m(2021, 1)));
    assert_eq!(warmup_start(&[m(2018, 1)], m(2018, 9)), Err(Refusal::StartMonthAbsent(m(2018, 9))));
    // combo: four runs separated by gaps, the last holds the start.
    assert_eq!(warmup_start(&[m(2018, 1), m(2018, 3), m(2018, 5), m(2018, 7), m(2018, 8), m(2018, 9)], m(2018, 9)), Ok(m(2018, 7)));
    // combo: a single-month run with a later month after it.
    assert_eq!(warmup_start(&[m(2018, 9), m(2018, 10)], m(2018, 9)), Ok(m(2018, 9)));
    // oob: two generations of one month.
    assert_eq!(warmup_start(&[m(2018, 9), m(2018, 9)], m(2018, 9)), Err(Refusal::Duplicate(m(2018, 9))));
}

/// P-F05-1: generated month sets against a walk-back model.
#[test]
fn p_f05_warmup_vs_walk_back_model() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let months: Vec<Month> = (0..24).filter(|_| r.chance(2, 3)).map(|i| m(2017, 1).plus(i)).collect();
            let start = m(2017, 1).plus(i32::try_from(r.below(24)).unwrap());
            let got = warmup_start(&months, start);
            if months.contains(&start) {
                let mut first = start;
                while months.contains(&first.prev()) {
                    first = first.prev();
                }
                assert_eq!(got, Ok(first));
            } else {
                assert_eq!(got, Err(Refusal::StartMonthAbsent(start)));
            }
        }
    }
}
